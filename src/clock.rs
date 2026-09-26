//! Times: Plaud's UTC times, local time for people, and the words used for both.
//!
//! Plaud sends times with no zone marker, and they are UTC (proven in
//! milestone 1: a recording named "17:27:14" in British Summer Time has
//! `start_at` "16:27:14"). Conversion to local time goes through Windows, which
//! knows the historical rules for every zone, so a recording from last winter
//! is shown in winter time even when viewed in summer.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use windows::Win32::Foundation::SYSTEMTIME;
use windows::Win32::System::Time::{DYNAMIC_TIME_ZONE_INFORMATION, SystemTimeToTzSpecificLocalTimeEx};

/// A moment as calendar fields. Used for both UTC and local time; which one it
/// is depends on where it came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Stamp {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
    pub millis: u32,
}

const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
const WEEKDAYS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

impl Stamp {
    /// Parse Plaud's form, `2026-09-24T08:00:26` with optional fractional
    /// seconds and an optional trailing `Z`. A space may stand for the `T`.
    pub fn parse(text: &str) -> Option<Stamp> {
        let t = text.trim().trim_end_matches('Z');
        let b = t.as_bytes();
        if b.len() < 19 || b[4] != b'-' || b[7] != b'-' || !(b[10] == b'T' || b[10] == b' ') || b[13] != b':' || b[16] != b':' {
            return None;
        }
        let num = |r: std::ops::Range<usize>| t.get(r).and_then(|s| s.parse::<u32>().ok());
        let mut millis = 0;
        if b.len() > 19 {
            if b[19] != b'.' {
                return None;
            }
            let frac = &t[20..];
            if frac.is_empty() || !frac.bytes().all(|c| c.is_ascii_digit()) {
                return None;
            }
            let padded = format!("{:0<3}", &frac[..frac.len().min(3)]);
            millis = padded.parse().ok()?;
        }
        let s = Stamp {
            year: num(0..4)? as i32,
            month: num(5..7)?,
            day: num(8..10)?,
            hour: num(11..13)?,
            minute: num(14..16)?,
            second: num(17..19)?,
            millis,
        };
        let valid = (1..=12).contains(&s.month)
            && s.day >= 1
            && s.day <= days_in_month(s.year, s.month)
            && s.hour < 24
            && s.minute < 60
            && s.second < 60;
        valid.then_some(s)
    }

    /// Milliseconds since 1 January 1970, treating the fields as UTC.
    pub fn unix_millis(&self) -> i64 {
        let days = days_from_civil(self.year, self.month, self.day);
        ((days * 24 + self.hour as i64) * 60 + self.minute as i64) * 60_000 + self.second as i64 * 1000 + self.millis as i64
    }

    pub fn from_unix_millis(ms: i64) -> Stamp {
        let days = ms.div_euclid(86_400_000);
        let rest = ms.rem_euclid(86_400_000);
        let (year, month, day) = civil_from_days(days);
        Stamp {
            year,
            month,
            day,
            hour: (rest / 3_600_000) as u32,
            minute: (rest / 60_000 % 60) as u32,
            second: (rest / 1000 % 60) as u32,
            millis: (rest % 1000) as u32,
        }
    }

    /// The moment as a `SystemTime`, treating the fields as UTC. Used to set
    /// file and folder dates.
    pub fn system_time(&self) -> SystemTime {
        let ms = self.unix_millis();
        if ms >= 0 {
            UNIX_EPOCH + Duration::from_millis(ms as u64)
        } else {
            UNIX_EPOCH - Duration::from_millis(ms.unsigned_abs())
        }
    }

    /// ISO 8601 with a `Z`, for `metadata.json`.
    pub fn iso_utc(&self) -> String {
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        )
    }

    /// Monday is 0.
    fn weekday(&self) -> usize {
        // 1 January 1970 was a Thursday.
        (days_from_civil(self.year, self.month, self.day) + 3).rem_euclid(7) as usize
    }

    /// "Thu 24 Sep 2026, 14:30", for the list. SPEC.md "Main window".
    pub fn list_date(&self) -> String {
        format!(
            "{} {} {} {}, {:02}:{:02}",
            WEEKDAYS[self.weekday()],
            self.day,
            MONTHS[self.month as usize - 1],
            self.year,
            self.hour,
            self.minute
        )
    }

    /// "2026-09-24 1430", the suffix of every folder and file name.
    pub fn name_suffix(&self) -> String {
        format!("{:04}-{:02}-{:02} {:02}{:02}", self.year, self.month, self.day, self.hour, self.minute)
    }
}

/// The current moment in UTC.
pub fn now_utc() -> Stamp {
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    Stamp::from_unix_millis(ms)
}

/// Convert UTC to this computer's local time, using Windows' rules for the
/// date in question.
pub fn to_local(utc: &Stamp) -> Stamp {
    to_local_in(utc, None)
}

/// Convert UTC to local time in a given zone, or this computer's when `None`.
pub fn to_local_in(utc: &Stamp, zone: Option<&DYNAMIC_TIME_ZONE_INFORMATION>) -> Stamp {
    let st = SYSTEMTIME {
        wYear: utc.year as u16,
        wMonth: utc.month as u16,
        wDayOfWeek: 0,
        wDay: utc.day as u16,
        wHour: utc.hour as u16,
        wMinute: utc.minute as u16,
        wSecond: utc.second as u16,
        wMilliseconds: utc.millis as u16,
    };
    let mut local = SYSTEMTIME::default();
    let zone_ptr = zone.map(|z| z as *const DYNAMIC_TIME_ZONE_INFORMATION);
    // SAFETY: both pointers refer to live, correctly sized structures.
    match unsafe { SystemTimeToTzSpecificLocalTimeEx(zone_ptr, &st, &mut local) } {
        Ok(()) => Stamp {
            year: local.wYear as i32,
            month: local.wMonth as u32,
            day: local.wDay as u32,
            hour: local.wHour as u32,
            minute: local.wMinute as u32,
            second: local.wSecond as u32,
            millis: local.wMilliseconds as u32,
        },
        // Windows only refuses impossible dates, which Stamp::parse rejects.
        Err(_) => *utc,
    }
}

/// "1 hour 12 minutes", for the list. SPEC.md "Main window".
pub fn duration_words(ms: u64) -> String {
    let total_seconds = ms.div_ceil(1000);
    if total_seconds < 60 {
        return plural(total_seconds, "second");
    }
    let total_minutes = (total_seconds + 30) / 60;
    let (hours, minutes) = (total_minutes / 60, total_minutes % 60);
    match (hours, minutes) {
        (0, m) => plural(m, "minute"),
        (h, 0) => plural(h, "hour"),
        (h, m) => format!("{} {}", plural(h, "hour"), plural(m, "minute")),
    }
}

fn plural(n: u64, unit: &str) -> String {
    if n == 1 { format!("1 {unit}") } else { format!("{n} {unit}s") }
}

/// A position in a recording: "12:05", or "1:02:05" from an hour on.
pub fn offset(ms: u64) -> String {
    let s = ms / 1000;
    let (h, m, s) = (s / 3600, s / 60 % 60, s % 60);
    if h > 0 { format!("{h}:{m:02}:{s:02}") } else { format!("{m:02}:{s:02}") }
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 => 29,
        _ => 28,
    }
}

// Howard Hinnant's civil calendar algorithms.
fn days_from_civil(y: i32, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y } as i64;
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let m = m as i64;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = (yoe + era * 400 + if m <= 2 { 1 } else { 0 }) as i32;
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::System::Time::EnumDynamicTimeZoneInformation;

    fn zone(key: &str) -> DYNAMIC_TIME_ZONE_INFORMATION {
        for i in 0.. {
            let mut z = DYNAMIC_TIME_ZONE_INFORMATION::default();
            // SAFETY: z is a live structure of the right size.
            if unsafe { EnumDynamicTimeZoneInformation(i, &mut z) } != 0 {
                break;
            }
            let name = String::from_utf16_lossy(&z.TimeZoneKeyName);
            if name.trim_end_matches('\0') == key {
                return z;
            }
        }
        panic!("time zone {key} not found");
    }

    fn uk(text: &str) -> Stamp {
        to_local_in(&Stamp::parse(text).unwrap(), Some(&zone("GMT Standard Time")))
    }

    #[test]
    fn parses_plaud_forms() {
        let s = Stamp::parse("2026-09-24T08:00:26.699000").unwrap();
        assert_eq!((s.year, s.month, s.day, s.hour, s.minute, s.second, s.millis), (2026, 9, 24, 8, 0, 26, 699));
        assert_eq!(Stamp::parse("2026-09-21T15:01:31").unwrap().millis, 0);
        assert_eq!(Stamp::parse("2026-09-21T15:01:31Z").unwrap().hour, 15);
        assert_eq!(Stamp::parse("2026-09-21T15:01:31.5").unwrap().millis, 500);
        assert!(Stamp::parse("2026-02-30T10:00:00").is_none());
        assert!(Stamp::parse("2026-09-21").is_none());
        assert!(Stamp::parse("").is_none());
        assert!(Stamp::parse("2026-09-21T15:01:31.").is_none());
    }

    #[test]
    fn unix_round_trip() {
        let s = Stamp::parse("2026-09-24T08:00:26.699").unwrap();
        assert_eq!(Stamp::from_unix_millis(s.unix_millis()), s);
        assert_eq!(Stamp::parse("1970-01-01T00:00:00").unwrap().unix_millis(), 0);
        assert_eq!(Stamp::parse("2000-03-01T00:00:00").unwrap().unix_millis(), 951_868_800_000);
    }

    #[test]
    fn milestone_1_example_is_bst() {
        // Laurent's recording "2026-09-09 17:27:14" has start_at 16:27:14.
        let local = uk("2026-09-09T16:27:14");
        assert_eq!((local.hour, local.minute, local.second), (17, 27, 14));
    }

    #[test]
    fn spring_clock_change() {
        // Clocks go forward at 01:00 UTC on 29 March 2026.
        assert_eq!(uk("2026-03-29T00:59:00").hour, 0);
        assert_eq!(uk("2026-03-29T01:00:00").hour, 2);
    }

    #[test]
    fn autumn_clock_change() {
        // Clocks go back at 01:00 UTC on 25 October 2026.
        assert_eq!(uk("2026-10-25T00:59:00").hour, 1);
        assert_eq!(uk("2026-10-25T01:00:00").hour, 1);
        assert_eq!(uk("2026-10-25T02:00:00").hour, 2);
    }

    #[test]
    fn winter_recording_stays_in_winter_time() {
        assert_eq!(uk("2026-02-11T10:00:00").hour, 10);
    }

    #[test]
    fn day_rolls_over_in_local_time() {
        let local = uk("2026-06-30T23:30:00");
        assert_eq!((local.month, local.day, local.hour), (7, 1, 0));
    }

    #[test]
    fn list_and_name_forms() {
        let s = Stamp::parse("2026-09-24T14:30:00").unwrap();
        assert_eq!(s.list_date(), "Thu 24 Sep 2026, 14:30");
        assert_eq!(s.name_suffix(), "2026-09-24 1430");
        assert_eq!(Stamp::parse("2026-02-01T09:05:00").unwrap().list_date(), "Sun 1 Feb 2026, 09:05");
    }

    #[test]
    fn durations_in_words() {
        assert_eq!(duration_words(0), "0 seconds");
        assert_eq!(duration_words(1000), "1 second");
        assert_eq!(duration_words(45_000), "45 seconds");
        assert_eq!(duration_words(60_000), "1 minute");
        assert_eq!(duration_words(1_713_000), "29 minutes");
        assert_eq!(duration_words(3_600_000), "1 hour");
        assert_eq!(duration_words(7_093_000), "1 hour 58 minutes");
        assert_eq!(duration_words(4_320_000), "1 hour 12 minutes");
        assert_eq!(duration_words(7_200_000), "2 hours");
    }

    #[test]
    fn offsets() {
        assert_eq!(offset(0), "00:00");
        assert_eq!(offset(11_040), "00:11");
        assert_eq!(offset(725_000), "12:05");
        assert_eq!(offset(3_725_000), "1:02:05");
    }
}
