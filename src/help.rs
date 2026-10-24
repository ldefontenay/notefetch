//! The user guide, compiled into the exe and opened in the browser on F1.
//! SPEC.md "Files the app writes"; PLAN.md "A user guide with tutorials".
//!
//! HTML in a browser, because a browser is the only place a screen reader
//! gives real heading navigation over a long document. The file is written out
//! afresh every time, so the guide can never be older than the program, and
//! handed to the shell as a `file:///` URL, not a path: a path can bring up an
//! "Open with" chooser when the .html association is stale, and Windows
//! reports that as success (the accessible-windows-apps skill, `[proven]` in
//! TweakSound).

use std::path::{Path, PathBuf};

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
use windows::core::{PCWSTR, w};

use crate::ui::wide;

const DOCUMENT: &str = include_str!("help.html");

/// The guide with this version's number in it.
fn document() -> String {
    DOCUMENT.replace("{{VERSION}}", env!("CARGO_PKG_VERSION"))
}

/// Write the guide out and open it in the default browser. The error, if
/// any, is a sentence for the Log saying what to do next.
pub fn open(owner: HWND) -> Result<(), String> {
    let path = write_out().map_err(|e| {
        format!("NoteFetch could not write its guide to a file ({e}), so it could not open it. Try again, and if it keeps happening, restart Windows.")
    })?;
    let url = wide(&file_url(&path));
    // SAFETY: plain call with a live string.
    let result = unsafe { ShellExecuteW(Some(owner), w!("open"), PCWSTR(url.as_ptr()), None, None, SW_SHOWNORMAL) };
    // An error code at or below 32, a meaningless positive value above it.
    if result.0 as isize <= 32 {
        return Err(format!(
            "Windows could not open the guide in a web browser. You can open it yourself from File Explorer: it is the file \"{}\".",
            path.display()
        ));
    }
    Ok(())
}

fn write_out() -> std::io::Result<PathBuf> {
    let dir = crate::config::local_dir();
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("notefetch-help.html");
    std::fs::write(&path, document())?;
    Ok(path)
}

/// A path as a `file:///` URL, percent-encoded, so a space in a user's folder
/// name does not end it early.
fn file_url(path: &Path) -> String {
    let mut url = String::from("file:///");
    for c in path.to_string_lossy().chars() {
        match c {
            '\\' | '/' => url.push('/'),
            ':' | '-' | '.' | '_' | '~' => url.push(c),
            c if c.is_ascii_alphanumeric() => url.push(c),
            c => {
                let mut buffer = [0u8; 4];
                for byte in c.encode_utf8(&mut buffer).as_bytes() {
                    url.push_str(&format!("%{byte:02X}"));
                }
            }
        }
    }
    url
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_space_or_accent_in_a_folder_name_is_encoded() {
        let url = file_url(Path::new(r"C:\Users\Anne Marie\AppData\Local\Zoë\notefetch-help.html"));
        assert!(!url.contains(' '));
        assert!(url.starts_with("file:///C:/Users/Anne%20Marie/"));
        assert!(url.contains("Zo%C3%AB"));
        assert!(url.ends_with("notefetch-help.html"));
    }

    #[test]
    fn the_version_is_filled_in() {
        let d = document();
        assert!(!d.contains("{{VERSION}}"));
        assert!(d.contains(env!("CARGO_PKG_VERSION")));
    }

    /// CLAUDE.md: UK English, no em dashes; the skill: no tables without
    /// proper headers, and here no tables at all.
    #[test]
    fn the_guide_keeps_the_house_style() {
        assert!(!DOCUMENT.contains('\u{2014}'), "no em dashes");
        assert!(!DOCUMENT.contains("<table"), "the keys are a list, not a table");
        assert!(DOCUMENT.contains("lang=\"en-GB\""));
        // After the stylesheet, where CSS's own "color" is no spelling choice.
        let text = &DOCUMENT[DOCUMENT.find("</style>").unwrap()..];
        for american in ["Uncategorized", "color", "organiz", "canceled", "favorite", "license"] {
            assert!(!text.contains(american), "American spelling: {american}");
        }
    }

    /// Every link inside the guide leads to a heading that exists.
    #[test]
    fn every_contents_link_has_a_target() {
        for part in DOCUMENT.split("href=\"#").skip(1) {
            let id = &part[..part.find('"').unwrap()];
            assert!(DOCUMENT.contains(&format!("id=\"{id}\"")), "no target for #{id}");
        }
    }

    /// The guide describes the real window: every Alt letter and shortcut it
    /// gives is the one in the template.
    #[test]
    fn the_guide_names_the_real_controls() {
        let rc = include_str!("../notefetch.rc");
        for (label, key) in [
            ("&Show:", "Alt+S"),
            ("Fold&er:", "Alt+E"),
            ("Filter by &name:", "Alt+N"),
            ("Re&cordings:", "Alt+C"),
            ("S&ync new", "Alt+Y"),
            ("&Download selected", "Alt+D"),
            ("&Open output folder", "Alt+O"),
            ("Lo&g:", "Alt+G"),
        ] {
            assert!(rc.contains(label), "{label} is not in notefetch.rc");
            assert!(DOCUMENT.contains(key), "{key} is not in the guide");
        }
        for key in ["Ctrl+S", "F5", "Ctrl+D", "Ctrl+T", "Ctrl+M", "Ctrl+Shift+M", "Ctrl+Shift+C", "Ctrl+Shift+O", "Ctrl+Comma", "F2", "Shift+F10", "Escape", "F1"] {
            assert!(DOCUMENT.contains(key), "{key} is not in the guide");
        }
    }
}
