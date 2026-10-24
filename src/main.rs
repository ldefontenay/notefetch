// No console window: NoteFetch is a window. The developer flags below still
// print to a terminal by attaching to it.
#![windows_subsystem = "windows"]

//! NoteFetch: download Plaud recordings, transcripts and summaries.
//!
//! Run with no arguments, it opens the main window (`ui.rs`). Hidden developer
//! flags drive the engine from a terminal instead, with plain-text output only:
//!
//! - `--sign-in` signs in to Plaud in the browser.
//! - `--sign-out` forgets the sign-in.
//! - `--refresh` lists Plaud's recordings and checks the output folder.
//! - `--sync` syncs. The first time, add `--first=all`, `--first=now` or
//!   `--first=N` for the last N days: the choice the app will ask for.
//! - `--list` prints the database.
//! - `--rename ID NAME`, `--move ID FOLDER` (or `-` for none), `--delete ID`,
//!   `--skip ID`, `--new-folder NAME`, `--rename-folder OLD NEW` and
//!   `--delete-folder NAME` do what the window's commands will do.
//! - `--tools` lists the tools Plaud's server offers, and saves the full
//!   reply as `tools-list.json` beside the database.
//! - `--write-settings` writes the settings file with every setting in it.
//! - `--first-sync-question` shows the first-sync question on its own, with
//!   the recordings in the database, and prints the choice. Nothing is saved.
//! - `--sounds` plays each of NoteFetch's sounds in turn at the volume in the
//!   settings, and `--sounds done` (or finished, refused, attention) just one.
//!
//! Set `NOTEFETCH_DATA` to a folder such as `test-data\dev` to keep the
//! settings, sign-in and database there instead of in the user's profile.

mod actions;
mod announce;
mod auth;
mod cancel;
mod clock;
mod config;
mod dialogs;
mod download;
mod error;
mod help;
mod library;
mod mcp;
mod naming;
mod plaud;
mod sound;
mod store;
mod sync;
mod ui;
mod worker;

use std::sync::{Arc, Mutex};

use crate::cancel::Cancel;
use crate::config::Config;
use crate::error::{Error, Result};
use crate::sync::{Engine, Event, FirstSync};

pub fn http_client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent(concat!("NoteFetch/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(std::time::Duration::from_secs(20))
        .read_timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|_| Error::Network("Windows' secure connection support could not be started.".into()))
}

fn load_config() -> Config {
    let (config, problems) = Config::load();
    for p in problems {
        println!("Settings: {p}");
    }
    config
}

async fn engine() -> Result<Arc<Engine>> {
    let config = load_config();
    let tokens = auth::load()?.ok_or(Error::NotSignedIn)?;
    let http = http_client()?;
    let client = mcp::Client::connect(http.clone(), tokens).await?;
    let store = store::Store::open(&config::local_dir().join("notefetch.db"))?;
    Ok(Arc::new(Engine {
        http,
        client,
        store: Mutex::new(store),
        library: library::Library::new(config.output_folder.clone()),
        config,
        guard: Default::default(),
    }))
}

fn print_event(e: Event) {
    match e {
        Event::Log(line) => println!("{line}"),
        Event::Progress { done, total } => println!("Progress: {done} of {total}."),
        Event::Changed(_) => {}
    }
}

fn first_sync_flag(args: &[String]) -> Option<FirstSync> {
    let v = args.iter().find_map(|a| a.strip_prefix("--first="))?;
    match v {
        "all" => Some(FirstSync::Everything),
        "now" => Some(FirstSync::FromNow),
        n => n.parse().ok().map(FirstSync::LastDays),
    }
}

async fn run(args: Vec<String>) -> Result<()> {
    let command = args.first().map(String::as_str).unwrap_or("");
    match command {
        "--sign-in" => {
            let http = http_client()?;
            let cancel = Cancel::new();
            auth::sign_in(&http, &cancel, &|line| println!("{line}")).await?;
            println!("Signed in to Plaud.");
        }
        "--sign-out" => {
            auth::forget()?;
            println!("Signed out. The sign-in has been removed from this computer.");
        }
        "--refresh" => {
            let e = engine().await?;
            e.refresh(&(Arc::new(print_event) as Arc<dyn Fn(Event) + Send + Sync>)).await?;
        }
        "--sync" => {
            let e = engine().await?;
            if let Some(choice) = first_sync_flag(&args) {
                e.set_first_sync(choice)?;
            }
            let cancel = Cancel::new();
            let report = e.sync(&cancel, Arc::new(print_event)).await?;
            if report.needs_first_sync_choice {
                println!("This is the first sync. Run it again with --first=all, --first=now or --first=N for the last N days.");
            }
        }
        "--tools" => {
            let e = engine().await?;
            let tools = e.client.list_tools().await?;
            let path = config::local_dir().join("tools-list.json");
            let text = serde_json::to_string_pretty(&tools).unwrap_or_default();
            std::fs::write(&path, text).map_err(|err| error::disk("save the tool list to", &path, &err))?;
            for t in tools["tools"].as_array().into_iter().flatten() {
                println!("{}", t["name"].as_str().unwrap_or("(no name)"));
            }
            println!("Full list saved to {}", path.display());
        }
        "--write-settings" => {
            let config = load_config();
            config.save().map_err(|e| error::disk("save the settings to", &config::config_path(), &e))?;
            println!("Settings written to {}", config::config_path().display());
        }
        "--rename" | "--move" | "--delete" | "--skip" | "--new-folder" | "--rename-folder" | "--delete-folder" => {
            // Local changes only, so no sign-in is needed.
            let config = load_config();
            let store = store::Store::open(&config::local_dir().join("notefetch.db"))?;
            let library = library::Library::new(config.output_folder.clone());
            let e = actions::Local { store: &store, library: &library };
            let arg = |i: usize| args.get(i).map(String::as_str).ok_or_else(|| Error::Disk(format!("{command} needs more arguments.")));
            let line = match command {
                "--rename" => e.rename(arg(1)?, arg(2)?)?,
                "--move" => e.move_to_folder(arg(1)?, args.get(2).map(String::as_str).filter(|f| *f != "-"))?,
                "--delete" => e.delete(arg(1)?)?,
                "--skip" => e.skip(arg(1)?)?,
                "--new-folder" => e.create_folder(arg(1)?)?,
                "--rename-folder" => e.rename_folder(arg(1)?, arg(2)?)?,
                _ => e.delete_folder(arg(1)?)?,
            };
            println!("{line}");
        }
        "--first-sync-question" => {
            let store = store::Store::open(&config::local_dir().join("notefetch.db"))?;
            let waiting: Vec<(i64, u64)> = store
                .all()?
                .into_iter()
                .filter(|r| matches!(r.state, store::State::New | store::State::Failed))
                .map(|r| (r.start_ms, r.duration_ms))
                .collect();
            let choice = dialogs::first_sync(windows::Win32::Foundation::HWND::default(), &waiting, Box::new(|t| println!("{t}")));
            println!("Chosen: {choice:?}");
        }
        "--sounds" => {
            // Each sound in turn, named first, with time to think between them.
            let volume = load_config().sound_volume;
            let only = args.get(1).map(|a| a.to_ascii_lowercase());
            for s in sound::ALL {
                let name = format!("{s:?}").to_ascii_lowercase();
                if only.as_ref().is_some_and(|o| *o != name) {
                    continue;
                }
                println!("{name}");
                sound::play_and_wait(s, volume);
                std::thread::sleep(std::time::Duration::from_millis(1500));
            }
        }
        "--list" => {
            let store = store::Store::open(&config::local_dir().join("notefetch.db"))?;
            for r in store.all()? {
                let local = clock::to_local(&clock::Stamp::from_unix_millis(r.start_ms));
                println!(
                    "{}. {}. {}. {}.",
                    library::display_name(&r),
                    local.list_date(),
                    clock::duration_words(r.duration_ms),
                    r.status()
                );
            }
        }
        other => {
            println!("NoteFetch does not know the option \"{other}\". See the developer flags at the top of src/main.rs.");
        }
    }
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        std::process::exit(ui::run());
    }
    // A developer flag: print to the terminal NoteFetch was started from.
    // SAFETY: attaching to the parent's console, if it has one.
    unsafe {
        let _ = windows::Win32::System::Console::AttachConsole(windows::Win32::System::Console::ATTACH_PARENT_PROCESS);
    }
    let runtime = match tokio::runtime::Builder::new_multi_thread().enable_all().build() {
        Ok(r) => r,
        Err(_) => {
            println!("NoteFetch could not start its background worker. Restart Windows and try again.");
            std::process::exit(2);
        }
    };
    if let Err(e) = runtime.block_on(run(args)) {
        println!("{e}");
        std::process::exit(1);
    }
}
