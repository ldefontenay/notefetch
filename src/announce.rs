//! Speaking through the screen reader with UI Automation notification events.
//! SPEC.md "Announcements"; PLAN.md "UI Automation notifications".
//!
//! Built into Windows, so there are no DLLs to ship. Whether JAWS speaks a
//! notification raised from a plain Win32 dialog is `[untested]`: milestone 3
//! has a Help menu item that raises one on purpose, to find out.
//!
//! Everything announced is also written to the Log by the caller, so nothing
//! is heard once and then gone.

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Accessibility::{
    NotificationKind_Other, NotificationProcessing_ImportantAll, UiaClientsAreListening, UiaHostProviderFromHwnd,
    UiaRaiseNotificationEvent,
};
use windows::core::BSTR;

/// Ask the screen reader to say `text`. Returns false if Windows refused, or
/// if nothing is listening, so the caller can note it in the Log.
pub fn say(hwnd: HWND, text: &str) -> bool {
    // SAFETY: plain calls on a window this thread owns; the provider and
    // strings are released when they go out of scope.
    unsafe {
        if !UiaClientsAreListening().as_bool() {
            return false;
        }
        let Ok(provider) = UiaHostProviderFromHwnd(hwnd) else { return false };
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
