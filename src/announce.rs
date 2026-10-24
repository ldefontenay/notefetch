//! Speaking through the screen reader with UI Automation notification events.
//! SPEC.md "Announcements"; PLAN.md "UI Automation notifications".
//!
//! Built into Windows, so there are no DLLs to ship. Two ways, because the
//! two screen readers listen differently (milestone 6, PLAN.md):
//!
//! - A UI Automation notification, for JAWS and anything else. It is raised
//!   on a small provider of NoteFetch's own, hosted by the main window, so a
//!   native UI Automation client hears it; on the bare host provider
//!   (`UiaHostProviderFromHwnd`) only JAWS did. The provider is never handed
//!   out through WM_GETOBJECT, so the window itself is still read through
//!   MSAA, exactly as before.
//! - For NVDA, which never received those notifications at all, a live
//!   region: the text goes into a hidden static control in whichever
//!   NoteFetch window is active, and an MSAA EVENT_OBJECT_LIVEREGIONCHANGED
//!   asks NVDA to read it. NVDA ignores events from windows behind the one
//!   in front, so with Settings open the main window's would not be heard.
//!
//! Everything announced is also written to the Log by the caller, so nothing
//! is heard once and then gone.

use std::cell::RefCell;

use windows::Win32::Foundation::HWND;
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::UI::Accessibility::{
    IRawElementProviderSimple, IRawElementProviderSimple_Impl, NotificationKind_Other, NotificationProcessing_ImportantAll,
    ProviderOptions, ProviderOptions_ServerSideProvider, ProviderOptions_UseComThreading, UIA_PATTERN_ID, UIA_PROPERTY_ID,
    NotifyWinEvent, UiaClientsAreListening, UiaHostProviderFromHwnd, UiaRaiseNotificationEvent,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::GetActiveWindow;
use windows::Win32::UI::WindowsAndMessaging::{
    CHILDID_SELF, CreateWindowExW, EVENT_OBJECT_LIVEREGIONCHANGED, FindWindowW, GetDlgItem, HMENU, OBJID_CLIENT,
    SetWindowTextW, WINDOW_EX_STYLE, WS_CHILD,
};
use windows::core::{BSTR, Error, IUnknown, PCWSTR, Result, implement, w};

/// A provider with nothing of its own: Windows fills in everything from the
/// window that hosts it, so the event comes from NoteFetch's main window.
#[implement(IRawElementProviderSimple)]
struct Speaker {
    hwnd: HWND,
}

impl IRawElementProviderSimple_Impl for Speaker_Impl {
    fn ProviderOptions(&self) -> Result<ProviderOptions> {
        Ok(ProviderOptions_ServerSideProvider | ProviderOptions_UseComThreading)
    }

    fn GetPatternProvider(&self, _pattern: UIA_PATTERN_ID) -> Result<IUnknown> {
        // No patterns: an empty error is S_OK with no object.
        Err(Error::empty())
    }

    fn GetPropertyValue(&self, _property: UIA_PROPERTY_ID) -> Result<VARIANT> {
        // Empty: the host window's own properties are used.
        Ok(VARIANT::default())
    }

    fn HostRawElementProvider(&self) -> Result<IRawElementProviderSimple> {
        // SAFETY: a lookup on a window owned by this thread.
        unsafe { UiaHostProviderFromHwnd(self.hwnd) }
    }
}

thread_local! {
    /// Made once, on the interface thread, for the main window.
    static SPEAKER: RefCell<Option<(HWND, IRawElementProviderSimple)>> = const { RefCell::new(None) };
}

/// Ask the screen reader to say `text`. Returns false if Windows refused, or
/// if nothing is listening, so the caller can note it in the Log.
pub fn say(hwnd: HWND, text: &str) -> bool {
    // SAFETY: a plain call with no arguments.
    if !unsafe { UiaClientsAreListening() }.as_bool() {
        return false;
    }
    let provider = SPEAKER.with(|cell| {
        let mut slot = cell.borrow_mut();
        match &*slot {
            Some((h, p)) if *h == hwnd => p.clone(),
            _ => {
                let p: IRawElementProviderSimple = Speaker { hwnd }.into();
                *slot = Some((hwnd, p.clone()));
                p
            }
        }
    });
    // SAFETY: the provider and strings live until the call returns.
    unsafe {
        UiaRaiseNotificationEvent(
            &provider,
            NotificationKind_Other,
            NotificationProcessing_ImportantAll,
            &BSTR::from(text),
            &BSTR::from("NoteFetch"),
        )
        .is_ok()
    }
}

/// Whether NVDA is running: its own hidden main window, which is how other
/// programs have long found it.
pub fn nvda_running() -> bool {
    // SAFETY: a plain lookup by class and title.
    unsafe { FindWindowW(w!("wxWindowClassNR"), w!("NVDA")) }.is_ok_and(|h| !h.is_invalid())
}

/// The hidden live region's control ID, in any NoteFetch window.
const LIVE_REGION: i32 = 1990;

/// Ask NVDA to say `text`, through a hidden live region in the NoteFetch
/// window that is active, or in `main` if none is. NVDA reads the control's
/// text when it handles the event, so several things to say at once must be
/// joined into one call, or only the last is heard.
pub fn say_live(main: HWND, text: &str) {
    let t: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    // SAFETY: windows this thread owns; the text lives until the calls
    // return. The control is made the window's last child, so it is never a
    // label for another control, and without WS_VISIBLE it is never shown or
    // reached with Tab.
    unsafe {
        let active = GetActiveWindow();
        let host = if active.is_invalid() { main } else { active };
        let region = match GetDlgItem(Some(host), LIVE_REGION) {
            Ok(r) if !r.is_invalid() => r,
            _ => {
                let instance = GetModuleHandleW(None).ok().map(Into::into);
                let made = CreateWindowExW(
                    WINDOW_EX_STYLE(0),
                    w!("STATIC"),
                    w!(""),
                    WS_CHILD,
                    0,
                    0,
                    0,
                    0,
                    Some(host),
                    Some(HMENU(LIVE_REGION as usize as *mut _)),
                    instance,
                    None,
                );
                let Ok(r) = made else { return };
                r
            }
        };
        let _ = SetWindowTextW(region, PCWSTR(t.as_ptr()));
        NotifyWinEvent(EVENT_OBJECT_LIVEREGIONCHANGED, region, OBJID_CLIENT.0, CHILDID_SELF as i32);
    }
}
