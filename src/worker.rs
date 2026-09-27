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
use crate::store::{State, Store};
use crate::sync::{Engine, Event};

/// Posted to the window when events are waiting in the queue.
pub const WM_APP_EVENTS: u32 = WM_APP + 10;

pub enum Command {
    Refresh,
    Sync,
    Download(Vec<String>),
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
    /// The name part, without the date.
    pub name: String,
    pub folder: Option<String>,
    pub date: String,
    pub duration: String,
    pub status: &'static str,
    pub state: State,
    pub location: Option<String>,
    pub incomplete: bool,
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

struct Context {
    config: Config,
    library: Library,
    store: Store,
    engine: Option<Arc<Engine>>,
}

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
    let mut cx = Context { config, library, store, engine: None };

    // Show what is known straight away, before the network answers.
    if let Some(scan) = cx.library.scan().ok().flatten() {
        let _ = cx.library.reconcile(&cx.store, &scan);
    }
    send_snapshot(&cx, &sink);

    match connect(&cx).await {
        Ok(engine) => {
            cx.engine = Some(engine);
            if cx.config.sync_on_start {
                run_command(&cx, Command::Sync, startup, &sink).await;
            } else {
                sink.send(UiEvent::Finished);
            }
        }
        Err(e) => {
            sink.announce(e.to_string(), Importance::Error);
            sink.send(UiEvent::Finished);
        }
    }

    while let Some((command, cancel)) = rx.recv().await {
        if cx.engine.is_none() {
            match connect(&cx).await {
                Ok(engine) => cx.engine = Some(engine),
                Err(e) => {
                    sink.announce(e.to_string(), Importance::Error);
                    sink.send(UiEvent::Finished);
                    continue;
                }
            }
        }
        run_command(&cx, command, cancel, &sink).await;
    }
}

async fn connect(cx: &Context) -> Result<Arc<Engine>, Error> {
    let tokens = crate::auth::load()?.ok_or(Error::NotSignedIn)?;
    let http = crate::http_client()?;
    let client = crate::mcp::Client::connect(http.clone(), tokens).await?;
    let store = Store::open(&config::local_dir().join("notefetch.db"))?;
    Ok(Arc::new(Engine {
        http,
        client,
        store: Mutex::new(store),
        library: Library::new(cx.config.output_folder.clone()),
        config: cx.config.clone(),
    }))
}

async fn run_command(cx: &Context, command: Command, cancel: Cancel, sink: &Sink) {
    let Some(engine) = cx.engine.clone() else {
        sink.send(UiEvent::Finished);
        return;
    };
    let last_line = Arc::new(Mutex::new(String::new()));
    let emit = {
        let (sink, last_line) = (sink.clone(), last_line.clone());
        let cx_snapshot = SnapshotSource { db: config::local_dir().join("notefetch.db"), output: cx.config.output_folder.clone() };
        Arc::new(move |e: Event| match e {
            Event::Log(line) => {
                if let Ok(mut l) = last_line.lock() {
                    l.clone_from(&line);
                }
                sink.log(line);
            }
            Event::Progress { done, total } => sink.send(UiEvent::Progress { done, total }),
            Event::Changed(_) => {
                if let Some(s) = cx_snapshot.read() {
                    sink.send(UiEvent::Snapshot(s));
                }
            }
        }) as Arc<dyn Fn(Event) + Send + Sync>
    };
    let last = || last_line.lock().map(|l| l.clone()).unwrap_or_default();

    // The final line to speak, and whether the engine has already logged it.
    let outcome: Result<Option<(String, bool)>, Error> = match command {
        Command::Refresh => {
            sink.announce("Refreshing the list from Plaud.", Importance::Routine);
            engine.refresh(&emit).await.map(|_| Some((last(), true)))
        }
        Command::Sync => {
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
            sink.announce(
                if n == 1 { "Downloading 1 recording.".to_string() } else { format!("Downloading {n} recordings.") },
                Importance::Routine,
            );
            match engine.fetch_many(ids, &cancel, emit.clone()).await {
                Ok((fetched, 0)) => Ok(Some((format!("Download finished. {} downloaded.", plural(fetched, "recording")), false))),
                Ok((fetched, failed)) => Ok(Some((
                    format!("Download finished. {} downloaded, {failed} failed. The Log says why for each.", plural(fetched, "recording")),
                    false,
                ))),
                Err(e) => Err(e),
            }
        }
    };
    match outcome {
        Ok(Some((line, true))) => sink.send(UiEvent::Speak(line, Importance::Completion)),
        Ok(Some((line, false))) => sink.announce(line, Importance::Completion),
        Ok(None) => {}
        Err(Error::Cancelled) => sink.announce("Cancelled. Anything already downloaded has been kept.", Importance::Completion),
        Err(e) => sink.announce(e.to_string(), Importance::Error),
    }
    send_snapshot(cx, sink);
    sink.send(UiEvent::Finished);
}

fn send_snapshot(cx: &Context, sink: &Sink) {
    if let Some(s) = snapshot(&cx.store, &cx.library) {
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
            Row {
                name: r.local_name.clone().unwrap_or_else(|| crate::naming::name_from_plaud(&r.plaud_name)),
                folder: r.location.as_deref().and_then(library::folder_of).map(str::to_string),
                date: local.list_date(),
                duration: clock::duration_words(r.duration_ms),
                status: r.status(),
                incomplete: r.state == State::Failed
                    || (r.state == State::Downloaded && !(r.has_audio && r.has_transcript && r.has_summary)),
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
