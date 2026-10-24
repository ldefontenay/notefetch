//! The main window: a modeless dialog from `notefetch.rc`. SPEC.md "Main
//! window" and "Keyboard".
//!
//! This thread owns every window and never waits on the network or the disk:
//! commands go to the worker, and results come back as events (see
//! `worker.rs`). Keyboard behaviour comes from Windows: `IsDialogMessageW`
//! gives Tab, mnemonics and Escape, the accelerator table gives the
//! window-wide shortcuts, and the list's own keys are handled by the list so
//! they never fire while typing in the filter box.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{GlobalFree, HANDLE, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::ClientToScreen;
use windows::Win32::System::DataExchange::{CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData};
use windows::Win32::System::Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalUnlock};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::{
    ICC_LISTVIEW_CLASSES, INITCOMMONCONTROLSEX, InitCommonControlsEx, LIST_VIEW_ITEM_STATE_FLAGS, LVCF_SUBITEM,
    LVCF_TEXT, LVCF_WIDTH, LVCOLUMNW, LVIF_PARAM, LVIF_TEXT, LVIS_FOCUSED, LVIS_SELECTED, LVITEMW, LVM_DELETEALLITEMS,
    LVM_ENSUREVISIBLE, LVM_GETITEMCOUNT, LVM_GETNEXTITEM, LVM_INSERTCOLUMNW, LVM_INSERTITEMW,
    LVM_EDITLABELW, LVM_GETEDITCONTROL, LVM_GETITEMRECT, LVM_SETEXTENDEDLISTVIEWSTYLE, LVM_SETITEMSTATE, LVM_SETITEMTEXTW,
    LVN_BEGINLABELEDITW, LVN_ENDLABELEDITW, LVN_ITEMCHANGED, LVNI_FOCUSED, LVNI_SELECTED, LVIR_LABEL, LVS_EX_FULLROWSELECT,
    NMHDR, NMLISTVIEW, NMLVDISPINFOW,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetFocus, GetKeyState, SetFocus, VK_CONTROL, VK_DELETE, VK_ESCAPE, VK_F2, VK_RETURN, VK_SHIFT,
};
use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, CoInitializeEx};
use windows::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass, ShellExecuteW};
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::{PCWSTR, PWSTR, w};

use crate::announce;
use crate::cancel::Cancel;
use crate::config::{Announcements, Config};
use crate::library;
use crate::naming;
use crate::sound::{self, Sound};
use crate::store::State;
use crate::worker::{Activity, Command, Importance, Row, Snapshot, UiEvent, WM_APP_EVENTS, Worker};

// Keep in step with notefetch.rc.
const IDD_MAIN: u16 = 100;
const IDR_ACCEL: u16 = 201;
const IDC_SHOW: i32 = 1001;
const IDC_FOLDER: i32 = 1002;
const IDC_FILTER: i32 = 1003;
const IDC_LIST: i32 = 1004;
const IDC_SYNC: i32 = 1005;
const IDC_DOWNLOAD: i32 = 1006;
const IDC_OPEN_OUTPUT: i32 = 1007;
const IDC_LOG: i32 = 1008;
const IDM_EXIT: i32 = 2001;
const IDM_SIGN_IN: i32 = 2002;
const IDM_SIGN_OUT: i32 = 2003;
const IDM_SETTINGS: i32 = 2004;
const IDM_DOWNLOAD: i32 = 2010;
const IDM_SELECT_ALL: i32 = 2011;
const IDM_OPEN_RECORDING: i32 = 2012;
const IDM_SYNC: i32 = 2020;
const IDM_REFRESH: i32 = 2021;
const IDM_CANCEL: i32 = 2022;
const IDM_FIND: i32 = 2023;
const IDM_OPEN_OUTPUT: i32 = 2024;
const IDM_TEST_SPEECH: i32 = 2030;
/// Enter in the list: the focused recording's default action.
const IDM_DEFAULT: i32 = 2040;
const IDM_PLAY: i32 = 2041;
const IDM_TRANSCRIPT: i32 = 2042;
const IDM_SUMMARY: i32 = 2043;
const IDM_COPY_PATH: i32 = 2044;
const IDM_RENAME: i32 = 2045;
const IDM_MOVE: i32 = 2046;
const IDM_DELETE: i32 = 2047;
const IDM_SKIP: i32 = 2048;
const IDM_NEW_FOLDER: i32 = 2060;
const IDM_RENAME_FOLDER: i32 = 2061;
const IDM_DELETE_FOLDER: i32 = 2062;
const IDD_MOVE: u16 = 110;
const IDC_MOVE_LIST: i32 = 1101;
const IDC_MOVE_NEW: i32 = 1102;
const IDD_NAME: u16 = 120;
const IDC_NAME_EDIT: i32 = 1202;

/// Menu bar positions, for setting up a menu as it opens.
const MENU_FILE: i32 = 0;
const MENU_RECORDING: i32 = 1;
const MENU_FOLDER: i32 = 2;
/// Every command in the Recording menu that depends on the focused recording.
const RECORDING_COMMANDS: [i32; 10] =
    [IDM_PLAY, IDM_TRANSCRIPT, IDM_SUMMARY, IDM_OPEN_RECORDING, IDM_COPY_PATH, IDM_DOWNLOAD, IDM_SKIP, IDM_RENAME, IDM_MOVE, IDM_DELETE];

// Edit control messages, from winuser.h.
const EM_GETSEL: u32 = 0x00B0;
const EM_SETSEL: u32 = 0x00B1;
const EM_SCROLLCARET: u32 = 0x00B7;
const EM_REPLACESEL: u32 = 0x00C2;
const EM_SETLIMITTEXT: u32 = 0x00C5;

/// Posted to the window to offer signing in, outside the event handling,
/// because the question is a modal box. wparam is 1 if the sign-in expired.
const WM_APP_ASK_SIGN_IN: u32 = WM_APP + 11;
/// Posted by the list when its context menu is asked for, with the
/// WM_CONTEXTMENU coordinates as lparam: -1 for the keyboard.
const WM_APP_CONTEXT: u32 = WM_APP + 12;
/// Posted when a first sync has stopped to ask how much history to download.
const WM_APP_ASK_FIRST: u32 = WM_APP + 13;

const FILTER_TIMER: usize = 1;
const SELECT_TIMER: usize = 2;
/// "All 71 recordings selected." waits this long after Ctrl+A. JAWS reads
/// out every newly selected row, and in milestone 3 the announcement raised
/// at once was not heard. `[untested]` whether waiting lets it through.
const SELECT_PAUSE_MS: u32 = 300;
/// Filtering waits this long after the last key, so each letter does not
/// rebuild the list and make the screen reader chatter.
const FILTER_PAUSE_MS: u32 = 400;
/// Progress is spoken at most this often. Every item is logged regardless.
const PROGRESS_EVERY: Duration = Duration::from_secs(5);
/// The Log keeps roughly this much text, dropping the oldest at the bottom.
const LOG_MAX_CHARS: usize = 60_000;

/// The "Show" choices, in order. SPEC.md "Main window".
const SHOW: [&str; 5] = ["All recordings", "Downloaded", "Not yet downloaded", "Incomplete", "Deleted"];

/// Columns: title and share of the width, in hundredths.
const COLUMNS: [(&str, i32); 5] = [("Name", 34), ("Folder", 16), ("Date", 22), ("Duration", 14), ("Status", 14)];

struct Ui {
    dialog: HWND,
    list: HWND,
    worker: Worker,
    snapshot: Snapshot,
    /// Indices into `snapshot.rows`, in list order.
    visible: Vec<usize>,
    /// The recording the user last moved to in the list, by Plaud ID. Kept
    /// here rather than read back from the list, because rebuilding the list
    /// moves the list's own focus about before it can be restored.
    focus_id: Option<String>,
    busy: bool,
    /// What the worker is doing while busy.
    activity: Activity,
    cancel: Option<Cancel>,
    /// The announcement waiting for SELECT_TIMER.
    pending_select: Option<String>,
    /// A sign-in question is already showing or on its way.
    asking: bool,
    /// When the sign-in Plaud refused was saved. A sign-in saved since
    /// replaces it, so the File menu offers Sign in again only until then.
    expired: Option<std::time::SystemTime>,
    /// The sound for when the running command finishes: Done or Finished for
    /// one the user started, Refused once anything has gone wrong, and none
    /// for the sync on start that went well or a command cancelled.
    sound: Option<Sound>,
    /// Recordings in the list when the running command started, by Plaud ID.
    /// They stay in the list until it finishes, even if they no longer match
    /// the Show choice, so "Not yet downloaded" does not shrink under the
    /// user during a download (Laurent, round 2).
    kept: Vec<String>,
    config: Config,
    last_progress: Option<Instant>,
    /// The Folder box's choice to make once the next list arrives, after a
    /// folder is renamed.
    want_folder: Option<String>,
    /// The recordings a first sync is waiting to ask about, as (start,
    /// duration). The question is asked once the sync has finished.
    first_sync: Option<Vec<(i64, u64)>>,
}

thread_local! {
    static UI: RefCell<Option<Ui>> = const { RefCell::new(None) };
    static QUEUE: Arc<Mutex<VecDeque<UiEvent>>> = Arc::new(Mutex::new(VecDeque::new()));
}

/// Run something with the window's state. Windows sometimes calls back into
/// the dialog while that state is already in use (for example a list
/// notification sent while the list is being filled); those calls are
/// skipped, which is what they would do anyway.
fn with_ui<R>(f: impl FnOnce(&mut Ui) -> R) -> Option<R> {
    UI.with(|cell| cell.try_borrow_mut().ok().and_then(|mut ui| ui.as_mut().map(f)))
}

pub(crate) fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

pub(crate) fn resource(id: u16) -> PCWSTR {
    PCWSTR(id as usize as *const u16)
}

pub(crate) fn item(dialog: HWND, id: i32) -> HWND {
    // SAFETY: a plain lookup on a window this thread owns.
    unsafe { GetDlgItem(Some(dialog), id).unwrap_or_default() }
}

pub(crate) fn send(hwnd: HWND, message: u32, wparam: usize, lparam: isize) -> isize {
    // SAFETY: messages to controls this thread owns, with arguments of the
    // types each message documents.
    unsafe { SendMessageW(hwnd, message, Some(WPARAM(wparam)), Some(LPARAM(lparam))).0 }
}

/// Show the main window and pump messages until it closes.
pub fn run() -> i32 {
    // SAFETY: standard Win32 start-up, all on this thread.
    unsafe {
        // The shell's Open With and file operations expect COM on this thread.
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE);
        let icc = INITCOMMONCONTROLSEX { dwSize: size_of::<INITCOMMONCONTROLSEX>() as u32, dwICC: ICC_LISTVIEW_CLASSES };
        let _ = InitCommonControlsEx(&icc);
        let Ok(instance) = GetModuleHandleW(None) else { return 2 };
        let Ok(dialog) = CreateDialogParamW(Some(instance.into()), resource(IDD_MAIN), None, Some(dialog_proc), LPARAM(0)) else {
            return 2;
        };
        let accel = LoadAcceleratorsW(Some(instance.into()), resource(IDR_ACCEL)).ok();
        let list = item(dialog, IDC_LIST);
        let _ = ShowWindow(dialog, SW_SHOW);
        let _ = SetForegroundWindow(dialog);

        let mut message = MSG::default();
        loop {
            // Zero is WM_QUIT and -1 an error; both end the loop.
            if GetMessageW(&mut message, None, 0, 0).0 <= 0 {
                break;
            }
            // Accelerators first, then the dialog's own keyboard handling.
            // None while a name is being edited in the list, so Ctrl+S or F5
            // cannot set something running in the middle of typing.
            let editing = send(list, LVM_GETEDITCONTROL, 0, 0) != 0;
            if let Some(a) = accel
                && !editing
                && TranslateAcceleratorW(dialog, a, &message) != 0
            {
                continue;
            }
            if IsDialogMessageW(dialog, &message).as_bool() {
                continue;
            }
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
    0
}

unsafe extern "system" fn dialog_proc(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> isize {
    match message {
        WM_INITDIALOG => {
            init(hwnd);
            // Zero: we set the focus ourselves, on the list.
            0
        }
        WM_APP_EVENTS => {
            drain();
            1
        }
        WM_APP_ASK_SIGN_IN => {
            ask_sign_in(hwnd, wparam.0 == 1);
            1
        }
        WM_APP_CONTEXT => {
            context_menu(hwnd, lparam.0);
            1
        }
        WM_APP_ASK_FIRST => {
            ask_first_sync(hwnd);
            1
        }
        WM_INITMENUPOPUP => {
            init_menu(hwnd, HMENU(wparam.0 as *mut _));
            0
        }
        WM_COMMAND => {
            let id = (wparam.0 & 0xFFFF) as i32;
            let code = ((wparam.0 >> 16) & 0xFFFF) as u32;
            command(hwnd, id, code);
            1
        }
        WM_NOTIFY => {
            // SAFETY: WM_NOTIFY's lparam points at an NMHDR, and for a list
            // view LVN_ITEMCHANGED at an NMLISTVIEW that begins with one.
            unsafe {
                let header = &*(lparam.0 as *const NMHDR);
                if header.idFrom == IDC_LIST as usize && header.code == LVN_BEGINLABELEDITW {
                    let info = &*(lparam.0 as *const NMLVDISPINFOW);
                    if !begin_rename(header.hwndFrom, info.item.iItem) {
                        // Refuse the edit: a dialog's answer to a notification
                        // goes in DWLP_MSGRESULT.
                        SetWindowLongPtrW(hwnd, WINDOW_LONG_PTR_INDEX(0), 1);
                        return 1;
                    }
                    return 0;
                }
                if header.idFrom == IDC_LIST as usize && header.code == LVN_ENDLABELEDITW {
                    let info = &*(lparam.0 as *const NMLVDISPINFOW);
                    if !info.item.pszText.is_null() {
                        let text = info.item.pszText.to_string().unwrap_or_default();
                        let index = info.item.iItem;
                        with_ui(|ui| end_rename(ui, index, &text));
                    }
                    // Leave the list's text alone: the new name arrives with
                    // the next list from the worker, once the disk agrees.
                    return 0;
                }
                if header.idFrom == IDC_LIST as usize && header.code == LVN_ITEMCHANGED {
                    let change = &*(lparam.0 as *const NMLISTVIEW);
                    let gained = change.uNewState & LVIS_FOCUSED.0 != 0 && change.uOldState & LVIS_FOCUSED.0 == 0;
                    if gained && change.iItem >= 0 {
                        // Skipped while the list is being rebuilt, because the
                        // window's state is in use then: exactly what is wanted.
                        with_ui(|ui| {
                            if let Some(&r) = ui.visible.get(change.iItem as usize) {
                                ui.focus_id = Some(ui.snapshot.rows[r].id.clone());
                            }
                        });
                    }
                }
            }
            0
        }
        WM_TIMER if wparam.0 == FILTER_TIMER => {
            // SAFETY: the timer belongs to this window.
            unsafe {
                let _ = KillTimer(Some(hwnd), FILTER_TIMER);
            }
            with_ui(|ui| {
                ui.kept.clear();
                apply_filter(ui);
            });
            1
        }
        WM_TIMER if wparam.0 == SELECT_TIMER => {
            // SAFETY: the timer belongs to this window.
            unsafe {
                let _ = KillTimer(Some(hwnd), SELECT_TIMER);
            }
            with_ui(|ui| {
                if let Some(text) = ui.pending_select.take() {
                    say(ui, &text, Importance::Completion);
                }
            });
            1
        }
        WM_CLOSE => {
            close(hwnd);
            1
        }
        WM_DESTROY => {
            // SAFETY: ends this thread's message loop.
            unsafe { PostQuitMessage(0) };
            1
        }
        _ => {
            let _ = lparam;
            0
        }
    }
}

fn init(dialog: HWND) {
    let list = item(dialog, IDC_LIST);
    send(list, LVM_SETEXTENDEDLISTVIEWSTYLE, LVS_EX_FULLROWSELECT as usize, LVS_EX_FULLROWSELECT as isize);
    let mut rect = RECT::default();
    // SAFETY: rect is a live structure.
    unsafe {
        let _ = GetClientRect(list, &mut rect);
    }
    let width = (rect.right - rect.left - 24).max(300);
    for (i, (title, share)) in COLUMNS.iter().enumerate() {
        let mut text = wide(title);
        let col = LVCOLUMNW {
            mask: LVCF_TEXT | LVCF_WIDTH | LVCF_SUBITEM,
            cx: width * share / 100,
            pszText: PWSTR(text.as_mut_ptr()),
            iSubItem: i as i32,
            ..Default::default()
        };
        send(list, LVM_INSERTCOLUMNW, i, &col as *const _ as isize);
    }
    // SAFETY: the subclass procedure lives for the whole program.
    unsafe {
        let _ = SetWindowSubclass(list, Some(list_proc), 1, 0);
    }

    let show = item(dialog, IDC_SHOW);
    for s in SHOW {
        let t = wide(s);
        send(show, CB_ADDSTRING, 0, t.as_ptr() as isize);
    }
    send(show, CB_SETCURSEL, 0, 0);
    fill_folders(dialog, &[], None);

    let log = item(dialog, IDC_LOG);
    // Lift the edit control's default limit of about 30,000 characters.
    send(log, EM_SETLIMITTEXT, 0, 0);
    // SAFETY: the subclass procedure lives for the whole program.
    unsafe {
        let _ = SetWindowSubclass(log, Some(log_proc), 2, 0);
    }

    let queue = QUEUE.with(Arc::clone);
    let startup = Cancel::new();
    let worker = Worker::start(dialog, queue, startup.clone());
    let (config, _) = Config::load();
    UI.with(|cell| {
        *cell.borrow_mut() = Some(Ui {
            dialog,
            list,
            worker,
            snapshot: Snapshot::default(),
            visible: Vec::new(),
            focus_id: None,
            // The worker starts by syncing, if that setting is on.
            busy: true,
            activity: Activity::Connecting,
            cancel: Some(startup),
            pending_select: None,
            asking: false,
            expired: None,
            sound: None,
            kept: Vec::new(),
            config,
            last_progress: None,
            want_folder: None,
            first_sync: None,
        });
    });
    // SAFETY: focus on a control of this window.
    unsafe {
        let _ = SetFocus(Some(list));
    }
}

fn fill_folders(dialog: HWND, folders: &[String], keep: Option<String>) {
    let combo = item(dialog, IDC_FOLDER);
    send(combo, CB_RESETCONTENT, 0, 0);
    let mut entries = vec!["All folders".to_string(), "Uncategorised".to_string()];
    entries.extend(folders.iter().cloned());
    let mut chosen = 0;
    for (i, e) in entries.iter().enumerate() {
        let t = wide(e);
        send(combo, CB_ADDSTRING, 0, t.as_ptr() as isize);
        if keep.as_deref() == Some(e.as_str()) {
            chosen = i;
        }
    }
    send(combo, CB_SETCURSEL, chosen, 0);
}

fn combo_text(dialog: HWND, id: i32) -> String {
    let combo = item(dialog, id);
    let i = send(combo, CB_GETCURSEL, 0, 0);
    if i < 0 {
        return String::new();
    }
    let len = send(combo, CB_GETLBTEXTLEN, i as usize, 0).max(0) as usize;
    let mut buf = vec![0u16; len + 1];
    send(combo, CB_GETLBTEXT, i as usize, buf.as_mut_ptr() as isize);
    String::from_utf16_lossy(&buf[..len])
}

fn filter_text(dialog: HWND) -> String {
    let mut buf = [0u16; 256];
    // SAFETY: buf is a live buffer of the length given.
    let len = unsafe { GetDlgItemTextW(dialog, IDC_FILTER, &mut buf) } as usize;
    String::from_utf16_lossy(&buf[..len]).trim().to_lowercase()
}

fn command(hwnd: HWND, id: i32, code: u32) {
    match id {
        IDC_FILTER if code == EN_CHANGE => {
            // SAFETY: a timer on this window.
            unsafe {
                SetTimer(Some(hwnd), FILTER_TIMER, FILTER_PAUSE_MS, None);
            }
        }
        IDC_SHOW | IDC_FOLDER if code == CBN_SELCHANGE => {
            with_ui(|ui| {
                // A new view is the user's choice; keep nothing from the old one.
                ui.kept.clear();
                apply_filter(ui);
            });
        }
        IDC_LOG if code == EN_SETFOCUS => caret_to_start(item(hwnd, IDC_LOG)),
        IDC_SYNC | IDM_SYNC => {
            with_ui(|ui| start(ui, Command::Sync, "A sync"));
        }
        IDM_REFRESH => {
            with_ui(|ui| start(ui, Command::Refresh, "A refresh"));
        }
        IDC_DOWNLOAD | IDM_DOWNLOAD => {
            with_ui(download_selected);
        }
        IDC_OPEN_OUTPUT | IDM_OPEN_OUTPUT => {
            with_ui(open_output);
        }
        IDM_OPEN_RECORDING => {
            with_ui(open_recording);
        }
        IDM_DEFAULT => {
            // Play a downloaded recording; download anything else. SPEC.md
            // "Keyboard": Enter carries out the context menu's default item.
            let action = with_ui(|ui| focused(ui).map(|r| default_command(&r))).flatten();
            if let Some(id) = action {
                command(hwnd, id, 0);
            }
        }
        IDM_PLAY => {
            with_ui(|ui| open_part(ui, FilePart::Audio));
        }
        IDM_TRANSCRIPT => {
            with_ui(|ui| open_part(ui, FilePart::Transcript));
        }
        IDM_SUMMARY => {
            with_ui(|ui| open_part(ui, FilePart::Summary));
        }
        IDM_COPY_PATH => {
            with_ui(copy_audio_path);
        }
        IDM_RENAME => rename_focused(hwnd),
        IDM_DELETE => delete_selected(hwnd),
        IDM_SKIP => {
            with_ui(skip_selected);
        }
        IDM_MOVE => move_selected(hwnd),
        IDM_NEW_FOLDER => new_folder(hwnd),
        IDM_RENAME_FOLDER => rename_folder(hwnd),
        IDM_DELETE_FOLDER => delete_folder(hwnd),
        IDM_SELECT_ALL => {
            with_ui(|ui| {
                select_all(ui);
                // SAFETY: focus on a control of this window.
                unsafe {
                    let _ = SetFocus(Some(ui.list));
                }
            });
        }
        IDM_FIND => {
            // SAFETY: focus on a control of this window.
            unsafe {
                let _ = SetFocus(Some(item(hwnd, IDC_FILTER)));
            }
        }
        // Escape arrives as IDCANCEL. It cancels; it never closes the window.
        IDM_CANCEL | 2 => {
            with_ui(cancel);
        }
        IDM_SIGN_IN => {
            with_ui(|ui| start(ui, Command::SignIn, "Signing in"));
        }
        IDM_SIGN_OUT => {
            with_ui(|ui| start(ui, Command::SignOut, "Signing out"));
        }
        IDM_SETTINGS => open_settings(hwnd),
        IDM_EXIT => close(hwnd),
        IDM_TEST_SPEECH => {
            with_ui(|ui| {
                say(ui, "This is a test announcement from NoteFetch. If you heard this, announcements work.", Importance::Completion)
            });
        }
        _ => {}
    }
}

fn start(ui: &mut Ui, command: Command, what: &str) {
    if ui.busy {
        say(
            ui,
            &format!("{what} cannot start while something else is running. Wait for it to finish, or press Escape to cancel it."),
            Importance::Error,
        );
        return;
    }
    let cancel = Cancel::new();
    ui.cancel = Some(cancel.clone());
    ui.busy = true;
    ui.sound = Some(if command.is_local() { Sound::Done } else { Sound::Finished });
    ui.last_progress = None;
    ui.kept = ui.visible.iter().map(|&r| ui.snapshot.rows[r].id.clone()).collect();
    ui.worker.send(command, cancel);
}

fn cancel(ui: &mut Ui) {
    match (&ui.cancel, ui.busy) {
        _ if ui.busy && ui.activity == Activity::Moving => say(
            ui,
            "Moving recordings to the new output folder cannot be stopped part way, so that none are left behind. It carries on.",
            Importance::Completion,
        ),
        (Some(c), true) => {
            c.cancel();
            ui.sound = None;
            say(ui, "Cancelling.", Importance::Completion);
        }
        _ => say(ui, "Nothing is running, so there is nothing to cancel.", Importance::Completion),
    }
}

/// Ask how much history the first sync should download, then run it.
fn ask_first_sync(hwnd: HWND) {
    let Some(waiting) = with_ui(|ui| ui.first_sync.take()).flatten() else { return };
    let say_error = Box::new(|text: &str| {
        with_ui(|ui| say(ui, text, Importance::Error));
    });
    attention();
    let choice = crate::dialogs::first_sync(hwnd, &waiting, say_error);
    with_ui(|ui| match choice {
        Some(c) => start(ui, Command::FirstSync(c), "The first sync"),
        None => say(
            ui,
            "Nothing was downloaded. NoteFetch will ask again at the next sync. Press Ctrl+S when you are ready.",
            Importance::Completion,
        ),
    });
    focus_list(hwnd);
}

/// Settings (Ctrl+Comma). SPEC.md "Settings".
fn open_settings(hwnd: HWND) {
    let Some((config, downloaded)) =
        with_ui(|ui| (ui.config.clone(), ui.snapshot.rows.iter().filter(|r| r.location.is_some()).count()))
    else {
        return;
    };
    let hooks = crate::dialogs::Hooks {
        busy: Box::new(|| with_ui(|ui| ui.busy).unwrap_or(true)),
        sign_out: Box::new(|| {
            with_ui(|ui| {
                let free = !ui.busy;
                start(ui, Command::SignOut, "Signing out");
                free
            })
            .unwrap_or(false)
        }),
        say: Box::new(|text| {
            with_ui(|ui| say(ui, text, Importance::Error));
        }),
    };
    let saved = crate::dialogs::settings(hwnd, &config, with_ui(signed_in).unwrap_or(false), downloaded, hooks);
    with_ui(|ui| {
        let Some(saved) = saved else {
            log(ui, "Settings were not changed.");
            return;
        };
        let changed = saved.config != ui.config;
        ui.config = saved.config.clone();
        let moving = saved.move_from.is_some();
        if changed {
            log(ui, "Settings saved.");
        }
        match saved.move_from {
            Some(old) => start(ui, Command::Settings { config: saved.config, move_from: Some(old) }, "Moving recordings"),
            None if changed => {
                ui.worker.send(Command::Settings { config: saved.config, move_from: None }, Cancel::new());
                if !saved.sign_in {
                    speak(ui, "Settings saved.", Importance::Completion);
                }
            }
            None => {}
        }
        if saved.sign_in {
            if moving {
                log(ui, "Sign in once the recordings have moved, from the File menu or from Settings.");
            } else {
                start(ui, Command::SignIn, "Signing in");
            }
        }
    });
    focus_list(hwnd);
}

/// Offer to sign in, after the worker found the sign-in missing or expired.
fn ask_sign_in(hwnd: HWND, expired: bool) {
    let question = if expired {
        "Your Plaud sign-in has expired. Sign in again now?\r\n\r\nNoteFetch will open Plaud's sign-in page in your web browser."
    } else {
        "You are not signed in to Plaud. Sign in now?\r\n\r\nNoteFetch will open Plaud's sign-in page in your web browser."
    };
    let text = wide(question);
    attention();
    // A question the user must answer, which is what a message box is for.
    // SAFETY: a modal box owned by this window.
    let answer = unsafe { MessageBoxW(Some(hwnd), PCWSTR(text.as_ptr()), w!("NoteFetch"), MB_YESNO | MB_ICONQUESTION | MB_DEFBUTTON1) };
    with_ui(|ui| {
        ui.asking = false;
        if answer == IDYES {
            start(ui, Command::SignIn, "Signing in");
        } else {
            log(ui, "Not signed in. You can sign in at any time from the File menu or from Settings.");
        }
    });
}

fn close(hwnd: HWND) {
    let moving = with_ui(|ui| {
        let moving = ui.busy && ui.activity == Activity::Moving;
        if moving {
            say(
                ui,
                "NoteFetch is still moving your recordings to the new output folder. Wait for it to finish, then close NoteFetch.",
                Importance::Error,
            );
        }
        moving
    });
    if moving != Some(false) {
        return;
    }
    // Only work that writes files is worth a question; anything else just stops.
    let question = with_ui(|ui| match (ui.busy, ui.activity) {
        (true, Activity::Downloading) => {
            Some("NoteFetch is still downloading. Stop the download and close NoteFetch?\r\n\r\nAnything already downloaded is kept.")
        }
        (true, Activity::Syncing) => {
            Some("NoteFetch is still syncing with Plaud. Stop the sync and close NoteFetch?\r\n\r\nAnything already downloaded is kept.")
        }
        _ => None,
    })
    .flatten();
    if let Some(question) = question {
        let text = wide(question);
        // A question the user must answer, which is what a message box is for.
        // SAFETY: a modal box owned by this window.
        let answer = unsafe { MessageBoxW(Some(hwnd), PCWSTR(text.as_ptr()), w!("NoteFetch"), MB_YESNO | MB_ICONQUESTION | MB_DEFBUTTON2) };
        if answer != IDYES {
            return;
        }
    }
    with_ui(|ui| {
        if let Some(c) = &ui.cancel {
            c.cancel();
        }
    });
    // SAFETY: destroying this thread's own window.
    unsafe {
        let _ = DestroyWindow(hwnd);
    }
}

fn drain() {
    let events: Vec<UiEvent> = QUEUE.with(|q| q.lock().map(|mut q| q.drain(..).collect()).unwrap_or_default());
    with_ui(|ui| {
        for e in events {
            match e {
                UiEvent::Log(line) => log(ui, &line),
                UiEvent::Announce(line, importance) => {
                    log(ui, &line);
                    speak(ui, &line, importance);
                    // Something went wrong in the running command: Refused
                    // once at its end, however many things did, in place of
                    // Finished.
                    if importance == Importance::Error {
                        if ui.busy {
                            ui.sound = Some(Sound::Refused);
                        } else {
                            play(ui, Sound::Refused);
                        }
                    }
                }
                UiEvent::Speak(line, importance) => speak(ui, &line, importance),
                UiEvent::Progress { done, total } => {
                    let due = ui.last_progress.is_none_or(|t| t.elapsed() >= PROGRESS_EVERY);
                    if due && done > 0 && done < total {
                        ui.last_progress = Some(Instant::now());
                        // A sync also counts recordings only checked again, so
                        // "Downloaded 1 of 6" was wrong when nothing was (round 3).
                        let verb = match ui.activity {
                            Activity::Downloading => "Downloaded",
                            Activity::Moving => "Moved",
                            _ => "Fetched",
                        };
                        say(ui, &format!("{verb} {done} of {total}."), Importance::Routine);
                    }
                }
                UiEvent::Snapshot(s) => take_snapshot(ui, s),
                UiEvent::Activity(a) => ui.activity = a,
                UiEvent::SignInNeeded(expired) => {
                    if expired {
                        ui.expired = crate::auth::saved_at();
                    }
                    if !ui.asking {
                        ui.asking = true;
                        // SAFETY: posting to this thread's own window.
                        unsafe {
                            let _ = PostMessageW(Some(ui.dialog), WM_APP_ASK_SIGN_IN, WPARAM(expired as usize), LPARAM(0));
                        }
                    }
                }
                UiEvent::FirstSyncNeeded(waiting) => ui.first_sync = Some(waiting),
                UiEvent::Sound(s) => play(ui, s),
                UiEvent::Finished => {
                    ui.busy = false;
                    ui.cancel = None;
                    if let Some(s) = ui.sound.take() {
                        play(ui, s);
                    }
                    if !ui.kept.is_empty() {
                        ui.kept.clear();
                        apply_filter(ui);
                    }
                    if ui.first_sync.is_some() {
                        // SAFETY: posting to this thread's own window.
                        unsafe {
                            let _ = PostMessageW(Some(ui.dialog), WM_APP_ASK_FIRST, WPARAM(0), LPARAM(0));
                        }
                    }
                }
            }
        }
    });
}

/// Write to the Log, and speak it if the announcement setting allows.
fn say(ui: &mut Ui, text: &str, importance: Importance) {
    log(ui, text);
    speak(ui, text, importance);
    // Called on this thread only for what the user just pressed, so a
    // refusal sounds at once, and leaves a running command's sound alone.
    if importance == Importance::Error {
        play(ui, Sound::Refused);
    }
}

/// Play one of NoteFetch's sounds, if the setting allows.
fn play(ui: &Ui, s: Sound) {
    if ui.config.sounds {
        sound::play(s, ui.config.sound_volume);
    }
}

/// The Attention sound, just before a question NoteFetch raises by itself:
/// the sign-in and the first sync. Not before one the user has just asked
/// for, such as Delete: "A bit too much, not necessary" (Laurent, milestone 6
/// sounds round).
fn attention() {
    with_ui(|ui| play(ui, Sound::Attention));
}

/// Speak a line already in the Log, if the announcement setting allows.
fn speak(ui: &mut Ui, text: &str, importance: Importance) {
    let wanted = match ui.config.announcements {
        Announcements::All => true,
        Announcements::ErrorsAndCompletion => importance != Importance::Routine,
        Announcements::Off => false,
    };
    if wanted {
        announce::say(ui.dialog, text);
    }
}

/// Add a line to the top of the Log. Newest first, as Laurent chose.
fn log(ui: &mut Ui, text: &str) {
    let edit = item(ui.dialog, IDC_LOG);
    let line: String = text.replace("\r\n", "\n").replace('\n', "\r\n") + "\r\n";
    let line_w = wide(&line);
    let inserted = line.encode_utf16().count();
    // SAFETY: focus query on this thread.
    let focused = unsafe { GetFocus() } == edit;
    let (mut start, mut end) = (0u32, 0u32);
    send(edit, EM_GETSEL, &mut start as *mut u32 as usize, &mut end as *mut u32 as isize);
    send(edit, EM_SETSEL, 0, 0);
    send(edit, EM_REPLACESEL, 0, line_w.as_ptr() as isize);
    if focused {
        // Someone is reading the Log: keep their place in the text.
        send(edit, EM_SETSEL, start as usize + inserted, end as isize + inserted as isize);
    } else {
        caret_to_start(edit);
    }
    let length = send(edit, WM_GETTEXTLENGTH, 0, 0).max(0) as usize;
    if length > LOG_MAX_CHARS {
        let (s, e) = if focused { (start as usize + inserted, end as usize + inserted) } else { (0, 0) };
        send(edit, EM_SETSEL, LOG_MAX_CHARS * 3 / 4, -1);
        let empty = wide("");
        send(edit, EM_REPLACESEL, 0, empty.as_ptr() as isize);
        send(edit, EM_SETSEL, s, e as isize);
    }
    if ui.config.log_to_file {
        append_log_file(text);
    }
}

fn caret_to_start(edit: HWND) {
    send(edit, EM_SETSEL, 0, 0);
    send(edit, EM_SCROLLCARET, 0, 0);
}

fn append_log_file(text: &str) {
    let path = crate::config::config_path().with_file_name("notefetch.log");
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let now = crate::clock::to_local(&crate::clock::now_utc());
        let _ = writeln!(f, "{} {}", now.list_date(), text.replace('\n', " "));
    }
}

fn take_snapshot(ui: &mut Ui, s: Snapshot) {
    let wanted = ui.want_folder.take();
    let keep_folder = wanted.clone().or_else(|| Some(combo_text(ui.dialog, IDC_FOLDER)));
    if s.folders != ui.snapshot.folders || wanted.is_some() {
        fill_folders(ui.dialog, &s.folders, keep_folder);
    }
    ui.snapshot = s;
    apply_filter(ui);
}

fn matches(row: &Row, show: usize, folder: &str, filter: &str) -> bool {
    let shown = match show {
        1 => row.state == State::Downloaded,
        2 => matches!(row.state, State::New | State::Failed),
        3 => row.incomplete,
        4 => matches!(row.state, State::Deleted | State::Skipped),
        _ => !matches!(row.state, State::Deleted | State::Skipped),
    };
    let in_folder = match folder {
        "" | "All folders" => true,
        "Uncategorised" => row.folder.is_none(),
        f => row.folder.as_deref() == Some(f),
    };
    shown && in_folder && (filter.is_empty() || row.name.to_lowercase().contains(filter))
}

/// Rebuild the list from the snapshot and the three choices above it,
/// keeping the selection and focus on the same recordings, by Plaud ID.
fn apply_filter(ui: &mut Ui) {
    let show = send(item(ui.dialog, IDC_SHOW), CB_GETCURSEL, 0, 0).max(0) as usize;
    let folder = combo_text(ui.dialog, IDC_FOLDER);
    let filter = filter_text(ui.dialog);

    let focused_index = next_item(ui.list, -1, LVNI_FOCUSED);
    let focused_id = ui.focus_id.clone();
    let selected: Vec<String> = selected_indices(ui.list)
        .into_iter()
        .filter_map(|i| ui.visible.get(i))
        .map(|&r| ui.snapshot.rows[r].id.clone())
        .collect();

    ui.visible = (0..ui.snapshot.rows.len())
        .filter(|&i| {
            let row = &ui.snapshot.rows[i];
            matches(row, show, &folder, &filter) || ui.kept.contains(&row.id)
        })
        .collect();

    send(ui.list, LVM_DELETEALLITEMS, 0, 0);
    for (n, &r) in ui.visible.iter().enumerate() {
        let row = &ui.snapshot.rows[r];
        // A recording on disk and in no category is in Uncategorised; one not
        // downloaded is in no folder yet.
        let folder = match (&row.folder, &row.location) {
            (Some(f), _) => f.as_str(),
            (None, Some(_)) => crate::library::UNCATEGORISED,
            (None, None) => "",
        };
        let cells = [row.name.as_str(), folder, &row.date, &row.duration, row.status];
        let mut first = wide(cells[0]);
        let it = LVITEMW {
            mask: LVIF_TEXT | LVIF_PARAM,
            iItem: n as i32,
            pszText: PWSTR(first.as_mut_ptr()),
            lParam: LPARAM(r as isize),
            ..Default::default()
        };
        send(ui.list, LVM_INSERTITEMW, 0, &it as *const _ as isize);
        for (c, text) in cells.iter().enumerate().skip(1) {
            let mut t = wide(text);
            let sub = LVITEMW { iSubItem: c as i32, pszText: PWSTR(t.as_mut_ptr()), ..Default::default() };
            send(ui.list, LVM_SETITEMTEXTW, n, &sub as *const _ as isize);
        }
    }

    if ui.visible.is_empty() {
        return;
    }
    let mut any_selected = false;
    for (n, &r) in ui.visible.iter().enumerate() {
        if selected.contains(&ui.snapshot.rows[r].id) {
            set_state(ui.list, n as isize, LVIS_SELECTED, LVIS_SELECTED);
            any_selected = true;
        }
    }
    // Focus the same recording, or failing that the row where it was.
    let target = focused_id
        .and_then(|id| ui.visible.iter().position(|&r| ui.snapshot.rows[r].id == id))
        .or(focused_index.map(|i| i.min(ui.visible.len() - 1)))
        .unwrap_or(0);
    // Select the focused row too when nothing selected is left, for example
    // after a delete: otherwise the next Delete or Ctrl+D finds nothing.
    let state = if any_selected { LVIS_FOCUSED } else { LIST_VIEW_ITEM_STATE_FLAGS(LVIS_FOCUSED.0 | LVIS_SELECTED.0) };
    set_state(ui.list, target as isize, state, state);
    send(ui.list, LVM_ENSUREVISIBLE, target, 0);
    ui.focus_id = Some(ui.snapshot.rows[ui.visible[target]].id.clone());
}

fn set_state(list: HWND, index: isize, state: LIST_VIEW_ITEM_STATE_FLAGS, mask: LIST_VIEW_ITEM_STATE_FLAGS) {
    let it = LVITEMW { state, stateMask: mask, ..Default::default() };
    send(list, LVM_SETITEMSTATE, index as usize, &it as *const _ as isize);
}

fn next_item(list: HWND, after: isize, flags: u32) -> Option<usize> {
    let i = send(list, LVM_GETNEXTITEM, after as usize, flags as isize);
    (i >= 0).then_some(i as usize)
}

fn selected_indices(list: HWND) -> Vec<usize> {
    let mut out = Vec::new();
    let mut at = -1isize;
    while let Some(i) = next_item(list, at, LVNI_SELECTED) {
        out.push(i);
        at = i as isize;
    }
    out
}

fn select_all(ui: &mut Ui) {
    if send(ui.list, LVM_GETITEMCOUNT, 0, 0) > 0 {
        set_state(ui.list, -1, LVIS_SELECTED, LVIS_SELECTED);
        let n = ui.visible.len();
        ui.pending_select = Some(if n == 1 { "1 recording selected.".to_string() } else { format!("All {n} recordings selected.") });
        // SAFETY: a timer on this window; setting it again restarts it.
        unsafe {
            SetTimer(Some(ui.dialog), SELECT_TIMER, SELECT_PAUSE_MS, None);
        }
    }
}

fn download_selected(ui: &mut Ui) {
    let ids: Vec<String> = selected_indices(ui.list)
        .into_iter()
        .filter_map(|i| ui.visible.get(i))
        .map(|&r| ui.snapshot.rows[r].id.clone())
        .collect();
    if ids.is_empty() {
        say(ui, "No recordings are selected. Select one or more in the list, then choose Download selected.", Importance::Error);
        return;
    }
    start(ui, Command::Download(ids), "A download");
    // SAFETY: focus on a control of this window.
    unsafe {
        let _ = SetFocus(Some(ui.list));
    }
}

fn open_output(ui: &mut Ui) {
    let path = ui.config.output_folder.clone();
    if let Err(e) = std::fs::create_dir_all(&path) {
        let text = crate::error::disk("create the output folder", &path, &e).to_string();
        say(ui, &text, Importance::Error);
        return;
    }
    open_in_explorer(ui, &path);
}

fn open_recording(ui: &mut Ui) {
    let row = next_item(ui.list, -1, LVNI_FOCUSED).and_then(|i| ui.visible.get(i)).map(|&r| ui.snapshot.rows[r].clone());
    match row.and_then(|r| r.location.clone().map(|l| (r, l))) {
        Some((_, location)) => {
            let path = ui.config.output_folder.join(location);
            open_in_explorer(ui, &path);
        }
        None => say(ui, "That recording is not downloaded, so it has no folder yet.", Importance::Error),
    }
}

fn open_in_explorer(ui: &mut Ui, path: &std::path::Path) {
    let target = wide(&path.to_string_lossy());
    // SAFETY: plain call with live strings.
    let result = unsafe { ShellExecuteW(Some(ui.dialog), w!("open"), PCWSTR(target.as_ptr()), None, None, SW_SHOWNORMAL) };
    if result.0 as isize <= 32 {
        say(ui, &format!("Windows could not open \"{}\". Check that it still exists.", path.display()), Importance::Error);
    }
}

/// The focused recording, if the list has one.
fn focused(ui: &Ui) -> Option<Row> {
    next_item(ui.list, -1, LVNI_FOCUSED).and_then(|i| ui.visible.get(i)).map(|&r| ui.snapshot.rows[r].clone())
}

fn selected_rows(ui: &Ui) -> Vec<Row> {
    selected_indices(ui.list).into_iter().filter_map(|i| ui.visible.get(i)).map(|&r| ui.snapshot.rows[r].clone()).collect()
}

/// A recording's name in messages: its folder's own name when it has one, as
/// the worker's messages use, so "(2)" tells two alike apart.
fn label(row: &Row) -> String {
    row.location.as_deref().map(library::base_of).map(str::to_string).unwrap_or_else(|| row.name.clone())
}

/// What Enter does for a recording: the context menu's default item.
fn default_command(row: &Row) -> i32 {
    if row.state == State::Downloaded { IDM_PLAY } else { IDM_DOWNLOAD }
}

struct MenuItem {
    id: i32,
    text: &'static str,
    enabled: bool,
}

/// The context menu for the focused recording. SPEC.md "Context menu": items
/// depend on its status, and with several recordings selected only the items
/// that make sense for a group are available; the rest are shown but
/// disabled, so the menu keeps its shape. The default item, what Enter does,
/// is always available.
fn recording_menu(row: &Row, several: bool) -> Vec<MenuItem> {
    let one = !several;
    let item = |id, text, enabled| MenuItem { id, text, enabled };
    match row.state {
        State::Downloaded => {
            let mut items = vec![
                item(IDM_PLAY, "&Play\tEnter", true),
                item(IDM_TRANSCRIPT, "Open &transcript\tCtrl+T", one && row.has_transcript),
                item(IDM_SUMMARY, "Open su&mmary\tCtrl+M", one && row.has_summary),
                item(IDM_OPEN_RECORDING, "Open &folder\tCtrl+Shift+O", one),
                item(IDM_COPY_PATH, "&Copy audio file path\tCtrl+Shift+C", one),
                item(IDM_RENAME, "&Rename\tF2", one),
                item(IDM_MOVE, "Mo&ve to folder...\tCtrl+Shift+M", true),
            ];
            if row.incomplete {
                items.push(item(IDM_DOWNLOAD, "&Download missing parts\tCtrl+D", true));
            }
            items.push(item(IDM_DELETE, "D&elete\tDelete", true));
            items
        }
        State::New | State::Failed => {
            vec![item(IDM_DOWNLOAD, "&Download\tEnter", true), item(IDM_SKIP, "Do&n't download", true)]
        }
        State::Deleted | State::Skipped => vec![item(IDM_DOWNLOAD, "&Download again\tEnter", true)],
    }
}

/// Show the context menu at the focused row when the keyboard asked for it,
/// or at the mouse pointer.
fn context_menu(dialog: HWND, coords: isize) {
    let info = with_ui(|ui| {
        let row = focused(ui)?;
        let several = selected_indices(ui.list).len() > 1;
        let index = next_item(ui.list, -1, LVNI_FOCUSED)?;
        let mut point = POINT { x: (coords & 0xFFFF) as i16 as i32, y: ((coords >> 16) & 0xFFFF) as i16 as i32 };
        if coords as u32 == 0xFFFF_FFFF {
            // The keyboard: at the focused row, not wherever the mouse is.
            let mut rect = RECT { left: LVIR_LABEL as i32, ..Default::default() };
            send(ui.list, LVM_ENSUREVISIBLE, index, 0);
            send(ui.list, LVM_GETITEMRECT, index, &mut rect as *mut _ as isize);
            point = POINT { x: rect.left, y: rect.bottom };
            // SAFETY: point is a live structure; the list is this thread's.
            unsafe {
                let _ = ClientToScreen(ui.list, &mut point);
            }
        }
        Some((recording_menu(&row, several), default_command(&row), point))
    })
    .flatten();
    let Some((items, default, point)) = info else { return };
    // Shown outside the window's state, because the menu runs its own message
    // loop, and the worker's events must still get through meanwhile.
    // SAFETY: a menu created, shown and destroyed here, owned by this window.
    let chosen = unsafe {
        let Ok(menu) = CreatePopupMenu() else { return };
        for i in &items {
            let text = wide(i.text);
            let flags = if i.enabled { MF_STRING } else { MF_STRING | MF_GRAYED };
            let _ = AppendMenuW(menu, flags, i.id as usize, PCWSTR(text.as_ptr()));
        }
        let _ = SetMenuDefaultItem(menu, default as u32, 0);
        let flags = TPM_RETURNCMD | TPM_RIGHTBUTTON | TPM_LEFTALIGN | TPM_TOPALIGN;
        let chosen = TrackPopupMenu(menu, flags, point.x, point.y, None, dialog, None);
        let _ = DestroyMenu(menu);
        chosen.0
    };
    if chosen != 0 {
        command(dialog, chosen, 0);
    }
}

/// Set up a menu bar menu as it opens: the File menu from the sign-in, the
/// Recording menu from the focused recording, as its context menu would be,
/// and the Folder menu from the Folder box.
fn init_menu(dialog: HWND, menu: HMENU) {
    // SAFETY: menu queries and changes on this window's own menu bar.
    unsafe {
        let bar = GetMenu(dialog);
        if menu == GetSubMenu(bar, MENU_FILE) {
            // Sign in only when there is none, or Plaud refused it; Sign out
            // whenever one is stored, even a refused one.
            let Some(signed_in) = with_ui(signed_in) else { return };
            let stored = crate::auth::has_sign_in();
            let _ = EnableMenuItem(menu, IDM_SIGN_IN as u32, if signed_in { MF_GRAYED } else { MF_ENABLED });
            let _ = EnableMenuItem(menu, IDM_SIGN_OUT as u32, if stored { MF_ENABLED } else { MF_GRAYED });
        } else if menu == GetSubMenu(bar, MENU_RECORDING) {
            let state = with_ui(|ui| {
                let several = selected_indices(ui.list).len() > 1;
                let row = focused(ui);
                (row.as_ref().map(|r| recording_menu(r, several)), row.as_ref().map(default_command), !ui.visible.is_empty())
            });
            let Some((items, default, any)) = state else { return };
            let items = items.unwrap_or_default();
            for id in RECORDING_COMMANDS {
                // Download selected works on whatever is selected.
                let on = match id {
                    IDM_DOWNLOAD => any,
                    _ => items.iter().any(|i| i.id == id && i.enabled),
                };
                let _ = EnableMenuItem(menu, id as u32, if on { MF_ENABLED } else { MF_GRAYED });
            }
            let _ = EnableMenuItem(menu, IDM_SELECT_ALL as u32, if any { MF_ENABLED } else { MF_GRAYED });
            let _ = SetMenuDefaultItem(menu, default.map_or(u32::MAX, |d| d as u32), 0);
        } else if menu == GetSubMenu(bar, MENU_FOLDER) {
            // Rename and Delete act on the folder chosen in the Folder box,
            // and say which, so the menu explains itself.
            let chosen = chosen_folder(dialog);
            let uncategorised = library::is_uncategorised(&combo_text(dialog, IDC_FOLDER));
            for (id, verb) in [(IDM_RENAME_FOLDER, "&Rename"), (IDM_DELETE_FOLDER, "&Delete")] {
                let (text, flags) = match &chosen {
                    Some(f) => (format!("{verb} the folder \"{f}\"..."), MF_STRING),
                    None if uncategorised => {
                        (format!("{verb} folder (Uncategorised is NoteFetch's own, so it cannot be changed)"), MF_STRING | MF_GRAYED)
                    }
                    None => (format!("{verb} folder (choose one in the Folder box first)"), MF_STRING | MF_GRAYED),
                };
                let text = wide(&text);
                let _ = ModifyMenuW(menu, id as u32, MF_BYCOMMAND | flags, id as usize, PCWSTR(text.as_ptr()));
            }
        }
    }
}

/// Whether a sign-in is stored that Plaud has not refused.
fn signed_in(ui: &mut Ui) -> bool {
    crate::auth::has_sign_in() && (ui.expired.is_none() || ui.expired != crate::auth::saved_at())
}

/// The folder chosen in the Folder box, if it is one of the user's folders.
fn chosen_folder(dialog: HWND) -> Option<String> {
    let f = combo_text(dialog, IDC_FOLDER);
    (!f.is_empty() && f != "All folders" && f != "Uncategorised").then_some(f)
}

#[derive(Clone, Copy)]
enum FilePart {
    Audio,
    Transcript,
    Summary,
}

/// The file for a part of a recording, or what to say instead.
fn part_path(ui: &Ui, row: &Row, part: FilePart) -> Result<PathBuf, String> {
    let name = label(row);
    let Some(location) = row.location.as_deref().filter(|_| row.state == State::Downloaded) else {
        return Err(format!("\"{name}\" is not downloaded yet. Press Enter on it to download it."));
    };
    let dir = ui.config.output_folder.join(location);
    let base = library::base_of(location);
    let names: Vec<String> = match part {
        FilePart::Audio => vec![naming::AUDIO.file_name(base)],
        FilePart::Transcript => vec![naming::TRANSCRIPT.file_name(base)],
        // Notes are .md, or .txt if that setting was on when it was saved.
        FilePart::Summary => [".md", ".txt"].iter().map(|ext| naming::Part { label: "Summary", ext }.file_name(base)).collect(),
    };
    if let Some(path) = names.iter().map(|n| dir.join(n)).find(|p| p.is_file()) {
        return Ok(path);
    }
    let days = ui.config.retry_days;
    Err(match part {
        FilePart::Audio => {
            format!("\"{name}\" has no audio file in its folder. To fetch it, press Ctrl+D with the recording selected.")
        }
        FilePart::Transcript => format!(
            "\"{name}\" has no transcript. Plaud may not have made one yet; each sync checks again for {days} days after the recording."
        ),
        FilePart::Summary => format!(
            "\"{name}\" has no summary. Plaud may not have made one yet; each sync checks again for {days} days after the recording."
        ),
    })
}

/// Play, Open transcript and Open summary, in the user's own programs.
fn open_part(ui: &mut Ui, part: FilePart) {
    let Some(row) = focused(ui) else { return };
    match part_path(ui, &row, part) {
        Ok(path) => {
            if open_file(ui, &path) {
                let what = match part {
                    FilePart::Audio => "Playing",
                    FilePart::Transcript => "Opened the transcript of",
                    FilePart::Summary => "Opened the summary of",
                };
                log(ui, &format!("{what} \"{}\".", label(&row)));
            }
        }
        Err(why) => say(ui, &why, Importance::Error),
    }
}

/// Open a file in its default program. With none set, Windows asks which to
/// use. Says so if it fails.
fn open_file(ui: &mut Ui, path: &Path) -> bool {
    // SE_ERR_NOASSOC: no program is set for this kind of file.
    const NO_PROGRAM: isize = 31;
    let target = wide(&path.to_string_lossy());
    // SAFETY: plain calls with live strings.
    let mut result = unsafe { ShellExecuteW(Some(ui.dialog), None, PCWSTR(target.as_ptr()), None, None, SW_SHOWNORMAL) }.0 as isize;
    if result == NO_PROGRAM {
        // SAFETY: as above.
        result =
            unsafe { ShellExecuteW(Some(ui.dialog), w!("openas"), PCWSTR(target.as_ptr()), None, None, SW_SHOWNORMAL) }.0 as isize;
    }
    if result <= 32 {
        let file = path.file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default();
        say(
            ui,
            &format!(
                "Windows could not open \"{file}\". Check that a program for this kind of file is installed, or open it from File Explorer with Ctrl+Shift+O."
            ),
            Importance::Error,
        );
        return false;
    }
    true
}

fn copy_audio_path(ui: &mut Ui) {
    let Some(row) = focused(ui) else { return };
    match part_path(ui, &row, FilePart::Audio) {
        Ok(path) => {
            if copy_text(ui.dialog, &path.to_string_lossy()) {
                say(ui, &format!("Copied the path of the audio file of \"{}\".", label(&row)), Importance::Completion);
            } else {
                say(ui, "Windows did not let NoteFetch use the clipboard. Try again in a moment.", Importance::Error);
            }
        }
        Err(why) => say(ui, &why, Importance::Error),
    }
}

/// Put text on the clipboard.
fn copy_text(owner: HWND, text: &str) -> bool {
    const CF_UNICODETEXT: u32 = 13;
    let data = wide(text);
    // SAFETY: the clipboard is opened and closed here; the memory handed to it
    // is filled while locked, and freed here only if the clipboard refused it.
    unsafe {
        if OpenClipboard(Some(owner)).is_err() {
            return false;
        }
        let done = (|| -> windows::core::Result<()> {
            EmptyClipboard()?;
            let memory = GlobalAlloc(GMEM_MOVEABLE, data.len() * 2)?;
            let p = GlobalLock(memory) as *mut u16;
            if p.is_null() {
                let _ = GlobalFree(Some(memory));
                return Err(windows::core::Error::from_thread());
            }
            std::ptr::copy_nonoverlapping(data.as_ptr(), p, data.len());
            let _ = GlobalUnlock(memory);
            if let Err(e) = SetClipboardData(CF_UNICODETEXT, Some(HANDLE(memory.0))) {
                let _ = GlobalFree(Some(memory));
                return Err(e);
            }
            Ok(())
        })()
        .is_ok();
        let _ = CloseClipboard();
        done
    }
}

/// Start a change: rename, move, delete or a folder command. Changes run
/// alongside a sync or download, which refuse only the recordings they are
/// fetching at that moment (Laurent, milestone 4).
fn change(ui: &mut Ui, command: Command) {
    // A recording deleted or moved away leaves the view now, even while a
    // download keeps the view's rows.
    if let Command::Delete(ids) | Command::Skip(ids) | Command::Move { ids, .. } = &command {
        ui.kept.retain(|k| !ids.contains(k));
    }
    ui.worker.send(command, Cancel::new());
}

/// F2: edit the focused recording's name in the list, as in File Explorer.
fn rename_focused(dialog: HWND) {
    let index = with_ui(|ui| {
        let row = focused(ui)?;
        if row.state != State::Downloaded {
            say(
                ui,
                &format!("\"{}\" is not downloaded, so there is nothing to rename yet. Press Enter on it to download it.", label(&row)),
                Importance::Error,
            );
            return None;
        }
        next_item(ui.list, -1, LVNI_FOCUSED)
    })
    .flatten();
    if let Some(i) = index {
        focus_list(dialog);
        // Sent outside the window's state: the list asks the dialog about it
        // (LVN_BEGINLABELEDIT) before it returns.
        send(item(dialog, IDC_LIST), LVM_EDITLABELW, i, 0);
    }
}

/// The list is about to show its edit box. Put the name part alone in it,
/// without the date or a clash number "(2)", which are kept automatically.
/// False refuses the edit, for example a slow second click on a recording
/// that is not downloaded.
fn begin_rename(list: HWND, index: i32) -> bool {
    let name = with_ui(|ui| {
        let row = ui.visible.get(usize::try_from(index).ok()?).map(|&r| &ui.snapshot.rows[r])?;
        (row.state == State::Downloaded).then(|| row.plain_name.clone())
    })
    .flatten();
    let Some(name) = name else { return false };
    let edit = HWND(send(list, LVM_GETEDITCONTROL, 0, 0) as *mut _);
    if !edit.is_invalid() {
        let text = wide(&name);
        // SAFETY: the list's own edit box, on this thread; the subclass
        // procedure lives for the whole program.
        unsafe {
            let _ = SetWindowTextW(edit, PCWSTR(text.as_ptr()));
            let _ = SetWindowSubclass(edit, Some(edit_proc), 3, 0);
        }
        send(edit, EM_SETSEL, 0, -1);
    }
    true
}

/// The edit was confirmed with Enter or by moving away.
fn end_rename(ui: &mut Ui, index: i32, text: &str) {
    let row = usize::try_from(index).ok().and_then(|i| ui.visible.get(i)).map(|&r| ui.snapshot.rows[r].clone());
    let Some(row) = row else { return };
    let text = text.trim();
    if text.is_empty() {
        say(ui, &format!("A name cannot be empty, so \"{}\" was not renamed.", label(&row)), Importance::Error);
    } else if text != row.plain_name {
        change(ui, Command::Rename(row.id.clone(), text.to_string()));
    }
}

/// Delete: downloaded recordings go to the Recycle Bin, after a question if
/// that setting is on. Recordings never downloaded are marked Don't download,
/// which is what deleting them means. SPEC.md "Deleting".
fn delete_selected(dialog: HWND) {
    let plan = with_ui(|ui| {
        let rows = selected_rows(ui);
        if rows.is_empty() {
            say(ui, "No recordings are selected. Select one or more in the list, then press Delete.", Importance::Error);
            return None;
        }
        let on_disk: Vec<&Row> = rows.iter().filter(|r| r.state == State::Downloaded).collect();
        let never: Vec<&Row> = rows.iter().filter(|r| matches!(r.state, State::New | State::Failed)).collect();
        // With the setting off, Delete leaves recordings never downloaded
        // alone. Laurent, milestone 4.
        let never = if ui.config.delete_marks_dont_download { never } else { Vec::new() };
        if on_disk.is_empty() && never.is_empty() && rows.iter().any(|r| matches!(r.state, State::New | State::Failed)) {
            let what = match rows.as_slice() {
                [one] => format!("\"{}\" is not downloaded, so there is nothing to delete.", label(one)),
                _ => "The selected recordings are not downloaded, so there is nothing to delete.".to_string(),
            };
            say(ui, &format!("{what} To leave a recording out of syncs, choose Don't download from the context menu."), Importance::Error);
            return None;
        }
        if on_disk.is_empty() && never.is_empty() {
            say(
                ui,
                "The selected recordings are already deleted. To bring one back, press Enter on it to download it again.",
                Importance::Error,
            );
            return None;
        }
        let ids: Vec<String> = on_disk.iter().chain(&never).map(|r| r.id.clone()).collect();
        let question = (ui.config.confirm_delete && !on_disk.is_empty()).then(|| delete_question(&on_disk, never.len()));
        Some((ids, question))
    })
    .flatten();
    let Some((ids, question)) = plan else { return };
    if let Some(q) = question {
        let text = wide(&q);
        // A question the user must answer. Yes is the default, as in File
        // Explorer: the Recycle Bin can undo it.
        // SAFETY: a modal box owned by this window.
        let answer = unsafe { MessageBoxW(Some(dialog), PCWSTR(text.as_ptr()), w!("Delete"), MB_YESNO | MB_ICONQUESTION | MB_DEFBUTTON1) };
        if answer != IDYES {
            with_ui(|ui| log(ui, "Nothing was deleted."));
            focus_list(dialog);
            return;
        }
    }
    with_ui(|ui| change(ui, Command::Delete(ids)));
    focus_list(dialog);
}

/// SPEC.md "Deleting": name the recording, and say it will not come back by
/// itself.
fn delete_question(on_disk: &[&Row], never: usize) -> String {
    let mut q = match on_disk {
        [one] => format!(
            "Move \"{}\" to the Recycle Bin?\r\n\r\nNoteFetch will not download it again unless you choose Download again in the Deleted view.",
            label(one)
        ),
        many => format!(
            "Move these {} recordings to the Recycle Bin?\r\n\r\nNoteFetch will not download them again unless you choose Download again in the Deleted view.",
            many.len()
        ),
    };
    match never {
        0 => {}
        1 => q.push_str(" 1 of the selected recordings is not downloaded; it will be marked Don't download."),
        n => q.push_str(&format!(" {n} of the selected recordings are not downloaded; they will be marked Don't download.")),
    }
    q.push_str(" Nothing is deleted from your Plaud account.");
    q
}

fn skip_selected(ui: &mut Ui) {
    let ids: Vec<String> =
        selected_rows(ui).into_iter().filter(|r| matches!(r.state, State::New | State::Failed)).map(|r| r.id).collect();
    if ids.is_empty() {
        say(
            ui,
            "Don't download is only for recordings not yet downloaded. To remove a downloaded recording, press Delete.",
            Importance::Error,
        );
        return;
    }
    change(ui, Command::Skip(ids));
}

fn focus_list(dialog: HWND) {
    // SAFETY: focus on a control of this window.
    unsafe {
        let _ = SetFocus(Some(item(dialog, IDC_LIST)));
    }
}

/// What the Move to folder window needs, and what it chose.
struct MoveChoice {
    folders: Vec<String>,
    /// Where the recordings are now, if all in one place: None for not in a
    /// folder.
    current: Option<Option<String>>,
    /// The folder chosen (None for out of any folder), and whether it is new.
    result: Option<(Option<String>, bool)>,
}

/// Ctrl+Shift+M: move the selected recordings into a folder. SPEC.md
/// "Folders".
fn move_selected(dialog: HWND) {
    let plan = with_ui(|ui| {
        let rows = selected_rows(ui);
        let on_disk: Vec<Row> = rows.iter().filter(|r| r.state == State::Downloaded).cloned().collect();
        if on_disk.is_empty() {
            let why = if rows.is_empty() {
                "No recordings are selected. Select one or more in the list, then press Ctrl+Shift+M."
            } else {
                "Only downloaded recordings can be moved to a folder. Download them first."
            };
            say(ui, why, Importance::Error);
            return None;
        }
        if on_disk.len() < rows.len() {
            let n = rows.len() - on_disk.len();
            log(ui, &format!("{n} of the selected recordings are not downloaded, so they will not be moved."));
        }
        let first = on_disk[0].folder.clone();
        let current = on_disk.iter().all(|r| r.folder == first).then_some(first);
        let ids: Vec<String> = on_disk.into_iter().map(|r| r.id).collect();
        Some((ids, MoveChoice { folders: ui.snapshot.folders.clone(), current, result: None }))
    })
    .flatten();
    let Some((ids, mut choice)) = plan else { return };
    // SAFETY: a modal window owned by this one; `choice` outlives it.
    unsafe {
        let instance = GetModuleHandleW(None).ok().map(Into::into);
        DialogBoxParamW(instance, resource(IDD_MOVE), Some(dialog), Some(move_proc), LPARAM(&mut choice as *mut _ as isize));
    }
    match choice.result {
        Some((folder, create)) => {
            with_ui(|ui| change(ui, Command::Move { ids, folder, create }));
        }
        None => {
            with_ui(|ui| log(ui, "Nothing was moved."));
        }
    }
    focus_list(dialog);
}

unsafe extern "system" fn move_proc(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> isize {
    // SAFETY: GWLP_USERDATA holds the MoveChoice given at WM_INITDIALOG, which
    // lives until DialogBoxParamW returns.
    let choice = || unsafe { (GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut MoveChoice).as_mut() };
    match message {
        WM_INITDIALOG => {
            // SAFETY: storing the pointer passed to DialogBoxParamW.
            unsafe {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, lparam.0);
            }
            if let Some(c) = choice() {
                let list = item(hwnd, IDC_MOVE_LIST);
                let mut chosen = 0;
                let entries = std::iter::once(None).chain(c.folders.iter().map(|f| Some(f.as_str())));
                for (i, f) in entries.enumerate() {
                    let t = wide(f.unwrap_or("Uncategorised"));
                    send(list, LB_ADDSTRING, 0, t.as_ptr() as isize);
                    if c.current.as_ref().is_some_and(|cur| cur.as_deref() == f) {
                        chosen = i;
                    }
                }
                // Start where the recordings are now, so the first thing heard
                // says where they are.
                send(list, LB_SETCURSEL, chosen, 0);
            }
            // One: Windows puts the focus on the first control, the list.
            1
        }
        WM_COMMAND => {
            let id = (wparam.0 & 0xFFFF) as i32;
            let code = ((wparam.0 >> 16) & 0xFFFF) as u32;
            if id == IDOK.0 || (id == IDC_MOVE_LIST && code == LBN_DBLCLK) {
                if let Some(c) = choice() {
                    let typed = dialog_text(hwnd, IDC_MOVE_NEW);
                    let typed = typed.trim();
                    c.result = Some(if !typed.is_empty() {
                        // A name typed that is already a folder means that folder.
                        match c.folders.iter().find(|f| f.eq_ignore_ascii_case(typed)) {
                            Some(f) => (Some(f.clone()), false),
                            None if crate::library::means_uncategorised(typed) => (None, false),
                            None => (Some(typed.to_string()), true),
                        }
                    } else {
                        let i = send(item(hwnd, IDC_MOVE_LIST), LB_GETCURSEL, 0, 0);
                        (if i > 0 { c.folders.get(i as usize - 1).cloned() } else { None }, false)
                    });
                }
                // SAFETY: ends this modal window.
                unsafe {
                    let _ = EndDialog(hwnd, 1);
                }
                return 1;
            }
            if id == IDCANCEL.0 {
                // SAFETY: ends this modal window.
                unsafe {
                    let _ = EndDialog(hwnd, 0);
                }
                return 1;
            }
            0
        }
        _ => 0,
    }
}

pub(crate) fn dialog_text(dialog: HWND, id: i32) -> String {
    let mut buf = [0u16; 512];
    // SAFETY: buf is a live buffer of the length given.
    let len = unsafe { GetDlgItemTextW(dialog, id, &mut buf) } as usize;
    String::from_utf16_lossy(&buf[..len])
}

/// What the name window shows, and what was typed.
struct NameChoice {
    title: String,
    initial: String,
    result: Option<String>,
}

/// Ask for a folder name. None if cancelled or left empty.
fn ask_name(dialog: HWND, title: &str, initial: &str) -> Option<String> {
    let mut choice = NameChoice { title: title.to_string(), initial: initial.to_string(), result: None };
    // SAFETY: a modal window owned by this one; `choice` outlives it.
    unsafe {
        let instance = GetModuleHandleW(None).ok().map(Into::into);
        DialogBoxParamW(instance, resource(IDD_NAME), Some(dialog), Some(name_proc), LPARAM(&mut choice as *mut _ as isize));
    }
    choice.result.map(|r| r.trim().to_string()).filter(|r| !r.is_empty())
}

unsafe extern "system" fn name_proc(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> isize {
    // SAFETY: GWLP_USERDATA holds the NameChoice given at WM_INITDIALOG, which
    // lives until DialogBoxParamW returns.
    let choice = || unsafe { (GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut NameChoice).as_mut() };
    match message {
        WM_INITDIALOG => {
            // SAFETY: storing the pointer passed to DialogBoxParamW, then
            // setting up this window's own controls.
            unsafe {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, lparam.0);
                if let Some(c) = choice() {
                    let title = wide(&c.title);
                    let _ = SetWindowTextW(hwnd, PCWSTR(title.as_ptr()));
                    let initial = wide(&c.initial);
                    let _ = SetDlgItemTextW(hwnd, IDC_NAME_EDIT, PCWSTR(initial.as_ptr()));
                }
                let _ = SetFocus(Some(item(hwnd, IDC_NAME_EDIT)));
            }
            send(item(hwnd, IDC_NAME_EDIT), EM_SETSEL, 0, -1);
            // Zero: the focus is set above, with the name selected.
            0
        }
        WM_COMMAND => {
            let id = (wparam.0 & 0xFFFF) as i32;
            if id == IDOK.0
                && let Some(c) = choice()
            {
                c.result = Some(dialog_text(hwnd, IDC_NAME_EDIT));
            }
            if id == IDOK.0 || id == IDCANCEL.0 {
                // SAFETY: ends this modal window.
                unsafe {
                    let _ = EndDialog(hwnd, 0);
                }
                return 1;
            }
            0
        }
        _ => 0,
    }
}

fn new_folder(dialog: HWND) {
    let name = ask_name(dialog, "New folder", "");
    with_ui(|ui| match name {
        Some(n) => change(ui, Command::NewFolder(n)),
        None => log(ui, "No folder was created."),
    });
    focus_list(dialog);
}

fn rename_folder(dialog: HWND) {
    let Some(old) = chosen_folder(dialog) else {
        with_ui(|ui| say(ui, "Choose a folder in the Folder box first, then choose Rename folder.", Importance::Error));
        return;
    };
    let new = ask_name(dialog, "Rename folder", &old);
    with_ui(|ui| match new {
        Some(n) if n != old => {
            // Keep showing the same folder, under its new name.
            ui.want_folder = Some(naming::clean(&n));
            change(ui, Command::RenameFolder(old, n));
        }
        _ => log(ui, &format!("The folder \"{old}\" was not renamed.")),
    });
    focus_list(dialog);
}

fn delete_folder(dialog: HWND) {
    let Some(name) = chosen_folder(dialog) else {
        with_ui(|ui| say(ui, "Choose a folder in the Folder box first, then choose Delete folder.", Importance::Error));
        return;
    };
    let Some(count) = with_ui(|ui| ui.snapshot.rows.iter().filter(|r| r.folder.as_deref() == Some(name.as_str())).count())
    else {
        return;
    };
    let what = match count {
        0 => "It has no recordings in it.".to_string(),
        1 => "Its recording will move into Uncategorised.".to_string(),
        n => format!("Its {n} recordings will move into Uncategorised."),
    };
    let text = wide(&format!("Delete the folder \"{name}\"?\r\n\r\n{what} Nothing goes to the Recycle Bin."));
    // A question the user must answer. Nothing is lost either way, but No
    // is the default, for caution (Laurent, milestone 5 round 2).
    // SAFETY: a modal box owned by this window.
    let answer =
        unsafe { MessageBoxW(Some(dialog), PCWSTR(text.as_ptr()), w!("Delete folder"), MB_YESNO | MB_ICONQUESTION | MB_DEFBUTTON2) };
    with_ui(|ui| {
        if answer == IDYES {
            change(ui, Command::DeleteFolder(name));
        } else {
            log(ui, &format!("The folder \"{name}\" was not deleted."));
        }
    });
    focus_list(dialog);
}

/// The list's own keys. Handled here rather than in the accelerator table so
/// that, for example, Ctrl+A in the filter box still selects its text.
unsafe extern "system" fn list_proc(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM, _id: usize, _data: usize) -> LRESULT {
    // SAFETY: key state queries on this thread.
    let ctrl = unsafe { GetKeyState(VK_CONTROL.0 as i32) } < 0;
    let shift = unsafe { GetKeyState(VK_SHIFT.0 as i32) } < 0;
    let post = |id: i32| {
        // SAFETY: the parent is the dialog on this thread.
        unsafe {
            if let Ok(parent) = GetParent(hwnd) {
                let _ = PostMessageW(Some(parent), WM_COMMAND, WPARAM(id as usize), LPARAM(0));
            }
        }
    };
    match message {
        // A dialog keeps Enter for its default button. The list asks for it,
        // so Enter carries out the focused recording's default action.
        WM_GETDLGCODE if lparam.0 != 0 => {
            // SAFETY: passes the message to the list itself; for
            // WM_GETDLGCODE a non-zero lparam points at the MSG being
            // dispatched.
            let result = unsafe { DefSubclassProc(hwnd, message, wparam, lparam) };
            let msg = unsafe { &*(lparam.0 as *const MSG) };
            if msg.message == WM_KEYDOWN && msg.wParam.0 == VK_RETURN.0 as usize {
                return LRESULT(result.0 | DLGC_WANTALLKEYS as isize);
            }
            return result;
        }
        WM_KEYDOWN => {
            let key = wparam.0 as u16;
            let target = match (ctrl, shift) {
                (false, _) if key == VK_RETURN.0 => Some(IDM_DEFAULT),
                (false, false) if key == VK_DELETE.0 => Some(IDM_DELETE),
                (false, false) if key == VK_F2.0 => Some(IDM_RENAME),
                (true, false) => match key as u8 {
                    b'A' => Some(IDM_SELECT_ALL),
                    b'D' => Some(IDM_DOWNLOAD),
                    b'T' => Some(IDM_TRANSCRIPT),
                    b'M' => Some(IDM_SUMMARY),
                    _ => None,
                },
                (true, true) => match key as u8 {
                    b'O' => Some(IDM_OPEN_RECORDING),
                    b'C' => Some(IDM_COPY_PATH),
                    b'M' => Some(IDM_MOVE),
                    _ => None,
                },
                _ => None,
            };
            if let Some(id) = target {
                post(id);
                return LRESULT(0);
            }
        }
        // Swallow the control characters those keys produce (Enter and Ctrl+M
        // both make a carriage return), or the list's type-ahead search beeps
        // at them.
        WM_CHAR if wparam.0 < 0x20 => return LRESULT(0),
        // The Applications key, Shift+F10 or a right click. The dialog builds
        // the menu, outside anything the list is in the middle of.
        WM_CONTEXTMENU => {
            // SAFETY: the parent is the dialog on this thread.
            unsafe {
                if let Ok(parent) = GetParent(hwnd) {
                    let _ = PostMessageW(Some(parent), WM_APP_CONTEXT, WPARAM(0), lparam);
                }
            }
            return LRESULT(0);
        }
        _ => {}
    }
    // SAFETY: passes everything else to the list itself.
    unsafe { DefSubclassProc(hwnd, message, wparam, lparam) }
}

/// The box for editing a name in the list. It must have Enter and Escape for
/// itself, or the dialog takes them, and Escape would cancel whatever is
/// running instead of the edit.
unsafe extern "system" fn edit_proc(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM, _id: usize, _data: usize) -> LRESULT {
    // SAFETY: passes the message to the edit box itself.
    let result = unsafe { DefSubclassProc(hwnd, message, wparam, lparam) };
    match message {
        WM_GETDLGCODE => LRESULT(result.0 | DLGC_WANTALLKEYS as isize),
        WM_NCDESTROY => {
            // SAFETY: removing this procedure from a window being destroyed.
            unsafe {
                let _ = RemoveWindowSubclass(hwnd, Some(edit_proc), 3);
            }
            result
        }
        _ => result,
    }
}

/// The Log's subclass. A multi-line edit box asks the dialog for every key,
/// and on Escape it sends the dialog WM_CLOSE, so Escape in the Log closed
/// NoteFetch instead of cancelling (milestone 3, round 2). Here Escape is
/// left to the dialog, which turns it into Cancel as everywhere else.
unsafe extern "system" fn log_proc(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM, _id: usize, _data: usize) -> LRESULT {
    // SAFETY: passes the message to the edit box itself.
    let result = unsafe { DefSubclassProc(hwnd, message, wparam, lparam) };
    if message == WM_GETDLGCODE && lparam.0 != 0 {
        // SAFETY: for WM_GETDLGCODE, a non-zero lparam points at the MSG
        // being dispatched.
        let msg = unsafe { &*(lparam.0 as *const MSG) };
        if msg.message == WM_KEYDOWN && msg.wParam.0 == VK_ESCAPE.0 as usize {
            return LRESULT(result.0 & !((DLGC_WANTALLKEYS | DLGC_WANTMESSAGE) as isize));
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(state: State, name: &str, folder: Option<&str>, incomplete: bool) -> Row {
        Row {
            id: name.into(),
            name: name.into(),
            plain_name: name.into(),
            folder: folder.map(str::to_string),
            date: String::new(),
            duration: String::new(),
            status: "",
            state,
            location: None,
            incomplete,
            has_transcript: true,
            has_summary: true,
        }
    }

    #[test]
    fn show_choices_filter_by_state() {
        let d = row(State::Downloaded, "Lecture", None, false);
        let n = row(State::New, "Seminar", None, false);
        let x = row(State::Deleted, "Old", None, false);
        let s = row(State::Skipped, "Test", None, false);
        let i = row(State::Downloaded, "Half", None, true);
        let all = |r: &Row| matches(r, 0, "All folders", "");
        assert!(all(&d) && all(&n) && all(&i) && !all(&x) && !all(&s), "deleted and skipped live in the Deleted view");
        assert!(matches(&d, 1, "", "") && !matches(&n, 1, "", ""));
        assert!(matches(&n, 2, "", "") && !matches(&d, 2, "", ""));
        assert!(matches(&i, 3, "", "") && !matches(&d, 3, "", ""));
        assert!(matches(&x, 4, "", "") && matches(&s, 4, "", "") && !matches(&d, 4, "", ""));
    }

    #[test]
    fn folder_and_name_filters() {
        let a = row(State::Downloaded, "Neuroscience week 3", Some("Neuro"), false);
        let b = row(State::Downloaded, "Staff training", None, false);
        assert!(matches(&a, 0, "Neuro", "") && !matches(&b, 0, "Neuro", ""));
        assert!(matches(&b, 0, "Uncategorised", "") && !matches(&a, 0, "Uncategorised", ""));
        assert!(matches(&a, 0, "All folders", "week"));
        assert!(!matches(&a, 0, "All folders", "training"));
    }

    #[test]
    fn mnemonics_in_the_template_are_unique() {
        // Controls and menu titles share Alt+letter in the main window
        // (notefetch.rc, rule 2). Each other window, and each menu, has its
        // own set of letters.
        let rc = include_str!("../notefetch.rc");
        let mut groups: std::collections::BTreeMap<String, Vec<char>> = Default::default();
        let (mut window, mut popup) = (String::new(), String::new());
        for line in rc.lines() {
            let t = line.trim_start();
            if t.contains(" DIALOGEX ") {
                window = t.split_whitespace().next().unwrap().to_string();
            }
            if t.starts_with("IDR_MENU") {
                window = "IDD_MAIN".into();
            }
            let group = if t.starts_with("POPUP") {
                popup = t.to_string();
                window.clone()
            } else if t.starts_with("MENUITEM") {
                popup.clone()
            } else if ["LTEXT", "PUSHBUTTON", "DEFPUSHBUTTON"].iter().any(|k| t.starts_with(k)) {
                window.clone()
            } else {
                continue;
            };
            if let Some(i) = t.find('&') {
                let letter = t[i + 1..].chars().next().unwrap().to_ascii_lowercase();
                let letters = groups.entry(group.clone()).or_default();
                assert!(!letters.contains(&letter), "Alt+{letter} is used twice in {group}");
                letters.push(letter);
            }
        }
        assert_eq!(groups["IDD_MAIN"].len(), 13);
        assert_eq!(groups["IDD_MOVE"].len(), 2);
        assert_eq!(groups["POPUP \"&Recording\""].len(), 11);
    }

    #[test]
    fn the_context_menu_follows_the_status() {
        let ids = |r: &Row, several| recording_menu(r, several).iter().map(|i| i.id).collect::<Vec<_>>();
        let done = row(State::Downloaded, "Lecture", None, false);
        assert_eq!(default_command(&done), IDM_PLAY);
        assert!(!ids(&done, false).contains(&IDM_DOWNLOAD), "Download missing parts only when something is missing");
        let half = row(State::Downloaded, "Half", None, true);
        assert!(ids(&half, false).contains(&IDM_DOWNLOAD));
        let new = row(State::New, "Seminar", None, false);
        assert_eq!(ids(&new, false), [IDM_DOWNLOAD, IDM_SKIP]);
        assert_eq!(default_command(&new), IDM_DOWNLOAD);
        let gone = row(State::Deleted, "Old", None, false);
        assert_eq!(ids(&gone, false), [IDM_DOWNLOAD]);
    }

    #[test]
    fn open_transcript_and_summary_are_unavailable_when_there_is_none() {
        let enabled = |r: &Row, id| recording_menu(r, false).iter().any(|i| i.id == id && i.enabled);
        let mut audio_only = row(State::Downloaded, "Lecture", None, true);
        assert!(enabled(&audio_only, IDM_TRANSCRIPT) && enabled(&audio_only, IDM_SUMMARY));
        audio_only.has_transcript = false;
        audio_only.has_summary = false;
        assert!(!enabled(&audio_only, IDM_TRANSCRIPT) && !enabled(&audio_only, IDM_SUMMARY));
    }

    #[test]
    fn several_selected_keeps_the_shape_and_the_default() {
        let done = row(State::Downloaded, "Lecture", None, true);
        let one = recording_menu(&done, false);
        let many = recording_menu(&done, true);
        assert_eq!(one.len(), many.len(), "the menu does not change shape");
        let on: Vec<i32> = many.iter().filter(|i| i.enabled).map(|i| i.id).collect();
        assert_eq!(on, [IDM_PLAY, IDM_MOVE, IDM_DOWNLOAD, IDM_DELETE], "group items, and the default");
    }

    #[test]
    fn context_menu_letters_are_unique() {
        for state in [State::Downloaded, State::New, State::Deleted] {
            let items = recording_menu(&row(state, "x", None, true), false);
            let mut letters = Vec::new();
            for i in items {
                let at = i.text.find('&').expect("every item has an Alt letter");
                let letter = i.text[at + 1..].chars().next().unwrap().to_ascii_lowercase();
                assert!(!letters.contains(&letter), "{letter} twice in {}", i.text);
                letters.push(letter);
            }
        }
    }

    #[test]
    fn the_delete_question_names_the_recording() {
        let mut a = row(State::Downloaded, "Lecture", None, false);
        a.location = Some("Lecture, 24 Sep 2026, 1430".into());
        let q = delete_question(&[&a], 0);
        assert!(q.starts_with("Move \"Lecture, 24 Sep 2026, 1430\" to the Recycle Bin?"));
        assert!(q.contains("Download again") && q.ends_with("Plaud account."));
        let b = row(State::Downloaded, "Seminar", None, false);
        assert!(delete_question(&[&a, &b], 1).contains("these 2 recordings") );
        assert!(delete_question(&[&a, &b], 1).contains("1 of the selected recordings is not downloaded"));
    }
}
