//! The Settings window and the first-sync question, both modal dialogs from
//! `notefetch.rc`. SPEC.md "Settings" and "Sync".
//!
//! Neither touches the worker: each returns what was chosen, and the main
//! window sends it on. The few things Settings must ask the main window while
//! it is open (is something running, sign out now, say this) come in as
//! [`Hooks`].

use std::path::{Path, PathBuf};

use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance, CoTaskMemFree};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::SetFocus;
use windows::Win32::UI::Shell::{
    FOS_FORCEFILESYSTEM, FOS_PATHMUSTEXIST, FOS_PICKFOLDERS, FileOpenDialog, IFileOpenDialog, IShellItem, SIGDN_FILESYSPATH,
    SHCreateItemFromParsingName,
};
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::{HSTRING, PCWSTR, w};

use crate::clock;
use crate::config::{Announcements, Config, Volume};
use crate::sound::{self, Sound};
use crate::sync::FirstSync;
use crate::ui::{dialog_text, item, resource, send, wide};

// Keep in step with notefetch.rc.
const IDD_SETTINGS: u16 = 130;
const IDC_SET_FOLDER: i32 = 1301;
const IDC_SET_BROWSE: i32 = 1302;
const IDC_SET_SYNC_START: i32 = 1303;
const IDC_SET_SYNC_NEW: i32 = 1304;
const IDC_SET_CONFIRM: i32 = 1305;
const IDC_SET_DEL_SKIPS: i32 = 1306;
const IDC_SET_AUDIO: i32 = 1307;
const IDC_SET_TRANSCRIPT: i32 = 1308;
const IDC_SET_SUMMARY: i32 = 1309;
const IDC_SET_OTHER: i32 = 1310;
const IDC_SET_TXT: i32 = 1311;
const IDC_SET_DAYS: i32 = 1312;
const IDC_SET_ANNOUNCE: i32 = 1313;
const IDC_SET_LOG_FILE: i32 = 1314;
const IDC_SET_ACCOUNT: i32 = 1315;
const IDC_SET_SOUNDS: i32 = 1316;
const IDC_SET_VOLUME: i32 = 1317;
const IDD_FIRST: u16 = 140;
const IDC_FIRST_INTRO: i32 = 1401;
const IDC_FIRST_ALL: i32 = 1402;
const IDC_FIRST_DAYS: i32 = 1403;
const IDC_FIRST_NOW: i32 = 1404;
const IDC_FIRST_N: i32 = 1405;

const BM_GETCHECK: u32 = 0x00F0;
const BM_SETCHECK: u32 = 0x00F1;
const BST_CHECKED: usize = 1;
const EN_CHANGE: u32 = 0x0300;

const ANNOUNCE_CHOICES: [(&str, Announcements); 3] = [
    ("All", Announcements::All),
    ("Errors and completion only", Announcements::ErrorsAndCompletion),
    ("Off", Announcements::Off),
];

const VOLUME_CHOICES: [(&str, Volume); 3] = [("Quiet", Volume::Quiet), ("Normal", Volume::Normal), ("Loud", Volume::Loud)];

const CBN_SELCHANGE: u32 = 1;

/// What a first sync offers to go back by default.
const FIRST_SYNC_DAYS: u32 = 30;

const SIGN_IN: &str = "Sign &in to Plaud";
const SIGN_OUT: &str = "Sign o&ut of Plaud";

/// What the Settings window may ask of the main window while it is open.
pub struct Hooks {
    /// Whether a sync, download or move is running.
    pub busy: Box<dyn Fn() -> bool>,
    /// Sign out now. False if it could not start.
    pub sign_out: Box<dyn Fn() -> bool>,
    /// Write a line to the Log and speak it as an error.
    pub say: Box<dyn Fn(&str)>,
}

/// Settings chosen with OK, already saved to the settings file.
pub struct Saved {
    pub config: Config,
    /// The old output folder, when its recordings are to move to the new one.
    pub move_from: Option<PathBuf>,
    /// Sign in once these settings are in use: the account button was pressed
    /// while signed out.
    pub sign_in: bool,
}

struct SettingsState {
    original: Config,
    folder: PathBuf,
    signed_in: bool,
    /// Recordings in the output folder now, for the question about moving them.
    downloaded: usize,
    hooks: Hooks,
    result: Option<Saved>,
}

/// Show Settings. `None` if cancelled, or if OK changed nothing.
pub fn settings(owner: HWND, config: &Config, signed_in: bool, downloaded: usize, hooks: Hooks) -> Option<Saved> {
    let mut state = SettingsState {
        original: config.clone(),
        folder: config.output_folder.clone(),
        signed_in,
        downloaded,
        hooks,
        result: None,
    };
    // SAFETY: a modal window owned by `owner`; `state` outlives it.
    unsafe {
        let instance = GetModuleHandleW(None).ok().map(Into::into);
        DialogBoxParamW(instance, resource(IDD_SETTINGS), Some(owner), Some(settings_proc), LPARAM(&mut state as *mut _ as isize));
    }
    state.result
}

fn set_check(dialog: HWND, id: i32, on: bool) {
    send(item(dialog, id), BM_SETCHECK, on as usize, 0);
}

fn checked(dialog: HWND, id: i32) -> bool {
    send(item(dialog, id), BM_GETCHECK, 0, 0) as usize == BST_CHECKED
}

fn set_text(dialog: HWND, id: i32, text: &str) {
    let t = wide(text);
    // SAFETY: a control of a window this thread owns.
    unsafe {
        let _ = SetDlgItemTextW(dialog, id, PCWSTR(t.as_ptr()));
    }
}

fn focus(dialog: HWND, id: i32) {
    // SAFETY: focus on a control of a window this thread owns.
    unsafe {
        let _ = SetFocus(Some(item(dialog, id)));
    }
}

fn end(dialog: HWND) {
    // SAFETY: ends a modal window this thread owns.
    unsafe {
        let _ = EndDialog(dialog, 0);
    }
}

fn fill_settings(dialog: HWND, s: &SettingsState) {
    let c = &s.original;
    set_text(dialog, IDC_SET_FOLDER, &s.folder.display().to_string());
    set_check(dialog, IDC_SET_SYNC_START, c.sync_on_start);
    set_check(dialog, IDC_SET_SYNC_NEW, c.sync_downloads_new);
    set_check(dialog, IDC_SET_CONFIRM, c.confirm_delete);
    set_check(dialog, IDC_SET_DEL_SKIPS, c.delete_marks_dont_download);
    set_check(dialog, IDC_SET_AUDIO, c.download_audio);
    set_check(dialog, IDC_SET_TRANSCRIPT, c.download_transcript);
    set_check(dialog, IDC_SET_SUMMARY, c.download_summary);
    set_check(dialog, IDC_SET_OTHER, c.download_other);
    set_check(dialog, IDC_SET_TXT, c.notes_as_txt);
    set_text(dialog, IDC_SET_DAYS, &c.retry_days.to_string());
    let combo = item(dialog, IDC_SET_ANNOUNCE);
    for (i, (label, a)) in ANNOUNCE_CHOICES.iter().enumerate() {
        let t = wide(label);
        send(combo, CB_ADDSTRING, 0, t.as_ptr() as isize);
        if *a == c.announcements {
            send(combo, CB_SETCURSEL, i, 0);
        }
    }
    set_check(dialog, IDC_SET_SOUNDS, c.sounds);
    let combo = item(dialog, IDC_SET_VOLUME);
    for (i, (label, v)) in VOLUME_CHOICES.iter().enumerate() {
        let t = wide(label);
        send(combo, CB_ADDSTRING, 0, t.as_ptr() as isize);
        if *v == c.sound_volume {
            send(combo, CB_SETCURSEL, i, 0);
        }
    }
    set_check(dialog, IDC_SET_LOG_FILE, c.log_to_file);
    set_text(dialog, IDC_SET_ACCOUNT, if s.signed_in { SIGN_OUT } else { SIGN_IN });
}

fn chosen_volume(dialog: HWND) -> Volume {
    let i = send(item(dialog, IDC_SET_VOLUME), CB_GETCURSEL, 0, 0).max(0) as usize;
    VOLUME_CHOICES.get(i).map_or(Volume::Normal, |c| c.1)
}

/// The settings as the controls have them, or the reason they cannot be used.
fn read_settings(dialog: HWND, s: &SettingsState) -> Result<Config, (&'static str, i32)> {
    let retry_days = dialog_text(dialog, IDC_SET_DAYS)
        .trim()
        .parse::<u32>()
        .ok()
        .filter(|d| *d <= 3650)
        .ok_or(("Type the number of days to keep checking, such as 14, then choose OK again.", IDC_SET_DAYS))?;
    let chosen = send(item(dialog, IDC_SET_ANNOUNCE), CB_GETCURSEL, 0, 0).max(0) as usize;
    let volume = chosen_volume(dialog);
    Ok(Config {
        output_folder: s.folder.clone(),
        sync_on_start: checked(dialog, IDC_SET_SYNC_START),
        sync_downloads_new: checked(dialog, IDC_SET_SYNC_NEW),
        confirm_delete: checked(dialog, IDC_SET_CONFIRM),
        delete_marks_dont_download: checked(dialog, IDC_SET_DEL_SKIPS),
        download_audio: checked(dialog, IDC_SET_AUDIO),
        download_transcript: checked(dialog, IDC_SET_TRANSCRIPT),
        download_summary: checked(dialog, IDC_SET_SUMMARY),
        download_other: checked(dialog, IDC_SET_OTHER),
        notes_as_txt: checked(dialog, IDC_SET_TXT),
        retry_days,
        announcements: ANNOUNCE_CHOICES.get(chosen).map_or(Announcements::All, |c| c.1),
        sounds: checked(dialog, IDC_SET_SOUNDS),
        sound_volume: volume,
        log_to_file: checked(dialog, IDC_SET_LOG_FILE),
    })
}

/// OK, or the account button while signed out. Returns true if the window
/// should close.
fn accept(dialog: HWND, s: &mut SettingsState, sign_in: bool) -> bool {
    let config = match read_settings(dialog, s) {
        Ok(c) => c,
        Err((why, id)) => {
            (s.hooks.say)(why);
            focus(dialog, id);
            return false;
        }
    };
    let mut move_from = None;
    if config.output_folder != s.original.output_folder {
        if (s.hooks.busy)() {
            (s.hooks.say)(
                "The output folder cannot change while NoteFetch is syncing or downloading. Wait for it to finish, then choose OK again.",
            );
            focus(dialog, IDC_SET_BROWSE);
            return false;
        }
        let old = &s.original.output_folder;
        if s.downloaded > 0 && old.is_dir() {
            let question = format!(
                "Move your {} to the new output folder?\r\n\r\nFrom: {}\r\nTo: {}\r\n\r\nOn another drive this copies them, which can take a while. Choose No to go back and keep the old folder.",
                if s.downloaded == 1 { "downloaded recording".to_string() } else { format!("{} downloaded recordings", s.downloaded) },
                old.display(),
                config.output_folder.display()
            );
            let text = wide(&question);
            if s.original.sounds {
                sound::play(Sound::Attention, s.original.sound_volume);
            }
            // A question the user must answer, which is what a message box is for.
            // SAFETY: a modal box owned by this window.
            let answer = unsafe {
                MessageBoxW(Some(dialog), PCWSTR(text.as_ptr()), w!("Settings"), MB_YESNO | MB_ICONQUESTION | MB_DEFBUTTON2)
            };
            if answer != IDYES {
                s.folder = old.clone();
                set_text(dialog, IDC_SET_FOLDER, &old.display().to_string());
                focus(dialog, IDC_SET_BROWSE);
                return false;
            }
            move_from = Some(old.clone());
        }
    }
    if config == s.original && !sign_in {
        return true;
    }
    if config != s.original
        && let Err(e) = config.save()
    {
        let path = crate::config::config_path();
        (s.hooks.say)(&crate::error::disk("save the settings to", &path, &e).to_string());
        return false;
    }
    s.result = Some(Saved { config, move_from, sign_in });
    true
}

unsafe extern "system" fn settings_proc(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> isize {
    // SAFETY: GWLP_USERDATA holds the SettingsState given at WM_INITDIALOG,
    // which lives until DialogBoxParamW returns.
    let state = || unsafe { (GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut SettingsState).as_mut() };
    match message {
        WM_INITDIALOG => {
            // SAFETY: storing the pointer passed to DialogBoxParamW.
            unsafe {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, lparam.0);
            }
            if let Some(s) = state() {
                fill_settings(hwnd, s);
            }
            // One: Windows puts the focus on the first control, the output folder.
            1
        }
        WM_COMMAND => {
            let id = (wparam.0 & 0xFFFF) as i32;
            let Some(s) = state() else { return 0 };
            let code = ((wparam.0 >> 16) & 0xFFFF) as u32;
            match id {
                // A choice of volume plays Done at that volume, so it can be
                // judged before OK. Something the user did, so it may sound.
                IDC_SET_VOLUME if code == CBN_SELCHANGE => {
                    if checked(hwnd, IDC_SET_SOUNDS) {
                        sound::play(Sound::Done, chosen_volume(hwnd));
                    }
                }
                IDC_SET_BROWSE => {
                    if let Some(folder) = pick_folder(hwnd, &s.folder) {
                        s.folder = folder;
                        set_text(hwnd, IDC_SET_FOLDER, &s.folder.display().to_string());
                        // The box is read in full when it gets the focus, so
                        // the new folder is heard.
                        focus(hwnd, IDC_SET_FOLDER);
                    }
                }
                IDC_SET_ACCOUNT if s.signed_in => {
                    if (s.hooks.sign_out)() {
                        s.signed_in = false;
                        set_text(hwnd, IDC_SET_ACCOUNT, SIGN_IN);
                    }
                }
                IDC_SET_ACCOUNT => {
                    if accept(hwnd, s, true) {
                        end(hwnd);
                    }
                }
                x if x == IDOK.0 => {
                    if accept(hwnd, s, false) {
                        end(hwnd);
                    }
                }
                x if x == IDCANCEL.0 => end(hwnd),
                _ => return 0,
            }
            1
        }
        _ => 0,
    }
}

/// The Windows folder picker. SPEC.md "Settings".
fn pick_folder(owner: HWND, start: &Path) -> Option<PathBuf> {
    // SAFETY: COM calls on this thread, which initialised COM at start-up;
    // the display name is freed once copied.
    unsafe {
        let picker: IFileOpenDialog = CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER).ok()?;
        let options = picker.GetOptions().ok()?;
        picker.SetOptions(options | FOS_PICKFOLDERS | FOS_FORCEFILESYSTEM | FOS_PATHMUSTEXIST).ok()?;
        let _ = picker.SetTitle(w!("Choose the output folder"));
        // Start in the folder holding the output folder, so the output folder
        // is in the list beside its neighbours, or the nearest folder above
        // that exists.
        let above = start.parent().unwrap_or(start);
        if let Some(existing) = above.ancestors().find(|p| p.is_dir())
            && let Ok(folder) = SHCreateItemFromParsingName::<_, _, IShellItem>(&HSTRING::from(existing.as_os_str()), None)
        {
            let _ = picker.SetFolder(&folder);
        }
        // An error here is Cancel.
        picker.Show(Some(owner)).ok()?;
        let chosen = picker.GetResult().ok()?;
        let name = chosen.GetDisplayName(SIGDN_FILESYSPATH).ok()?;
        let path = name.to_string().ok();
        CoTaskMemFree(Some(name.0 as *const _));
        path.map(PathBuf::from)
    }
}

struct FirstState {
    /// (start, duration) in milliseconds of each recording not yet downloaded.
    waiting: Vec<(i64, u64)>,
    /// Write a line to the Log and speak it as an error.
    say: Box<dyn Fn(&str)>,
    result: Option<FirstSync>,
}

/// Ask how much of the Plaud recordings the first sync should download.
/// `None` if cancelled.
pub fn first_sync(owner: HWND, waiting: &[(i64, u64)], say: Box<dyn Fn(&str)>) -> Option<FirstSync> {
    let mut state = FirstState { waiting: waiting.to_vec(), say, result: None };
    // SAFETY: a modal window owned by `owner`; `state` outlives it.
    unsafe {
        let instance = GetModuleHandleW(None).ok().map(Into::into);
        DialogBoxParamW(instance, resource(IDD_FIRST), Some(owner), Some(first_proc), LPARAM(&mut state as *mut _ as isize));
    }
    state.result
}

/// "12 recordings, about 9 hours", for the first-sync choices.
fn amount(recordings: &[(i64, u64)]) -> String {
    if recordings.is_empty() {
        return "no recordings".to_string();
    }
    let ms: u64 = recordings.iter().map(|r| r.1).sum();
    let count = match recordings.len() {
        1 => "1 recording".to_string(),
        n => format!("{n} recordings"),
    };
    let hours = ms as f64 / 3_600_000.0;
    let length = if hours < 1.0 {
        clock::duration_words(ms)
    } else {
        let h = hours.round() as u64;
        format!("about {h} hour{}", if h == 1 { "" } else { "s" })
    };
    format!("{count}, {length}")
}

fn days_choice(days: Option<u32>, waiting: &[(i64, u64)]) -> String {
    match days {
        Some(d) => {
            let from = clock::now_utc().unix_millis() - d as i64 * 86_400_000;
            let within: Vec<(i64, u64)> = waiting.iter().copied().filter(|r| r.0 >= from).collect();
            let span = if d == 1 { "day".to_string() } else { format!("{d} days") };
            format!("&Specified duration, the last {span}: {}", amount(&within))
        }
        None => "&Specified duration: type the number of days below".to_string(),
    }
}

fn typed_days(dialog: HWND) -> Option<u32> {
    dialog_text(dialog, IDC_FIRST_N).trim().parse().ok().filter(|d| (1..=36_500).contains(d))
}

unsafe extern "system" fn first_proc(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> isize {
    // SAFETY: GWLP_USERDATA holds the FirstState given at WM_INITDIALOG, which
    // lives until DialogBoxParamW returns.
    let state = || unsafe { (GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut FirstState).as_mut() };
    match message {
        WM_INITDIALOG => {
            // SAFETY: storing the pointer passed to DialogBoxParamW.
            unsafe {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, lparam.0);
            }
            if let Some(s) = state() {
                set_text(
                    hwnd,
                    IDC_FIRST_INTRO,
                    "This is NoteFetch's first sync. How much of your Plaud audio, transcripts and summaries should it download now? Anything left out stays in the list as Not yet downloaded, ready to download whenever you like.",
                );
                set_text(hwnd, IDC_FIRST_ALL, &format!("&Everything: {}", amount(&s.waiting)));
                set_text(hwnd, IDC_FIRST_N, &FIRST_SYNC_DAYS.to_string());
                set_text(hwnd, IDC_FIRST_DAYS, &days_choice(Some(FIRST_SYNC_DAYS), &s.waiting));
                set_text(hwnd, IDC_FIRST_NOW, "&Nothing old: start from now, and download only new recordings");
            }
            set_check(hwnd, IDC_FIRST_DAYS, true);
            focus(hwnd, IDC_FIRST_DAYS);
            // Zero: the focus is set above, on the choice already made.
            0
        }
        WM_COMMAND => {
            let id = (wparam.0 & 0xFFFF) as i32;
            let code = ((wparam.0 >> 16) & 0xFFFF) as u32;
            let Some(s) = state() else { return 0 };
            match id {
                IDC_FIRST_N if code == EN_CHANGE => {
                    // Typing a number of days chooses that option, and its
                    // text follows the number.
                    set_text(hwnd, IDC_FIRST_DAYS, &days_choice(typed_days(hwnd), &s.waiting));
                    for (choice, on) in [(IDC_FIRST_ALL, false), (IDC_FIRST_DAYS, true), (IDC_FIRST_NOW, false)] {
                        set_check(hwnd, choice, on);
                    }
                }
                x if x == IDOK.0 => {
                    let choice = if checked(hwnd, IDC_FIRST_ALL) {
                        Some(FirstSync::Everything)
                    } else if checked(hwnd, IDC_FIRST_NOW) {
                        Some(FirstSync::FromNow)
                    } else {
                        typed_days(hwnd).map(FirstSync::LastDays)
                    };
                    match choice {
                        Some(c) => {
                            s.result = Some(c);
                            end(hwnd);
                        }
                        // No usable number: say so, and back to the box for one.
                        None => {
                            (s.say)("Type the number of days to go back, such as 30, then choose OK again.");
                            focus(hwnd, IDC_FIRST_N);
                        }
                    }
                }
                x if x == IDCANCEL.0 => end(hwnd),
                _ => return 0,
            }
            1
        }
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amounts_are_in_words() {
        assert_eq!(amount(&[]), "no recordings");
        assert_eq!(amount(&[(0, 20 * 60_000)]), "1 recording, 20 minutes");
        assert_eq!(amount(&[(0, 3_600_000), (0, 5_400_000)]), "2 recordings, about 3 hours");
        assert_eq!(amount(&[(0, 3_000_000), (0, 1_000_000)]), "2 recordings, about 1 hour");
    }

    #[test]
    fn the_days_choice_counts_only_recordings_inside_it() {
        let now = clock::now_utc().unix_millis();
        let waiting = [(now - 86_400_000, 60_000), (now - 40 * 86_400_000, 60_000)];
        assert_eq!(days_choice(Some(30), &waiting), "&Specified duration, the last 30 days: 1 recording, 1 minute");
        assert_eq!(days_choice(Some(1), &[]), "&Specified duration, the last day: no recordings");
        assert!(days_choice(None, &waiting).contains("type the number of days"));
    }
}
