//! Settings in `notefetch.conf`, and where NoteFetch keeps its files.
//!
//! The settings file is plain text, one `key = value` per line, safe to edit
//! by hand. It lives beside the exe when that folder can be written to, so a
//! copy on a USB stick carries its settings, and in `%APPDATA%\NoteFetch`
//! otherwise. SPEC.md "Files the app writes".

use std::path::{Path, PathBuf};

use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::UI::Shell::{FOLDERID_Documents, KF_FLAG_DEFAULT, SHGetKnownFolderPath};

pub const FILE_NAME: &str = "notefetch.conf";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Announcements {
    All,
    ErrorsAndCompletion,
    Off,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub output_folder: PathBuf,
    pub sync_on_start: bool,
    /// Download new recordings when syncing. Off, a sync only lists them, and
    /// they wait in "Not yet downloaded" (Laurent, milestone 3 round 3).
    pub sync_downloads_new: bool,
    pub confirm_delete: bool,
    /// Delete on a recording never downloaded marks it Don't download. Off,
    /// Delete does nothing to it (Laurent, milestone 4).
    pub delete_marks_dont_download: bool,
    pub download_audio: bool,
    pub download_transcript: bool,
    pub download_summary: bool,
    /// The cleaned transcript, outline and highlights.
    pub download_other: bool,
    /// Save notes, which are Markdown, with .txt instead of .md.
    pub notes_as_txt: bool,
    /// Days after a recording's start during which missing parts are retried.
    pub retry_days: u32,
    pub announcements: Announcements,
    pub completion_tone: bool,
    pub log_to_file: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            output_folder: documents_folder().join("NoteFetch"),
            sync_on_start: true,
            sync_downloads_new: true,
            confirm_delete: true,
            delete_marks_dont_download: true,
            download_audio: true,
            download_transcript: true,
            download_summary: true,
            download_other: true,
            notes_as_txt: false,
            retry_days: 14,
            announcements: Announcements::All,
            completion_tone: false,
            log_to_file: false,
        }
    }
}

impl Config {
    /// Read settings from text. Unknown keys and unreadable values are
    /// skipped and described in the returned list, so the Log can say which
    /// lines were ignored; the defaults stand for anything missing.
    pub fn parse(text: &str) -> (Config, Vec<String>) {
        let mut c = Config::default();
        let mut problems = Vec::new();
        for (number, raw) in text.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                problems.push(format!("Line {} has no equals sign, so it was ignored.", number + 1));
                continue;
            };
            let (key, value) = (key.trim().to_ascii_lowercase(), value.trim());
            let ok = match key.as_str() {
                "output_folder" if !value.is_empty() => {
                    c.output_folder = PathBuf::from(value);
                    true
                }
                "sync_on_start" => set_bool(&mut c.sync_on_start, value),
                "sync_downloads_new" => set_bool(&mut c.sync_downloads_new, value),
                "confirm_delete" => set_bool(&mut c.confirm_delete, value),
                "delete_marks_dont_download" => set_bool(&mut c.delete_marks_dont_download, value),
                "download_audio" => set_bool(&mut c.download_audio, value),
                "download_transcript" => set_bool(&mut c.download_transcript, value),
                "download_summary" => set_bool(&mut c.download_summary, value),
                "download_other" => set_bool(&mut c.download_other, value),
                "notes_as_txt" => set_bool(&mut c.notes_as_txt, value),
                "retry_days" => value.parse().map(|v| c.retry_days = v).is_ok(),
                "announcements" => match value.to_ascii_lowercase().as_str() {
                    "all" => Some(Announcements::All),
                    "errors and completion" => Some(Announcements::ErrorsAndCompletion),
                    "off" => Some(Announcements::Off),
                    _ => None,
                }
                .map(|a| c.announcements = a)
                .is_some(),
                "completion_tone" => set_bool(&mut c.completion_tone, value),
                "log_to_file" => set_bool(&mut c.log_to_file, value),
                _ => {
                    problems.push(format!("Line {}: \"{key}\" is not a NoteFetch setting, so it was ignored.", number + 1));
                    continue;
                }
            };
            if !ok {
                problems.push(format!(
                    "Line {}: \"{value}\" is not a valid value for {key}, so the default was used.",
                    number + 1
                ));
            }
        }
        (c, problems)
    }

    pub fn to_text(&self) -> String {
        let yn = |b: bool| if b { "yes" } else { "no" };
        let announcements = match self.announcements {
            Announcements::All => "all",
            Announcements::ErrorsAndCompletion => "errors and completion",
            Announcements::Off => "off",
        };
        format!(
            "# NoteFetch settings. Safe to edit while NoteFetch is closed.\n\
             # Yes or no settings take yes or no.\n\
             \n\
             # Where recordings are saved.\n\
             output_folder = {}\n\
             \n\
             # Look for new recordings when NoteFetch starts.\n\
             sync_on_start = {}\n\
             \n\
             # Download new recordings when syncing. With no, a sync only\n\
             # lists them, and you choose what to download.\n\
             sync_downloads_new = {}\n\
             \n\
             # Ask before moving a recording to the Recycle Bin.\n\
             confirm_delete = {}\n\
             \n\
             # Delete on a recording that was never downloaded marks it\n\
             # Don't download. With no, Delete leaves it alone.\n\
             delete_marks_dont_download = {}\n\
             \n\
             # Which parts to download. \"Other\" means the cleaned transcript,\n\
             # the outline and the highlights.\n\
             download_audio = {}\n\
             download_transcript = {}\n\
             download_summary = {}\n\
             download_other = {}\n\
             \n\
             # Save summaries and other notes as .txt instead of .md. The\n\
             # text inside is the same Markdown either way.\n\
             notes_as_txt = {}\n\
             \n\
             # For how many days after a recording to keep checking for a\n\
             # transcript or summary that Plaud has not made yet.\n\
             retry_days = {}\n\
             \n\
             # What is spoken: all, errors and completion, or off.\n\
             announcements = {announcements}\n\
             \n\
             # A short tone when a sync finishes.\n\
             completion_tone = {}\n\
             \n\
             # Also write the Log to notefetch.log.\n\
             log_to_file = {}\n",
            self.output_folder.display(),
            yn(self.sync_on_start),
            yn(self.sync_downloads_new),
            yn(self.confirm_delete),
            yn(self.delete_marks_dont_download),
            yn(self.download_audio),
            yn(self.download_transcript),
            yn(self.download_summary),
            yn(self.download_other),
            yn(self.notes_as_txt),
            self.retry_days,
            yn(self.completion_tone),
            yn(self.log_to_file),
        )
    }

    /// Load from the settings file, or defaults if there is none yet.
    pub fn load() -> (Config, Vec<String>) {
        match std::fs::read_to_string(config_path()) {
            Ok(text) => Config::parse(&text),
            Err(_) => (Config::default(), Vec::new()),
        }
    }

    pub fn save(&self) -> std::io::Result<()> {
        let path = config_path();
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        write_atomically(&path, self.to_text().as_bytes())
    }
}

fn set_bool(slot: &mut bool, value: &str) -> bool {
    match value.to_ascii_lowercase().as_str() {
        "yes" | "true" | "on" | "1" => *slot = true,
        "no" | "false" | "off" | "0" => *slot = false,
        _ => return false,
    }
    true
}

/// Write through a temporary file and rename, so a crash never leaves half a file.
pub fn write_atomically(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
}

/// The settings file: beside the exe if that folder is writable, else in
/// `%APPDATA%\NoteFetch`.
pub fn config_path() -> PathBuf {
    if let Some(dir) = dev_dir() {
        return dir.join(FILE_NAME);
    }
    if let Some(dir) = std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf)) {
        let beside = dir.join(FILE_NAME);
        if beside.exists() || folder_is_writable(&dir) {
            return beside;
        }
    }
    roaming_dir().join(FILE_NAME)
}

fn folder_is_writable(dir: &Path) -> bool {
    let probe = dir.join(".notefetch-write-test");
    let ok = std::fs::write(&probe, b"").is_ok();
    let _ = std::fs::remove_file(&probe);
    ok
}

/// `%APPDATA%\NoteFetch`: sign-in, and settings when they cannot sit beside the exe.
pub fn roaming_dir() -> PathBuf {
    dev_dir().unwrap_or_else(|| env_dir("APPDATA").join("NoteFetch"))
}

/// `%LOCALAPPDATA%\NoteFetch`: the database.
pub fn local_dir() -> PathBuf {
    dev_dir().unwrap_or_else(|| env_dir("LOCALAPPDATA").join("NoteFetch"))
}

/// Development only: `NOTEFETCH_DATA` puts the settings, sign-in and database
/// in a folder of the developer's choosing, such as `test-data`, so trying
/// things out never touches a real installation's state.
fn dev_dir() -> Option<PathBuf> {
    std::env::var_os("NOTEFETCH_DATA").filter(|v| !v.is_empty()).map(PathBuf::from)
}

fn env_dir(var: &str) -> PathBuf {
    std::env::var_os(var)
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// The user's Documents folder, wherever Windows has put it (it may be in OneDrive).
fn documents_folder() -> PathBuf {
    // SAFETY: the returned string is copied, then freed with CoTaskMemFree as
    // the API requires.
    unsafe {
        match SHGetKnownFolderPath(&FOLDERID_Documents, KF_FLAG_DEFAULT, None) {
            Ok(p) => {
                let path = p.to_string().map(PathBuf::from).ok();
                CoTaskMemFree(Some(p.0 as *const _));
                path
            }
            Err(_) => None,
        }
    }
    .unwrap_or_else(|| env_dir("USERPROFILE").join("Documents"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let c = Config {
            output_folder: PathBuf::from(r"D:\Lectures"),
            download_other: false,
            sync_downloads_new: false,
            delete_marks_dont_download: false,
            notes_as_txt: true,
            retry_days: 30,
            announcements: Announcements::ErrorsAndCompletion,
            ..Config::default()
        };
        let (back, problems) = Config::parse(&c.to_text());
        assert!(problems.is_empty(), "{problems:?}");
        assert_eq!(back, c);
    }

    #[test]
    fn hand_edits_are_forgiving() {
        let (c, problems) = Config::parse("  SYNC_ON_START = No \n# comment\nretry_days=7\ncompletion_tone = TRUE\n");
        assert!(problems.is_empty());
        assert!(!c.sync_on_start);
        assert_eq!(c.retry_days, 7);
        assert!(c.completion_tone);
    }

    #[test]
    fn problems_are_described_and_defaults_kept() {
        let (c, problems) = Config::parse("retry_days = soon\ncolour = blue\njust some words\n");
        assert_eq!(c.retry_days, 14);
        assert_eq!(problems.len(), 3);
        assert!(problems[0].contains("not a valid value for retry_days"));
        assert!(problems[1].contains("\"colour\" is not a NoteFetch setting"));
        assert!(problems[2].contains("no equals sign"));
    }

    #[test]
    fn default_output_is_in_documents() {
        assert!(Config::default().output_folder.ends_with("NoteFetch"));
    }
}
