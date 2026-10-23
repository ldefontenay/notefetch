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

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, MutexGuard};

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
    pub guard: Arc<Guard>,
}

/// Keeps the user's own changes apart from fetching, so a sync can run while
/// the user renames, moves or deletes other recordings (Laurent, milestone 4).
/// A change holds the disk lock while it runs. A fetch claims its recording
/// under the same lock, so a change never starts on a recording being
/// fetched, and a fetch never starts in the middle of a change.
#[derive(Default)]
pub struct Guard {
    disk: Mutex<()>,
    fetching: Mutex<HashSet<String>>,
    /// Recordings are moving to a new output folder, so no change may start.
    pub moving: std::sync::atomic::AtomicBool,
}

impl Guard {
    /// Held by each change, and by reconciling, which reads the whole disk.
    pub fn lock(&self) -> MutexGuard<'_, ()> {
        self.disk.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// The recordings being fetched right now, by Plaud ID.
    pub fn fetching(&self) -> HashSet<String> {
        self.fetching.lock().unwrap_or_else(|p| p.into_inner()).clone()
    }

    fn claim(self: &Arc<Self>, id: &str) -> Claim {
        let _disk = self.lock();
        self.fetching.lock().unwrap_or_else(|p| p.into_inner()).insert(id.to_string());
        Claim { guard: self.clone(), id: id.to_string() }
    }
}

/// A recording being fetched, released when dropped.
struct Claim {
    guard: Arc<Guard>,
    id: String,
}

impl Drop for Claim {
    fn drop(&mut self) {
        self.guard.fetching.lock().unwrap_or_else(|p| p.into_inner()).remove(&self.id);
    }
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
        // A rename half way through the scan could look like a deletion.
        let _disk = self.guard.lock();
        match self.library.scan()? {
            Some(mut scan) => {
                let tidied = self.library.tidy(&mut scan, &self.guard.fetching());
                for line in tidied.into_iter().chain(self.library.reconcile(&self.store(), &scan)?) {
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
                    State::New => self.config.sync_downloads_new && r.start_ms >= from,
                    State::Failed => r.start_ms >= from,
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
        if report.new_on_plaud > 0 && !self.config.sync_downloads_new {
            emit(Event::Log(
                "New recordings are not downloaded when syncing, as set in the settings. They are in the \"Not yet downloaded\" view.".into(),
            ));
        }
        let Some(work) = self.work()? else {
            report.needs_first_sync_choice = true;
            return Ok(report);
        };
        if work.is_empty() {
            emit(Event::Log("Everything is up to date.".into()));
            return Ok(report);
        }
        emit(Event::Log(self.plan_line(&work)));
        let ids: Vec<String> = work.into_iter().map(|r| r.plaud_id).collect();
        let (fetched, checked, failed) = self.fetch_many(ids, true, cancel, emit.clone()).await?;
        report.fetched = fetched;
        report.failed = failed;
        if checked > 0 {
            emit(Event::Log(checked_line(checked)));
        }
        emit(Event::Log(match (fetched, failed) {
            (0, 0) => "Sync finished. Everything is up to date.".to_string(),
            (f, 0) => format!("Sync finished. {} downloaded.", plural(f, "recording")),
            (f, n) => format!("Sync finished. {} downloaded, {} failed. The Log says why for each.", plural(f, "recording"), n),
        }));
        Ok(report)
    }

    /// What a sync is about to fetch, and why, for the Log. Laurent found a
    /// sync that said "None are new" and then downloaded two recordings
    /// confusing (milestone 3, round 2).
    fn plan_line(&self, work: &[Record]) -> String {
        let (mut fresh, mut unfinished, mut recheck) = (0, 0, 0);
        for r in work {
            match r.state {
                State::New | State::Failed => fresh += 1,
                _ if self.config.download_audio && !r.has_audio => unfinished += 1,
                _ => recheck += 1,
            }
        }
        plan_words(work.len(), fresh, unfinished, recheck)
    }

    /// Fetch the given recordings whatever their state: Download selected and
    /// Download again. With `fold_checks`, recordings checked again with
    /// nothing new are counted rather than each given a line in the Log, as a
    /// sync does. Returns (recordings with something new, recordings checked
    /// with nothing new, failures).
    pub async fn fetch_many(
        self: &Arc<Self>,
        ids: Vec<String>,
        fold_checks: bool,
        cancel: &Cancel,
        emit: Emit,
    ) -> Result<(usize, usize, usize)> {
        let total = ids.len();
        // As each recording stood when asked for, to tell a deletion made
        // while it waited from a Download again.
        let before: HashMap<String, State> = {
            let store = self.store();
            ids.iter().filter_map(|id| Some((id.clone(), store.get(id).ok().flatten()?.state))).collect()
        };
        let gate = Arc::new(Semaphore::new(AT_ONCE));
        let mut tasks = JoinSet::new();
        for id in ids {
            let (engine, gate, cancel, emit) = (self.clone(), gate.clone(), cancel.clone(), emit.clone());
            let was = before.get(&id).copied();
            tasks.spawn(async move {
                let _permit = gate.acquire_owned().await;
                if cancel.is_cancelled() {
                    return Err(Error::Cancelled);
                }
                let _claim = engine.guard.claim(&id);
                if engine.left_out(&id, was, &emit) {
                    return Ok(None);
                }
                let result = engine.fetch_one(&id, &cancel).await;
                engine.after_fetch(&id, &result, fold_checks, &emit);
                result.map(Some)
            });
        }
        let (mut fetched, mut checked, mut failed, mut done, mut cancelled) = (0, 0, 0, 0, false);
        emit(Event::Progress { done: 0, total });
        while let Some(joined) = tasks.join_next().await {
            match joined {
                // Deleted while it waited: nothing to count but progress.
                Ok(Ok(None)) => done += 1,
                Ok(Ok(Some(new))) => {
                    if new.is_empty() {
                        checked += 1;
                    } else {
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
        Ok((fetched, checked, failed))
    }

    /// True if the user deleted the recording, or chose Don't download, while
    /// it waited to be fetched. It is left alone, and the Log says so.
    fn left_out(&self, id: &str, was: Option<State>, emit: &Emit) -> bool {
        let gone = |s: State| matches!(s, State::Deleted | State::Skipped);
        let store = self.store();
        let Ok(Some(r)) = store.get(id) else { return false };
        if !gone(r.state) || was.is_some_and(gone) {
            return false;
        }
        emit(Event::Log(format!(
            "Did not download \"{}\", because it was deleted or marked Don't download while it waited.",
            library::display_name(&r)
        )));
        emit(Event::Changed(id.to_string()));
        true
    }

    fn after_fetch(&self, id: &str, result: &Result<Vec<String>>, fold_checks: bool, emit: &Emit) {
        let store = self.store();
        let Ok(Some(r)) = store.get(id) else { return };
        let name = library::display_name(&r);
        match result {
            Ok(new) if new.is_empty() => {
                let _ = store.set_error(id, None);
                if !fold_checks {
                    emit(Event::Log(format!("Checked \"{name}\" again: {}", still_missing(&r))));
                }
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

/// "Fetching 8 recordings: 2 not finished last time, and 6 to check again
/// for a transcript or summary."
fn plan_words(total: usize, fresh: usize, unfinished: usize, recheck: usize) -> String {
    let mut parts = Vec::new();
    if fresh > 0 {
        parts.push(format!("{fresh} not yet downloaded"));
    }
    if unfinished > 0 {
        parts.push(format!("{unfinished} not finished last time"));
    }
    if recheck > 0 {
        parts.push(format!("{recheck} to check again for a transcript or summary"));
    }
    let list = match parts.as_slice() {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{}, and {last}", rest.join(", ")),
    };
    format!("Fetching {}: {list}.", plural(total, "recording"))
}

fn checked_line(n: usize) -> String {
    if n == 1 {
        "Checked 1 recording again for a transcript or summary: Plaud has nothing new for it yet.".into()
    } else {
        format!("Checked {n} recordings again for a transcript or summary: Plaud has nothing new for them yet.")
    }
}

fn plural(n: usize, unit: &str) -> String {
    if n == 1 { format!("1 {unit}") } else { format!("{n} {unit}s") }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_claim_lasts_as_long_as_the_fetch() {
        let guard = Arc::new(Guard::default());
        let claim = guard.claim("a");
        assert!(guard.fetching().contains("a"), "a change on it is refused meanwhile");
        drop(claim);
        assert!(guard.fetching().is_empty(), "and allowed again afterwards");
    }

    #[test]
    fn the_plan_says_why_each_recording_is_fetched() {
        assert_eq!(
            plan_words(8, 0, 2, 6),
            "Fetching 8 recordings: 2 not finished last time, and 6 to check again for a transcript or summary."
        );
        assert_eq!(plan_words(1, 1, 0, 0), "Fetching 1 recording: 1 not yet downloaded.");
        assert_eq!(
            plan_words(6, 3, 1, 2),
            "Fetching 6 recordings: 3 not yet downloaded, 1 not finished last time, and 2 to check again for a transcript or summary."
        );
    }

    #[test]
    fn checks_are_counted_in_one_line() {
        assert!(checked_line(1).contains("1 recording again") && checked_line(1).ends_with("for it yet."));
        assert!(checked_line(6).contains("6 recordings again") && checked_line(6).ends_with("for them yet."));
    }
}
