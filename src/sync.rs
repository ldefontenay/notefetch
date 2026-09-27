//! Deciding what to fetch, and fetching it. SPEC.md "Sync".
//!
//! A sync lists every recording on Plaud, brings the database into line with
//! the disk, then fetches what is incomplete: new recordings since the
//! first-sync cutoff, earlier failures, and recordings still missing a
//! transcript or summary within the retry window. It never fetches a
//! recording the user deleted or skipped.
//!
//! Progress and results come out as [`Event`]s. The interface turns them into
//! Log lines and announcements; the `--sync` developer flag prints them.

use std::sync::{Arc, Mutex};

use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use crate::cancel::Cancel;
use crate::clock::{self, Stamp};
use crate::config::Config;
use crate::error::{Error, Result};
use crate::library::{self, Library, Metadata};
use crate::mcp::Client;
use crate::naming;
use crate::plaud::{self, Detail};
use crate::store::{Record, State, Store};

/// At most two recordings are fetched at once (SPEC.md).
const AT_ONCE: usize = 2;

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// A line for the Log.
    Log(String),
    /// How far a sync has got.
    Progress { done: usize, total: usize },
    /// A recording changed, so the list should update its row.
    Changed(String),
}

/// The first-sync choice. SPEC.md "Sync".
#[derive(Debug, Clone, Copy)]
pub enum FirstSync {
    Everything,
    LastDays(u32),
    FromNow,
}

pub struct Engine {
    pub http: reqwest::Client,
    pub client: Client,
    pub store: Mutex<Store>,
    pub library: Library,
    pub config: Config,
}

#[derive(Debug, Default, PartialEq)]
pub struct Report {
    pub new_on_plaud: usize,
    pub fetched: usize,
    pub failed: usize,
    /// True when the first-sync choice has not been made, so nothing was fetched.
    pub needs_first_sync_choice: bool,
}

type Emit = Arc<dyn Fn(Event) + Send + Sync>;

impl Engine {
    fn store(&self) -> std::sync::MutexGuard<'_, Store> {
        // A panic while holding the lock would leave it poisoned; the data is
        // still consistent (every write is one statement), so carry on.
        self.store.lock().unwrap_or_else(|p| p.into_inner())
    }

    pub fn set_first_sync(&self, choice: FirstSync) -> Result<()> {
        let now = clock::now_utc().unix_millis();
        let from = match choice {
            FirstSync::Everything => i64::MIN,
            FirstSync::LastDays(d) => now - d as i64 * 86_400_000,
            FirstSync::FromNow => now,
        };
        self.store().set_sync_from(from)
    }

    /// List Plaud's recordings and reconcile with the disk, fetching nothing.
    /// Returns how many recordings Plaud has that NoteFetch had not seen.
    pub async fn refresh(&self, emit: &Emit) -> Result<usize> {
        let listed = plaud::list_all(&self.client).await?;
        let mut new = 0;
        {
            let store = self.store();
            for l in &listed {
                if store.upsert_listed(l)? {
                    new += 1;
                }
            }
        }
        self.reconcile(emit)?;
        emit(Event::Log(match (listed.len(), new) {
            (n, 0) => format!("Plaud has {n} recordings. None are new."),
            (n, 1) => format!("Plaud has {n} recordings, 1 of them new."),
            (n, k) => format!("Plaud has {n} recordings, {k} of them new."),
        }));
        Ok(new)
    }

    /// Bring the database into line with the output folder.
    pub fn reconcile(&self, emit: &Emit) -> Result<()> {
        match self.library.scan()? {
            Some(scan) => {
                for line in self.library.reconcile(&self.store(), &scan)? {
                    emit(Event::Log(line));
                }
            }
            // Before the first download there is no output folder, and that is
            // fine. Only say something if recordings should be there.
            None => {
                let expected = self.store().all()?.iter().any(|r| r.state == State::Downloaded);
                if expected {
                    emit(Event::Log(format!(
                        "The output folder \"{}\" is not there, so NoteFetch has not checked what is downloaded. If it is on a drive that is not plugged in, plug it in and refresh.",
                        self.library.root.display()
                    )));
                }
            }
        }
        Ok(())
    }

    /// Which recordings a sync would fetch.
    fn work(&self) -> Result<Option<Vec<Record>>> {
        let store = self.store();
        let Some(from) = store.sync_from()? else { return Ok(None) };
        let now = clock::now_utc().unix_millis();
        let window = self.config.retry_days as i64 * 86_400_000;
        let wants_text = self.config.download_transcript || self.config.download_summary;
        Ok(Some(
            store
                .all()?
                .into_iter()
                .filter(|r| match r.state {
                    State::New | State::Failed => r.start_ms >= from,
                    State::Downloaded => {
                        let missing_text = (self.config.download_transcript && !r.has_transcript)
                            || (self.config.download_summary && !r.has_summary);
                        let missing_audio = self.config.download_audio && !r.has_audio;
                        missing_audio || (wants_text && missing_text && now - r.start_ms < window)
                    }
                    State::Skipped | State::Deleted => false,
                })
                .collect(),
        ))
    }

    /// Sync new recordings. SPEC.md "Sync".
    pub async fn sync(self: &Arc<Self>, cancel: &Cancel, emit: Emit) -> Result<Report> {
        let mut report = Report { new_on_plaud: self.refresh(&emit).await?, ..Report::default() };
        let Some(work) = self.work()? else {
            report.needs_first_sync_choice = true;
            return Ok(report);
        };
        if work.is_empty() {
            emit(Event::Log("Everything is up to date.".into()));
            return Ok(report);
        }
        let ids: Vec<String> = work.into_iter().map(|r| r.plaud_id).collect();
        let (fetched, failed) = self.fetch_many(ids, cancel, emit.clone()).await?;
        report.fetched = fetched;
        report.failed = failed;
        emit(Event::Log(match (fetched, failed) {
            (0, 0) => "Sync finished. Everything is up to date.".to_string(),
            (f, 0) => format!("Sync finished. {} downloaded.", plural(f, "recording")),
            (f, n) => format!("Sync finished. {} downloaded, {} failed. The Log says why for each.", plural(f, "recording"), n),
        }));
        Ok(report)
    }

    /// Fetch the given recordings whatever their state: Download selected and
    /// Download again. Returns (recordings with something new, failures).
    pub async fn fetch_many(self: &Arc<Self>, ids: Vec<String>, cancel: &Cancel, emit: Emit) -> Result<(usize, usize)> {
        let total = ids.len();
        let gate = Arc::new(Semaphore::new(AT_ONCE));
        let mut tasks = JoinSet::new();
        for id in ids {
            let (engine, gate, cancel, emit) = (self.clone(), gate.clone(), cancel.clone(), emit.clone());
            tasks.spawn(async move {
                let _permit = gate.acquire_owned().await;
                if cancel.is_cancelled() {
                    return Err(Error::Cancelled);
                }
                let result = engine.fetch_one(&id, &cancel).await;
                engine.after_fetch(&id, &result, &emit);
                result
            });
        }
        let (mut fetched, mut failed, mut done, mut cancelled) = (0, 0, 0, false);
        emit(Event::Progress { done: 0, total });
        while let Some(joined) = tasks.join_next().await {
            match joined {
                Ok(Ok(new)) => {
                    if !new.is_empty() {
                        fetched += 1;
                    }
                    done += 1;
                }
                Ok(Err(Error::Cancelled)) => cancelled = true,
                Ok(Err(Error::SignInExpired)) => {
                    cancel.cancel();
                    tasks.abort_all();
                    return Err(Error::SignInExpired);
                }
                _ => {
                    failed += 1;
                    done += 1;
                }
            }
            emit(Event::Progress { done, total });
        }
        if cancelled {
            return Err(Error::Cancelled);
        }
        Ok((fetched, failed))
    }

    fn after_fetch(&self, id: &str, result: &Result<Vec<String>>, emit: &Emit) {
        let store = self.store();
        let Ok(Some(r)) = store.get(id) else { return };
        let name = library::display_name(&r);
        match result {
            Ok(new) if new.is_empty() => {
                let _ = store.set_error(id, None);
                emit(Event::Log(format!("Checked \"{name}\" again: {}", still_missing(&r))));
            }
            Ok(new) => {
                let _ = store.set_error(id, None);
                emit(Event::Log(format!("Downloaded \"{name}\": {}.", join_words(new))));
            }
            Err(Error::Cancelled) | Err(Error::SignInExpired) => {}
            Err(e) => {
                if r.state != State::Downloaded {
                    let _ = store.set_state(id, State::Failed);
                }
                let _ = store.set_error(id, Some(&e.to_string()));
                emit(Event::Log(format!("Could not download \"{name}\". {e}")));
            }
        }
        emit(Event::Changed(id.to_string()));
    }

    /// Fetch one recording: every part Plaud has and the settings ask for,
    /// skipping files already there. Returns the parts newly written, in words.
    pub async fn fetch_one(&self, id: &str, cancel: &Cancel) -> Result<Vec<String>> {
        let detail = plaud::detail(&self.client, id).await?;
        cancel.check()?;
        let location = self.place(&detail)?;
        let mut meta = self.library.read_metadata(&location).unwrap_or_else(|| Metadata {
            plaud_id: id.to_string(),
            plaud_name: detail.listed.name.clone(),
            local_name: naming::name_of_base(library::base_of(&location)).to_string(),
            start: detail.listed.start.iso_utc(),
            duration_ms: detail.listed.duration_ms,
            parts: Default::default(),
        });
        meta.plaud_name = detail.listed.name.clone();
        let now = clock::now_utc().iso_utc();
        let mut new = Vec::new();

        let texts = self.texts(&detail);
        for t in &texts {
            let part = naming::Part { label: &t.label, ext: &t.ext };
            let existed = self.library.file_path(&location, part).exists();
            if self.library.write_text(&location, part, &t.text)? && !existed {
                meta.parts.insert(t.key.clone(), now.clone());
                new.push(t.label.to_lowercase());
            }
        }
        self.library.write_metadata(&location, &meta)?;

        if self.config.download_audio {
            let dest = self.library.file_path(&location, naming::AUDIO);
            if !dest.exists() {
                let link = detail
                    .audio_link
                    .as_deref()
                    .ok_or_else(|| Error::Plaud("it has no audio for this recording yet.".into()))?;
                crate::download::audio(&self.http, &self.client, id, link, &dest, cancel).await?;
                meta.parts.insert("audio".into(), now.clone());
                self.library.write_metadata(&location, &meta)?;
                new.insert(0, "audio".into());
            }
        }
        self.library.set_dates(&location, &detail.listed.start);

        let store = self.store();
        let has_audio = self.library.file_path(&location, naming::AUDIO).exists();
        let has_transcript = self.library.file_path(&location, naming::TRANSCRIPT).exists();
        store.set_state(id, State::Downloaded)?;
        store.set_location(id, Some(&location))?;
        store.set_parts(id, has_audio, has_transcript, meta.has_summary())?;
        store.set_checked(id, clock::now_utc().unix_millis())?;
        Ok(new)
    }

    /// Where a recording goes: its existing folder if it has one on disk,
    /// otherwise a new one in the output folder.
    fn place(&self, detail: &Detail) -> Result<String> {
        let store = self.store();
        let record = match store.get(&detail.listed.id)? {
            Some(r) => r,
            None => {
                store.upsert_listed(&detail.listed)?;
                store.get(&detail.listed.id)?.ok_or_else(|| Error::Database("a recording just added was not found.".into()))?
            }
        };
        if let Some(loc) = &record.location
            && self.library.path_of(loc).join(naming::METADATA).is_file()
        {
            return Ok(loc.clone());
        }
        let name = record.local_name.clone().unwrap_or_else(|| naming::name_from_plaud(&record.plaud_name));
        let start = Stamp::from_unix_millis(record.start_ms);
        let location = self.library.create_recording_folder(&record.plaud_id, &name, &start)?;
        store.set_location(&record.plaud_id, Some(&location))?;
        Ok(location)
    }

    /// The text parts to write.
    fn texts(&self, d: &Detail) -> Vec<Text> {
        let mut out = Vec::new();
        let mut add = |key: String, part: naming::Part, text: String| {
            out.push(Text { key, label: part.label.to_string(), ext: part.ext.to_string(), text });
        };
        if self.config.download_transcript
            && let Some(t) = &d.transcript
        {
            add("transcript".into(), naming::TRANSCRIPT, plaud::transcript_text(t));
        }
        if self.config.download_summary {
            // Notes are Markdown. Laurent asked for a setting to save them as
            // .txt for anyone whose programs do not open .md files.
            let ext = if self.config.notes_as_txt { ".txt" } else { ".md" };
            let mut seen = Vec::new();
            for n in &d.notes {
                let label = naming::note_label(&n.tab, &mut seen);
                add(format!("note: {}", n.tab), naming::Part { label: &label, ext }, n.markdown.clone());
            }
        }
        if self.config.download_other {
            if let Some(t) = &d.cleaned_transcript {
                add("cleaned transcript".into(), naming::CLEANED_TRANSCRIPT, plaud::transcript_text(t));
            }
            if let Some(o) = &d.outline {
                add("outline".into(), naming::OUTLINE, plaud::outline_text(o));
            }
            if let Some(h) = &d.highlights {
                add("highlights".into(), naming::HIGHLIGHTS, plaud::highlights_text(h));
            }
        }
        out
    }
}

/// A text part: its name in metadata.json, its file label and extension, and
/// its content.
struct Text {
    key: String,
    label: String,
    ext: String,
    text: String,
}

/// "audio, transcript and summary".
fn join_words(parts: &[String]) -> String {
    match parts {
        [] => "nothing new".into(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

/// Why a recording checked again gave nothing new.
fn still_missing(r: &Record) -> String {
    match (r.has_transcript, r.has_summary) {
        (false, false) => "Plaud has no transcript or summary for it yet.".into(),
        (false, true) => "Plaud has no transcript for it yet.".into(),
        (true, false) => "Plaud has no summary for it yet.".into(),
        (true, true) => "nothing new.".into(),
    }
}

fn plural(n: usize, unit: &str) -> String {
    if n == 1 { format!("1 {unit}") } else { format!("{n} {unit}s") }
}
