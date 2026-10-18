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
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::{
    ICC_LISTVIEW_CLASSES, INITCOMMONCONTROLSEX, InitCommonControlsEx, LIST_VIEW_ITEM_STATE_FLAGS, LVCF_SUBITEM,
    LVCF_TEXT, LVCF_WIDTH, LVCOLUMNW, LVIF_PARAM, LVIF_TEXT, LVIS_FOCUSED, LVIS_SELECTED, LVITEMW, LVM_DELETEALLITEMS,
    LVM_ENSUREVISIBLE, LVM_GETITEMCOUNT, LVM_GETNEXTITEM, LVM_INSERTCOLUMNW, LVM_INSERTITEMW,
    LVM_SETEXTENDEDLISTVIEWSTYLE, LVM_SETITEMSTATE, LVM_SETITEMTEXTW, LVN_ITEMCHANGED, LVNI_FOCUSED, LVNI_SELECTED,
    LVS_EX_FULLROWSELECT, NMHDR, NMLISTVIEW,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetFocus, GetKeyState, SetFocus, VK_CONTROL, VK_ESCAPE, VK_SHIFT};
use windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass, ShellExecuteW};
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::{PCWSTR, PWSTR, w};

use crate::announce;
use crate::cancel::Cancel;
use crate::config::{Announcements, Config};
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
const IDM_DOWNLOAD: i32 = 2010;
const IDM_SELECT_ALL: i32 = 2011;
const IDM_OPEN_RECORDING: i32 = 2012;
const IDM_SYNC: i32 = 2020;
const IDM_REFRESH: i32 = 2021;
const IDM_CANCEL: i32 = 2022;
const IDM_FIND: i32 = 2023;
const IDM_OPEN_OUTPUT: i32 = 2024;
const IDM_TEST_SPEECH: i32 = 2030;

// Edit control messages, from winuser.h.
const EM_GETSEL: u32 = 0x00B0;
const EM_SETSEL: u32 = 0x00B1;
const EM_SCROLLCARET: u32 = 0x00B7;
const EM_REPLACESEL: u32 = 0x00C2;
const EM_SETLIMITTEXT: u32 = 0x00C5;

/// Posted to the window to offer signing in, outside the event handling,
/// because the question is a modal box. wparam is 1 if the sign-in expired.
const WM_APP_ASK_SIGN_IN: u32 = WM_APP + 11;

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
    /// Recordings in the list when the running command started, by Plaud ID.
    /// They stay in the list until it finishes, even if they no longer match
    /// the Show choice, so "Not yet downloaded" does not shrink under the
    /// user during a download (Laurent, round 2).
    kept: Vec<String>,
    config: Config,
    last_progress: Option<Instant>,
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

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

fn resource(id: u16) -> PCWSTR {
    PCWSTR(id as usize as *const u16)
}

fn item(dialog: HWND, id: i32) -> HWND {
    // SAFETY: a plain lookup on a window this thread owns.
    unsafe { GetDlgItem(Some(dialog), id).unwrap_or_default() }
}

fn send(hwnd: HWND, message: u32, wparam: usize, lparam: isize) -> isize {
    // SAFETY: messages to controls this thread owns, with arguments of the
    // types each message documents.
    unsafe { SendMessageW(hwnd, message, Some(WPARAM(wparam)), Some(LPARAM(lparam))).0 }
}

/// Show the main window and pump messages until it closes.
pub fn run() -> i32 {
    // SAFETY: standard Win32 start-up, all on this thread.
    unsafe {
        let icc = INITCOMMONCONTROLSEX { dwSize: size_of::<INITCOMMONCONTROLSEX>() as u32, dwICC: ICC_LISTVIEW_CLASSES };
        let _ = InitCommonControlsEx(&icc);
        let Ok(instance) = GetModuleHandleW(None) else { return 2 };
        let Ok(dialog) = CreateDialogParamW(Some(instance.into()), resource(IDD_MAIN), None, Some(dialog_proc), LPARAM(0)) else {
            return 2;
        };
        let accel = LoadAcceleratorsW(Some(instance.into()), resource(IDR_ACCEL)).ok();
        let _ = ShowWindow(dialog, SW_SHOW);
        let _ = SetForegroundWindow(dialog);

        let mut message = MSG::default();
        loop {
            // Zero is WM_QUIT and -1 an error; both end the loop.
            if GetMessageW(&mut message, None, 0, 0).0 <= 0 {
                break;
            }
            // Accelerators first, then the dialog's own keyboard handling.
            if let Some(a) = accel
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
    let mut rect = windows::Win32::Foundation::RECT::default();
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
            kept: Vec::new(),
            config,
            last_progress: None,
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
    let mut entries = vec!["All folders".to_string(), "Not in a folder".to_string()];
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
    ui.last_progress = None;
    ui.kept = ui.visible.iter().map(|&r| ui.snapshot.rows[r].id.clone()).collect();
    ui.worker.send(command, cancel);
}

fn cancel(ui: &mut Ui) {
    match (&ui.cancel, ui.busy) {
        (Some(c), true) => {
            c.cancel();
            say(ui, "Cancelling.", Importance::Completion);
        }
        _ => say(ui, "Nothing is running, so there is nothing to cancel.", Importance::Completion),
    }
}

/// Offer to sign in, after the worker found the sign-in missing or expired.
fn ask_sign_in(hwnd: HWND, expired: bool) {
    let question = if expired {
        "Your Plaud sign-in has expired. Sign in again now?\r\n\r\nNoteFetch will open Plaud's sign-in page in your web browser."
    } else {
        "You are not signed in to Plaud. Sign in now?\r\n\r\nNoteFetch will open Plaud's sign-in page in your web browser."
    };
    let text = wide(question);
    // A question the user must answer, which is what a message box is for.
    // SAFETY: a modal box owned by this window.
    let answer = unsafe { MessageBoxW(Some(hwnd), PCWSTR(text.as_ptr()), w!("NoteFetch"), MB_YESNO | MB_ICONQUESTION | MB_DEFBUTTON1) };
    with_ui(|ui| {
        ui.asking = false;
        if answer == IDYES {
            start(ui, Command::SignIn, "Signing in");
        } else {
            log(ui, "Not signed in. You can sign in at any time from the File menu.");
        }
    });
}

fn close(hwnd: HWND) {
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
                UiEvent::Announce(line, importance) => say(ui, &line, importance),
                UiEvent::Speak(line, importance) => speak(ui, &line, importance),
                UiEvent::Progress { done, total } => {
                    let due = ui.last_progress.is_none_or(|t| t.elapsed() >= PROGRESS_EVERY);
                    if due && done > 0 && done < total {
                        ui.last_progress = Some(Instant::now());
                        // A sync also counts recordings only checked again, so
                        // "Downloaded 1 of 6" was wrong when nothing was (round 3).
                        let verb = if ui.activity == Activity::Downloading { "Downloaded" } else { "Fetched" };
                        say(ui, &format!("{verb} {done} of {total}."), Importance::Routine);
                    }
                }
                UiEvent::Snapshot(s) => take_snapshot(ui, s),
                UiEvent::Activity(a) => ui.activity = a,
                UiEvent::SignInNeeded(expired) => {
                    if !ui.asking {
                        ui.asking = true;
                        // SAFETY: posting to this thread's own window.
                        unsafe {
                            let _ = PostMessageW(Some(ui.dialog), WM_APP_ASK_SIGN_IN, WPARAM(expired as usize), LPARAM(0));
                        }
                    }
                }
                UiEvent::Finished => {
                    ui.busy = false;
                    ui.cancel = None;
                    if !ui.kept.is_empty() {
                        ui.kept.clear();
                        apply_filter(ui);
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
    let keep_folder = Some(combo_text(ui.dialog, IDC_FOLDER));
    if s.folders != ui.snapshot.folders {
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
        "Not in a folder" => row.folder.is_none(),
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
        let cells = [row.name.as_str(), row.folder.as_deref().unwrap_or(""), &row.date, &row.duration, row.status];
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
    for (n, &r) in ui.visible.iter().enumerate() {
        if selected.contains(&ui.snapshot.rows[r].id) {
            set_state(ui.list, n as isize, LVIS_SELECTED, LVIS_SELECTED);
        }
    }
    // Focus the same recording, or failing that the row where it was.
    let target = focused_id
        .and_then(|id| ui.visible.iter().position(|&r| ui.snapshot.rows[r].id == id))
        .or(focused_index.map(|i| i.min(ui.visible.len() - 1)))
        .unwrap_or(0);
    let state = if selected.is_empty() { LIST_VIEW_ITEM_STATE_FLAGS(LVIS_FOCUSED.0 | LVIS_SELECTED.0) } else { LVIS_FOCUSED };
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

/// The list's own keys. Handled here rather than in the accelerator table so
/// that, for example, Ctrl+A in the filter box still selects its text.
unsafe extern "system" fn list_proc(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM, _id: usize, _data: usize) -> LRESULT {
    // SAFETY: key state queries on this thread.
    let ctrl = unsafe { GetKeyState(VK_CONTROL.0 as i32) } < 0;
    let shift = unsafe { GetKeyState(VK_SHIFT.0 as i32) } < 0;
    if message == WM_KEYDOWN && ctrl {
        let target = match (wparam.0 as u8, shift) {
            (b'A', false) => Some(IDM_SELECT_ALL),
            (b'D', false) => Some(IDM_DOWNLOAD),
            (b'O', true) => Some(IDM_OPEN_RECORDING),
            _ => None,
        };
        if let Some(id) = target {
            // SAFETY: the parent is the dialog on this thread.
            unsafe {
                if let Ok(parent) = GetParent(hwnd) {
                    let _ = PostMessageW(Some(parent), WM_COMMAND, WPARAM(id as usize), LPARAM(0));
                }
            }
            return LRESULT(0);
        }
    }
    // Swallow the control characters those keys produce, or the list's
    // type-ahead search beeps at them.
    if message == WM_CHAR && ctrl && matches!(wparam.0, 0x01 | 0x04 | 0x0F) {
        return LRESULT(0);
    }
    // SAFETY: passes everything else to the list itself.
    unsafe { DefSubclassProc(hwnd, message, wparam, lparam) }
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
            folder: folder.map(str::to_string),
            date: String::new(),
            duration: String::new(),
            status: "",
            state,
            location: None,
            incomplete,
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
        assert!(matches(&b, 0, "Not in a folder", "") && !matches(&a, 0, "Not in a folder", ""));
        assert!(matches(&a, 0, "All folders", "week"));
        assert!(!matches(&a, 0, "All folders", "training"));
    }

    #[test]
    fn mnemonics_in_the_template_are_unique() {
        // Controls and menu titles share Alt+letter in one window (notefetch.rc, rule 2).
        let rc = include_str!("../notefetch.rc");
        let mut letters = Vec::new();
        for line in rc.lines() {
            let t = line.trim_start();
            let is_label = t.starts_with("LTEXT") || t.starts_with("PUSHBUTTON") || t.starts_with("POPUP");
            if let (true, Some(i)) = (is_label, t.find('&')) {
                let letter = t[i + 1..].chars().next().unwrap().to_ascii_lowercase();
                assert!(!letters.contains(&letter), "Alt+{letter} is used twice");
                letters.push(letter);
            }
        }
        assert_eq!(letters.len(), 13);
    }
}
