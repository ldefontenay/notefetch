//! The worker thread: network and disk work, off the interface thread.
//!
//! The window sends [`Command`]s over a channel. The worker runs them on its
//! own tokio runtime and puts [`UiEvent`]s in a queue, then posts
//! [`WM_APP_EVENTS`] to the window as a signal that events are waiting. The
//! window drains the queue on its own thread, which is the only thread that
//! touches windows (CLAUDE.md).

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tokio::sync::mpsc;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_APP};

use crate::cancel::Cancel;
use crate::clock::{self, Stamp};
use crate::config::{self, Config};
use crate::error::Error;
use crate::library::{self, Library};
use crate::actions::Local;
use crate::store::{State, Store};
use crate::sync::{Engine, Event, Guard};

/// Posted to the window when events are waiting in the queue.
pub const WM_APP_EVENTS: u32 = WM_APP + 10;

pub enum Command {
    Refresh,
    Sync,
    Download(Vec<String>),
    /// Sign in through the browser, then sync if sync on start is set.
    SignIn,
    SignOut,
    /// The user's own changes, which need no sign-in. SPEC.md "Playing,
    /// renaming and deleting" and "Folders".
    Rename(String, String),
    /// Downloaded recordings go to the Recycle Bin; ones never downloaded are
    /// marked Don't download, which is what Delete means for them.
    Delete(Vec<String>),
    Skip(Vec<String>),
    /// Move to a folder, or out of any folder with `None`. With `create`, the
    /// folder is made first.
    Move { ids: Vec<String>, folder: Option<String>, create: bool },
    NewFolder(String),
    RenameFolder(String, String),
    DeleteFolder(String),
}

impl Command {
    /// Work on the disk and the database alone, with no need for Plaud.
    fn is_local(&self) -> bool {
        matches!(
            self,
            Command::Rename(..)
                | Command::Delete(_)
                | Command::Skip(_)
                | Command::Move { .. }
                | Command::NewFolder(_)
                | Command::RenameFolder(..)
                | Command::DeleteFolder(_)
        )
    }
}

/// What the worker is doing, so closing the window knows whether to ask.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Activity {
    Connecting,
    SigningIn,
    Refreshing,
    Syncing,
    Downloading,
}

/// How much an announcement matters, for the announcement level setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Importance {
    /// Starting something, or progress.
    Routine,
    /// Something finished.
    Completion,
    Error,
}

/// One row of the recordings list.
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub id: String,
    /// The name part, without the date, with any clash number: "Untitled (2)".
    pub name: String,
    /// The name part alone, as F2 edits it: no date, no clash number.
    pub plain_name: String,
    pub folder: Option<String>,
    pub date: String,
    pub duration: String,
    pub status: &'static str,
    pub state: State,
    pub location: Option<String>,
    pub incomplete: bool,
    /// Whether a transcript and a summary were downloaded, so the menus can
    /// show Open transcript and Open summary as unavailable. Laurent, milestone 4.
    pub has_transcript: bool,
    pub has_summary: bool,
}

/// Everything the window shows, read fresh after each change.
#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    pub rows: Vec<Row>,
    pub folders: Vec<String>,
}

pub enum UiEvent {
    /// A line for the Log.
    Log(String),
    /// A line for the Log that is also spoken, depending on the setting.
    Announce(String, Importance),
    /// A line already in the Log, to be spoken, depending on the setting.
    Speak(String, Importance),
    Progress { done: usize, total: usize },
    Snapshot(Snapshot),
    /// What the worker has started doing.
    Activity(Activity),
    /// The sign-in is missing or no longer works, so the window offers to
    /// sign in. True if there was one and it has expired.
    SignInNeeded(bool),
    /// The command that was running has finished, one way or another.
    Finished,
}

#[derive(Clone)]
struct Sink {
    queue: Arc<Mutex<VecDeque<UiEvent>>>,
    /// The window, as a number: a window handle cannot cross threads, but the
    /// number can, and posting to it is safe from any thread.
    hwnd: isize,
}

impl Sink {
    fn send(&self, event: UiEvent) {
        if let Ok(mut q) = self.queue.lock() {
            q.push_back(event);
        }
        // SAFETY: PostMessageW may be called from any thread; a window that
        // has gone away makes it fail harmlessly.
        unsafe {
            let _ = PostMessageW(Some(HWND(self.hwnd as *mut _)), WM_APP_EVENTS, WPARAM(0), LPARAM(0));
        }
    }

    fn log(&self, line: impl Into<String>) {
        self.send(UiEvent::Log(line.into()));
    }

    fn announce(&self, line: impl Into<String>, importance: Importance) {
        self.send(UiEvent::Announce(line.into(), importance));
    }

    /// Announce an error, and offer to sign in if that is what it needs.
    fn error(&self, e: Error) {
        let needed = match e {
            Error::NotSignedIn => Some(false),
            Error::SignInExpired => Some(true),
            _ => None,
        };
        self.announce(e.to_string(), Importance::Error);
        if let Some(expired) = needed {
            self.send(UiEvent::SignInNeeded(expired));
        }
    }
}

pub struct Worker {
    tx: mpsc::UnboundedSender<(Command, Cancel)>,
}

impl Worker {
    /// Start the worker for the window `hwnd`, delivering events into `queue`.
    /// `startup` cancels the sync that runs at start-up.
    pub fn start(hwnd: HWND, queue: Arc<Mutex<VecDeque<UiEvent>>>, startup: Cancel) -> Worker {
        let (tx, rx) = mpsc::unbounded_channel();
        let sink = Sink { queue, hwnd: hwnd.0 as isize };
        std::thread::spawn(move || {
            let runtime = match tokio::runtime::Builder::new_multi_thread().enable_all().build() {
                Ok(r) => r,
                Err(_) => {
                    sink.announce(
                        "NoteFetch could not start its background worker, so it cannot reach Plaud. Close NoteFetch and open it again.",
                        Importance::Error,
                    );
                    return;
                }
            };
            runtime.block_on(run(rx, sink, startup));
        });
        Worker { tx }
    }

    pub fn send(&self, command: Command, cancel: Cancel) {
        let _ = self.tx.send((command, cancel));
    }
}

/// What the network tasks share with the loop. The loop's own database
/// connection stays with the loop, because a connection cannot be shared
/// between threads.
struct Shared {
    config: Config,
    guard: Arc<Guard>,
    engine: Mutex<Option<Arc<Engine>>>,
    snapshots: SnapshotSource,
}

impl Shared {
    fn engine(&self) -> Option<Arc<Engine>> {
        self.engine.lock().ok().and_then(|e| e.clone())
    }

    fn set_engine(&self, engine: Option<Arc<Engine>>) {
        if let Ok(mut e) = self.engine.lock() {
            *e = engine;
        }
    }

    fn send_snapshot(&self, sink: &Sink) {
        if let Some(s) = self.snapshots.read() {
            sink.send(UiEvent::Snapshot(s));
        }
    }
}

/// The loop takes each command as it comes. Syncs, downloads and sign-in run
/// as tasks of their own, so the user's changes are not held up behind them
/// (Laurent, milestone 4). The window starts only one of those at a time.
async fn run(mut rx: mpsc::UnboundedReceiver<(Command, Cancel)>, sink: Sink, startup: Cancel) {
    let (config, problems) = Config::load();
    for p in problems {
        sink.log(format!("Settings: {p}"));
    }
    let db = config::local_dir().join("notefetch.db");
    let store = match Store::open(&db) {
        Ok(s) => s,
        Err(e) => {
            sink.announce(e.to_string(), Importance::Error);
            sink.send(UiEvent::Finished);
            return;
        }
    };
    let library = Library::new(config.output_folder.clone());
    let shared = Arc::new(Shared {
        snapshots: SnapshotSource { db, output: config.output_folder.clone() },
        config,
        guard: Arc::default(),
        engine: Mutex::new(None),
    });

    // Show what is known straight away, before the network answers.
    if let Some(scan) = library.scan().ok().flatten() {
        let _ = library.reconcile(&store, &scan);
    }
    send_snapshot(&store, &library, &sink);

    tokio::spawn(start_up(shared.clone(), startup, sink.clone()));
    while let Some((command, cancel)) = rx.recv().await {
        match command {
            c if c.is_local() => local_command(&shared, &store, &library, c, &sink),
            Command::SignOut => sign_out(&shared, &sink),
            c => {
                tokio::spawn(network(shared.clone(), c, cancel, sink.clone()));
            }
        }
    }
}

async fn start_up(shared: Arc<Shared>, cancel: Cancel, sink: Sink) {
    match connect(&shared).await {
        Ok(engine) => {
            shared.set_engine(Some(engine));
            if shared.config.sync_on_start {
                run_command(&shared, Command::Sync, cancel, &sink).await;
            } else {
                sink.send(UiEvent::Finished);
            }
        }
        Err(e) => {
            sink.send(UiEvent::Finished);
            sink.error(e);
        }
    }
}

/// Sign-in, sync, refresh and download: everything that needs Plaud.
async fn network(shared: Arc<Shared>, command: Command, cancel: Cancel, sink: Sink) {
    if let Command::SignIn = command {
        sign_in(&shared, cancel, &sink).await;
        return;
    }
    if shared.engine().is_none() {
        sink.send(UiEvent::Activity(Activity::Connecting));
        match connect(&shared).await {
            Ok(engine) => shared.set_engine(Some(engine)),
            Err(e) => {
                sink.send(UiEvent::Finished);
                sink.error(e);
                return;
            }
        }
    }
    run_command(&shared, command, cancel, &sink).await;
}

/// Sign in through the browser. Escape cancels; it gives up by itself after
/// five minutes.
async fn sign_in(shared: &Shared, cancel: Cancel, sink: &Sink) {
    sink.send(UiEvent::Activity(Activity::SigningIn));
    sink.announce("Signing in to Plaud.", Importance::Routine);
    let result = async {
        let http = crate::http_client()?;
        let status = |line: String| sink.announce(line, Importance::Completion);
        crate::auth::sign_in(&http, &cancel, &status).await?;
        connect(shared).await
    }
    .await;
    match result {
        Ok(engine) => {
            shared.set_engine(Some(engine));
            sink.announce("Signed in to Plaud.", Importance::Completion);
            if shared.config.sync_on_start && !cancel.is_cancelled() {
                run_command(shared, Command::Sync, cancel, sink).await;
                return;
            }
        }
        Err(Error::Cancelled) => {
            sink.announce("Sign-in cancelled. You can sign in later from the File menu.", Importance::Completion)
        }
        // Not sink.error: offering to sign in again straight after a failed
        // sign-in would only go round in a circle.
        Err(e) => sink.announce(e.to_string(), Importance::Error),
    }
    sink.send(UiEvent::Finished);
}

fn sign_out(shared: &Shared, sink: &Sink) {
    shared.set_engine(None);
    match crate::auth::forget() {
        Ok(()) => sink.announce(
            "Signed out of Plaud. Your downloaded recordings are still in the output folder. To sign in again, choose Sign in from the File menu.",
            Importance::Completion,
        ),
        Err(e) => sink.announce(e.to_string(), Importance::Error),
    }
    sink.send(UiEvent::Finished);
}

async fn connect(shared: &Shared) -> Result<Arc<Engine>, Error> {
    let tokens = crate::auth::load()?.ok_or(Error::NotSignedIn)?;
    let http = crate::http_client()?;
    let client = crate::mcp::Client::connect(http.clone(), tokens).await?;
    let store = Store::open(&shared.snapshots.db)?;
    Ok(Arc::new(Engine {
        http,
        client,
        store: Mutex::new(store),
        library: Library::new(shared.config.output_folder.clone()),
        config: shared.config.clone(),
        guard: shared.guard.clone(),
    }))
}

async fn run_command(shared: &Shared, command: Command, cancel: Cancel, sink: &Sink) {
    let Some(engine) = shared.engine() else {
        sink.send(UiEvent::Finished);
        return;
    };
    let last_line = Arc::new(Mutex::new(String::new()));
    let emit = {
        let (sink, last_line) = (sink.clone(), last_line.clone());
        let source = SnapshotSource { db: shared.snapshots.db.clone(), output: shared.snapshots.output.clone() };
        Arc::new(move |e: Event| match e {
            Event::Log(line) => {
                if let Ok(mut l) = last_line.lock() {
                    l.clone_from(&line);
                }
                sink.log(line);
            }
            Event::Progress { done, total } => sink.send(UiEvent::Progress { done, total }),
            Event::Changed(_) => {
                if let Some(s) = source.read() {
                    sink.send(UiEvent::Snapshot(s));
                }
            }
        }) as Arc<dyn Fn(Event) + Send + Sync>
    };
    let last = || last_line.lock().map(|l| l.clone()).unwrap_or_default();

    // The final line to speak, and whether the engine has already logged it.
    let outcome: Result<Option<(String, bool)>, Error> = match command {
        Command::Refresh => {
            sink.send(UiEvent::Activity(Activity::Refreshing));
            sink.announce("Refreshing the list from Plaud.", Importance::Routine);
            engine.refresh(&emit).await.map(|_| Some((last(), true)))
        }
        Command::Sync => {
            sink.send(UiEvent::Activity(Activity::Syncing));
            sink.announce("Syncing with Plaud.", Importance::Routine);
            match engine.sync(&cancel, emit.clone()).await {
                Ok(report) if report.needs_first_sync_choice => Ok(Some((
                    "Nothing was downloaded, because NoteFetch has not yet asked how much of your Plaud history to download. That question arrives in a later version.".to_string(),
                    false,
                ))),
                Ok(_) => Ok(Some((last(), true))),
                Err(e) => Err(e),
            }
        }
        Command::Download(ids) => {
            let n = ids.len();
            sink.send(UiEvent::Activity(Activity::Downloading));
            sink.announce(
                if n == 1 { "Downloading 1 recording.".to_string() } else { format!("Downloading {n} recordings.") },
                Importance::Routine,
            );
            match engine.fetch_many(ids, false, &cancel, emit.clone()).await {
                Ok((fetched, _, 0)) => Ok(Some((format!("Download finished. {} downloaded.", plural(fetched, "recording")), false))),
                Ok((fetched, _, failed)) => Ok(Some((
                    format!("Download finished. {} downloaded, {failed} failed. The Log says why for each.", plural(fetched, "recording")),
                    false,
                ))),
                Err(e) => Err(e),
            }
        }
        // Sign-in and local changes are handled before a connection is needed.
        _ => Ok(None),
    };
    match outcome {
        Ok(Some((line, true))) => sink.send(UiEvent::Speak(line, Importance::Completion)),
        Ok(Some((line, false))) => sink.announce(line, Importance::Completion),
        Ok(None) => {}
        Err(Error::Cancelled) => sink.announce("Cancelled. Anything already downloaded has been kept.", Importance::Completion),
        Err(e) => {
            shared.send_snapshot(sink);
            sink.send(UiEvent::Finished);
            sink.error(e);
            return;
        }
    }
    shared.send_snapshot(sink);
    sink.send(UiEvent::Finished);
}

/// The user's own changes to the disk and the database. Each result goes to
/// the Log; one line is spoken at the end. SPEC.md "Playing, renaming and
/// deleting" and "Folders". They run while a sync or download does, except
/// on the recordings it is fetching at that moment.
fn local_command(shared: &Shared, store: &Store, library: &Library, command: Command, sink: &Sink) {
    let _disk = shared.guard.lock();
    let fetching = shared.guard.fetching();
    let local = Local { store, library };
    // Refuse a recording being fetched, naming it.
    let wait = |id: &str, verb: &str| -> Option<Error> {
        fetching.contains(id).then(|| {
            let name = store.get(id).ok().flatten().map(|r| library::display_name(&r)).unwrap_or_default();
            Error::Disk(format!("\"{name}\" is being downloaded, so it was not {verb}. Try again when the download finishes."))
        })
    };
    // Refuse a folder holding a recording being fetched.
    let busy_folder = |folder: &str, verb: &str| -> Option<Error> {
        let inside = fetching.iter().any(|id| {
            store.get(id).ok().flatten().and_then(|r| r.location).as_deref().and_then(library::folder_of) == Some(folder)
        });
        inside.then(|| {
            Error::Disk(format!(
                "A recording in the folder \"{folder}\" is being downloaded, so the folder was not {verb}. Try again when the download finishes."
            ))
        })
    };
    let mut results: Vec<Result<String, Error>> = Vec::new();
    // What a group of results adds up to, when there is more than one.
    let summary: Option<(&str, usize)> = match command {
        Command::Rename(id, name) => {
            results.push(match wait(&id, "renamed") {
                Some(e) => Err(e),
                None => local.rename(&id, &name),
            });
            None
        }
        Command::Delete(ids) => {
            let n = ids.len();
            for id in &ids {
                if let Some(e) = wait(id, "deleted") {
                    results.push(Err(e));
                    continue;
                }
                let state = store.get(id).ok().flatten().map(|r| r.state);
                results.push(match state {
                    Some(State::New | State::Failed) => local.skip(id),
                    _ => local.delete(id),
                });
            }
            Some(("deleted", n))
        }
        Command::Skip(ids) => {
            let n = ids.len();
            for id in &ids {
                results.push(match wait(id, "marked Don't download") {
                    Some(e) => Err(e),
                    None => local.skip(id),
                });
            }
            Some(("marked Don't download", n))
        }
        Command::Move { ids, folder, create } => {
            let n = ids.len();
            let mut target = folder;
            if create && let Some(name) = target.clone() {
                match library.create_folder(&name) {
                    Ok(made) => {
                        results.push(Ok(format!("Created the folder \"{made}\".")));
                        target = Some(made);
                    }
                    // No folder, so nothing can move into it.
                    Err(e) => results.push(Err(e)),
                }
            }
            if results.iter().all(Result::is_ok) {
                for id in &ids {
                    results.push(match wait(id, "moved") {
                        Some(e) => Err(e),
                        None => local.move_to_folder(id, target.as_deref()),
                    });
                }
            }
            Some(("moved", n))
        }
        Command::NewFolder(name) => {
            results.push(local.create_folder(&name));
            None
        }
        Command::RenameFolder(old, new) => {
            results.push(match busy_folder(&old, "renamed") {
                Some(e) => Err(e),
                None => local.rename_folder(&old, &new),
            });
            None
        }
        Command::DeleteFolder(name) => {
            results.push(match busy_folder(&name, "deleted") {
                Some(e) => Err(e),
                None => local.delete_folder(&name),
            });
            None
        }
        _ => None,
    };
    let (mut done, mut failed) = (Vec::new(), Vec::new());
    for r in results {
        match r {
            Ok(line) => done.push(line),
            Err(e) => failed.push(e.to_string()),
        }
    }
    for line in done.iter().chain(&failed) {
        sink.log(line.clone());
    }
    send_snapshot(store, library, sink);
    match (summary, failed.len()) {
        (Some((verb, n)), 0) if n > 1 => {
            sink.announce(format!("{n} recordings {verb}."), Importance::Completion)
        }
        (_, 0) => {
            if let Some(line) = done.last() {
                sink.send(UiEvent::Speak(line.clone(), Importance::Completion));
            }
        }
        (Some((verb, n)), bad) if n > 1 => sink.announce(
            format!("{bad} of {n} recordings could not be {verb}. The Log says why for each."),
            Importance::Error,
        ),
        (_, _) => sink.send(UiEvent::Speak(failed[0].clone(), Importance::Error)),
    }
}

fn send_snapshot(store: &Store, library: &Library, sink: &Sink) {
    if let Some(s) = snapshot(store, library) {
        sink.send(UiEvent::Snapshot(s));
    }
}

/// Opens its own database connection, so it can be used from inside a sync.
struct SnapshotSource {
    db: PathBuf,
    output: PathBuf,
}

impl SnapshotSource {
    fn read(&self) -> Option<Snapshot> {
        let store = Store::open(&self.db).ok()?;
        snapshot(&store, &Library::new(self.output.clone()))
    }
}

fn snapshot(store: &Store, library: &Library) -> Option<Snapshot> {
    let records = store.all().ok()?;
    let folders = library.scan().ok().flatten().map(|s| s.folders).unwrap_or_default();
    let rows = records
        .into_iter()
        .map(|r| {
            let local = clock::to_local(&Stamp::from_unix_millis(r.start_ms));
            let name = r.local_name.clone().unwrap_or_else(|| crate::naming::name_from_plaud(&r.plaud_name));
            // "Untitled (2)" for the second of two recordings started in the
            // same minute, as its folder is named. Laurent, round 3.
            let clash = r.location.as_deref().map(library::base_of).and_then(crate::naming::clash_of).unwrap_or("");
            Row {
                name: format!("{name}{clash}"),
                plain_name: name,
                folder: r.location.as_deref().and_then(library::folder_of).map(str::to_string),
                date: local.list_date(),
                duration: clock::duration_words(r.duration_ms),
                status: r.status(),
                incomplete: r.state == State::Failed
                    || (r.state == State::Downloaded && !(r.has_audio && r.has_transcript && r.has_summary)),
                has_transcript: r.has_transcript,
                has_summary: r.has_summary,
                state: r.state,
                location: r.location.clone(),
                id: r.plaud_id,
            }
        })
        .collect();
    Some(Snapshot { rows, folders })
}

fn plural(n: usize, unit: &str) -> String {
    if n == 1 { format!("1 {unit}") } else { format!("{n} {unit}s") }
}
