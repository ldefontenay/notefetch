//! Folder and file names. SPEC.md "What is saved".
//!
//! Every recording's folder and files share one base name: the recording's
//! name first, then a comma, then the local start date and time, for example
//! `Cognitive neuroscience week 3, 2026-09-24 1430`. The name comes first so a
//! screen reader says the meaningful part first and first-letter navigation
//! works in File Explorer.

use std::path::Path;

use crate::clock::Stamp;

/// Full paths are kept under this length (SPEC.md).
pub const MAX_PATH_CHARS: usize = 240;

/// Room kept for the longest part suffix, such as " cleaned transcript.txt"
/// or a note tab name, plus a clash number.
const SUFFIX_ROOM: usize = 40;

/// Longest note tab name kept in a file name.
const TAB_NAME_CHARS: usize = 30;

pub const UNTITLED: &str = "Untitled";

pub const AUDIO: &str = ".ogg";
pub const TRANSCRIPT: &str = " transcript.txt";
pub const CLEANED_TRANSCRIPT: &str = " cleaned transcript.txt";
pub const OUTLINE: &str = " outline.txt";
pub const HIGHLIGHTS: &str = " highlights.txt";
pub const METADATA: &str = "metadata.json";

/// The name part for a recording from its Plaud name. Plaud's default name is
/// the local date and time, and renamed recordings often keep them at the end,
/// so a trailing `YYYY-MM-DD HH:MM[:SS]` is removed: the suffix adds it back.
pub fn name_from_plaud(plaud_name: &str) -> String {
    clean(strip_trailing_date_time(plaud_name))
}

fn strip_trailing_date_time(name: &str) -> &str {
    let trimmed = name.trim_end();
    for len in [19, 16] {
        if trimmed.len() >= len && trimmed.is_char_boundary(trimmed.len() - len) {
            let (head, tail) = trimmed.split_at(trimmed.len() - len);
            if is_date_time(tail) && (head.is_empty() || head.ends_with([' ', ',', '-', '_'])) {
                return head.trim_end_matches([' ', ',', '-', '_']);
            }
        }
    }
    trimmed
}

fn is_date_time(s: &str) -> bool {
    let b = s.as_bytes();
    let digits = |r: std::ops::Range<usize>| b[r].iter().all(u8::is_ascii_digit);
    let base = b.len() >= 16
        && digits(0..4)
        && b[4] == b'-'
        && digits(5..7)
        && b[7] == b'-'
        && digits(8..10)
        && (b[10] == b' ' || b[10] == b'T')
        && digits(11..13)
        && b[13] == b':'
        && digits(14..16);
    match b.len() {
        16 => base,
        19 => base && b[16] == b':' && digits(17..19),
        _ => false,
    }
}

/// Make text safe as part of a Windows file name. Characters Windows forbids
/// are replaced with readable ones, runs of spaces are collapsed, trailing dots
/// and spaces are removed, and a reserved device name gets a leading
/// underscore. Empty text becomes "Untitled".
pub fn clean(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        let replacement = match c {
            ':' => Some('.'),
            '/' | '\\' | '|' => Some('-'),
            '"' => Some('\''),
            '<' | '>' | '?' | '*' => None,
            c if c.is_control() => Some(' '),
            c => Some(c),
        };
        if let Some(r) = replacement {
            if r.is_whitespace() {
                if !out.is_empty() && !out.ends_with(' ') {
                    out.push(' ');
                }
            } else {
                out.push(r);
            }
        }
    }
    let trimmed = out.trim_matches(' ').trim_end_matches(['.', ' ']);
    if trimmed.is_empty() {
        return UNTITLED.to_string();
    }
    if is_reserved(trimmed) {
        return format!("_{trimmed}");
    }
    trimmed.to_string()
}

fn is_reserved(name: &str) -> bool {
    let stem = name.split('.').next().unwrap_or(name).trim_end().to_ascii_uppercase();
    matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$")
        || ((stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.chars().count() == 4
            && stem.chars().nth(3).is_some_and(|c| c.is_ascii_digit() || "¹²³".contains(c)))
}

/// The base name for a recording whose folder will sit in `parent`, shortened
/// so the longest file path stays under the limit. `None` if `parent` itself
/// leaves no room.
pub fn fit_base(parent: &Path, name: &str, start_local: &Stamp) -> Option<String> {
    let suffix = format!(", {}", start_local.name_suffix());
    // parent \ base \ base + part suffix
    let parent_chars = parent.as_os_str().to_string_lossy().chars().count();
    let fixed = parent_chars + 2 + SUFFIX_ROOM + 2 * suffix.chars().count();
    let room = MAX_PATH_CHARS.checked_sub(fixed)? / 2;
    if room == 0 {
        return None;
    }
    Some(format!("{}{suffix}", shorten(name, room)))
}

/// Cut text to at most `max` characters, at a word boundary where one is near,
/// without leaving trailing dots or spaces.
pub fn shorten(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let cut: String = text.chars().take(max).collect();
    let at_word = match cut.rfind(' ') {
        Some(i) if cut[..i].chars().count() * 3 >= max * 2 => &cut[..i],
        _ => &cut[..],
    };
    let tidy = at_word.trim_end_matches(['.', ' ', ',', '-']);
    if tidy.is_empty() { cut } else { tidy.to_string() }
}

/// The name part of a base name: everything before the last ", ".
pub fn name_of_base(base: &str) -> &str {
    match base.rfind(", ") {
        Some(i) => &base[..i],
        None => base,
    }
}

/// A base name with a clash number, for a second recording with the same name
/// started in the same minute: "Lecture, 2026-09-24 1430 (2)".
pub fn numbered(base: &str, n: u32) -> String {
    if n <= 1 { base.to_string() } else { format!("{base} ({n})") }
}

/// The file name suffix for a note tab. The Summary tab gives " summary.md";
/// any other tab keeps Plaud's name for it, for example " Meeting minutes.md".
/// `seen` holds suffixes already used for this recording, so two tabs with the
/// same name get " 2", " 3" and so on.
pub fn note_suffix(tab_name: &str, seen: &mut Vec<String>) -> String {
    let label = if tab_name.trim().eq_ignore_ascii_case("summary") || tab_name.trim().is_empty() {
        "summary".to_string()
    } else {
        shorten(&clean(tab_name), TAB_NAME_CHARS)
    };
    let mut n = 1;
    loop {
        let candidate = if n == 1 { format!(" {label}.md") } else { format!(" {label} {n}.md") };
        if !seen.iter().any(|s| s.eq_ignore_ascii_case(&candidate)) {
            seen.push(candidate.clone());
            return candidate;
        }
        n += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(text: &str) -> Stamp {
        Stamp::parse(text).unwrap()
    }

    #[test]
    fn default_plaud_name_becomes_untitled() {
        assert_eq!(name_from_plaud("2026-09-09 17:27:14"), "Untitled");
        assert_eq!(name_from_plaud("2026-09-09 17:27"), "Untitled");
    }

    #[test]
    fn trailing_date_time_is_removed() {
        assert_eq!(name_from_plaud("Staff training 2026-09-24 09:00:26"), "Staff training");
        assert_eq!(name_from_plaud("Lecture, 2026-09-24 09:00"), "Lecture");
        assert_eq!(name_from_plaud("Lecture - 2026-09-24 09:00:26  "), "Lecture");
    }

    #[test]
    fn date_time_in_the_middle_is_kept() {
        assert_eq!(name_from_plaud("2026-09-24 09:00 revision"), "2026-09-24 09.00 revision");
        assert_eq!(name_from_plaud("Week3 2026-09-24 09:00:2"), "Week3 2026-09-24 09.00.2");
        // No separator before the date: it is part of a word, so it stays.
        assert_eq!(name_from_plaud("X2026-09-24 09:00"), "X2026-09-24 09.00");
    }

    #[test]
    fn forbidden_characters_are_replaced() {
        assert_eq!(clean("Q&A: what/why? <draft> *final* \"quoted\" a|b"), "Q&A. what-why draft final 'quoted' a-b");
        assert_eq!(clean("tab\there\nnew line"), "tab here new line");
        assert_eq!(clean("  lots   of   spaces  "), "lots of spaces");
    }

    #[test]
    fn trailing_dots_and_spaces_are_removed() {
        assert_eq!(clean("Notes..."), "Notes");
        assert_eq!(clean("Notes. . ."), "Notes");
        assert_eq!(clean("..."), "Untitled");
        assert_eq!(clean(""), "Untitled");
        assert_eq!(clean("???"), "Untitled");
    }

    #[test]
    fn reserved_names_get_an_underscore() {
        assert_eq!(clean("CON"), "_CON");
        assert_eq!(clean("nul"), "_nul");
        assert_eq!(clean("com1"), "_com1");
        assert_eq!(clean("LPT9.txt"), "_LPT9.txt");
        assert_eq!(clean("COM¹"), "_COM¹");
        assert_eq!(clean("Console"), "Console");
        assert_eq!(clean("COM10"), "COM10");
    }

    #[test]
    fn non_latin_names_survive() {
        assert_eq!(clean("Réunion équipe, 会议记录 — Ωmega"), "Réunion équipe, 会议记录 — Ωmega");
        assert_eq!(name_from_plaud("会议 2026-09-24 09:00:26"), "会议");
    }

    #[test]
    fn base_name_puts_the_name_first() {
        let base = fit_base(Path::new(r"C:\Users\x\Documents\NoteFetch"), "Cognitive neuroscience week 3", &at("2026-09-24T14:30:00")).unwrap();
        assert_eq!(base, "Cognitive neuroscience week 3, 2026-09-24 1430");
        assert_eq!(name_of_base(&base), "Cognitive neuroscience week 3");
    }

    #[test]
    fn long_names_are_shortened_to_fit_the_path_limit() {
        let parent = Path::new(r"C:\Users\someone\Documents\NoteFetch\A folder");
        let long = "word ".repeat(100);
        let base = fit_base(parent, long.trim(), &at("2026-09-24T14:30:00")).unwrap();
        let longest = format!("{}\\{base}\\{base} cleaned transcript.txt (2)", parent.display());
        assert!(longest.chars().count() < MAX_PATH_CHARS, "{} characters", longest.chars().count());
        assert!(base.ends_with(", 2026-09-24 1430"));
        assert!(!name_of_base(&base).ends_with(' '));
    }

    #[test]
    fn a_parent_too_deep_leaves_no_room() {
        let parent = "x".repeat(230);
        assert!(fit_base(Path::new(&parent), "Lecture", &at("2026-09-24T14:30:00")).is_none());
    }

    #[test]
    fn shortening_prefers_a_word_boundary() {
        assert_eq!(shorten("Cognitive neuroscience week", 20), "Cognitive neuroscien");
        assert_eq!(shorten("Neuroscience lecture three", 22), "Neuroscience lecture");
        assert_eq!(shorten("Supercalifragilistic", 5), "Super");
        assert_eq!(shorten("会议记录会议记录", 4), "会议记录");
        assert_eq!(shorten("short", 10), "short");
    }

    #[test]
    fn clashes_are_numbered_at_the_end() {
        assert_eq!(numbered("Lecture, 2026-09-24 1430", 1), "Lecture, 2026-09-24 1430");
        assert_eq!(numbered("Lecture, 2026-09-24 1430", 2), "Lecture, 2026-09-24 1430 (2)");
    }

    #[test]
    fn note_tabs_become_suffixes() {
        let mut seen = Vec::new();
        assert_eq!(note_suffix("Summary", &mut seen), " summary.md");
        assert_eq!(note_suffix("Meeting minutes", &mut seen), " Meeting minutes.md");
        assert_eq!(note_suffix("Meeting minutes", &mut seen), " Meeting minutes 2.md");
        assert_eq!(note_suffix("", &mut seen), " summary 2.md");
        assert_eq!(note_suffix("Ask: what next?", &mut seen), " Ask. what next.md");
    }
}
