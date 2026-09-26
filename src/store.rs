//! The local database of recordings, `notefetch.db` in `%LOCALAPPDATA%\NoteFetch`.
//!
//! It remembers what Plaud has, what is on disk, and what the user decided.
//! Two things in it are the user's and sync must never override them: the
//! local name, and the Deleted and Skipped states. SPEC.md "Sync".

use std::path::Path;

use rusqlite::{Connection, OptionalExtension, Row, params};

use crate::error::Result;
use crate::plaud::Listed;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// Known from Plaud, never downloaded.
    New,
    /// On disk, at least in part.
    Downloaded,
    /// The last download attempt failed.
    Failed,
    /// Marked "Don't download".
    Skipped,
    /// Moved to the Recycle Bin, in NoteFetch or outside it.
    Deleted,
}

impl State {
    fn as_str(self) -> &'static str {
        match self {
            State::New => "new",
            State::Downloaded => "downloaded",
            State::Failed => "failed",
            State::Skipped => "skipped",
            State::Deleted => "deleted",
        }
    }

    fn parse(s: &str) -> State {
        match s {
            "downloaded" => State::Downloaded,
            "failed" => State::Failed,
            "skipped" => State::Skipped,
            "deleted" => State::Deleted,
            _ => State::New,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Record {
    pub plaud_id: String,
    pub plaud_name: String,
    /// The name part the user chose, if they renamed it.
    pub local_name: Option<String>,
    /// Unix milliseconds, UTC.
    pub start_ms: i64,
    pub duration_ms: u64,
    pub state: State,
    /// The recording's folder relative to the output folder, when on disk.
    pub location: Option<String>,
    pub has_audio: bool,
    pub has_transcript: bool,
    pub has_summary: bool,
    /// When `get_file` was last called for it, Unix milliseconds.
    pub last_checked_ms: Option<i64>,
    /// What went wrong last time, in plain words.
    pub error: Option<String>,
}

impl Record {
    /// The status shown in the list, in words. SPEC.md "Main window".
    pub fn status(&self) -> &'static str {
        match self.state {
            State::Deleted => "Deleted",
            State::Skipped => "Skipped",
            State::Failed => "Failed",
            State::New => "Not downloaded",
            State::Downloaded => match (self.has_transcript, self.has_summary) {
                (true, true) => "Downloaded",
                (false, true) => "No transcript",
                (true, false) => "No summary",
                (false, false) => "Audio only",
            },
        }
    }
}

pub struct Store {
    db: Connection,
}

/// Each entry moves the schema up one version. Never edit an entry once
/// released; add a new one.
const MIGRATIONS: &[&str] = &["
    CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
    CREATE TABLE recordings (
        plaud_id TEXT PRIMARY KEY,
        plaud_name TEXT NOT NULL,
        local_name TEXT,
        start_ms INTEGER NOT NULL,
        duration_ms INTEGER NOT NULL,
        state TEXT NOT NULL,
        location TEXT,
        has_audio INTEGER NOT NULL DEFAULT 0,
        has_transcript INTEGER NOT NULL DEFAULT 0,
        has_summary INTEGER NOT NULL DEFAULT 0,
        last_checked_ms INTEGER,
        error TEXT
    );
    CREATE INDEX recordings_by_start ON recordings (start_ms DESC);
"];

const COLUMNS: &str = "plaud_id, plaud_name, local_name, start_ms, duration_ms, state, location, has_audio, has_transcript, has_summary, last_checked_ms, error";

impl Store {
    pub fn open(path: &Path) -> Result<Store> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| crate::error::disk("create the folder", dir, &e))?;
        }
        Store::prepare(Connection::open(path)?)
    }

    #[cfg(test)]
    pub fn in_memory() -> Store {
        Store::prepare(Connection::open_in_memory().unwrap()).unwrap()
    }

    fn prepare(db: Connection) -> Result<Store> {
        db.busy_timeout(std::time::Duration::from_secs(5))?;
        let version: i64 = db.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        for (i, sql) in MIGRATIONS.iter().enumerate().skip(version as usize) {
            let tx = db.unchecked_transaction()?;
            tx.execute_batch(sql)?;
            tx.pragma_update(None, "user_version", i as i64 + 1)?;
            tx.commit()?;
        }
        Ok(Store { db })
    }

    fn row(r: &Row) -> rusqlite::Result<Record> {
        Ok(Record {
            plaud_id: r.get(0)?,
            plaud_name: r.get(1)?,
            local_name: r.get(2)?,
            start_ms: r.get(3)?,
            duration_ms: r.get::<_, i64>(4)? as u64,
            state: State::parse(&r.get::<_, String>(5)?),
            location: r.get(6)?,
            has_audio: r.get(7)?,
            has_transcript: r.get(8)?,
            has_summary: r.get(9)?,
            last_checked_ms: r.get(10)?,
            error: r.get(11)?,
        })
    }

    /// Record a recording from the Plaud list. A new one is added as New; a
    /// known one gets Plaud's current name, time and duration, and nothing of
    /// the user's is touched. Returns true if it was new.
    pub fn upsert_listed(&self, l: &Listed) -> Result<bool> {
        let changed = self.db.execute(
            "INSERT INTO recordings (plaud_id, plaud_name, start_ms, duration_ms, state) VALUES (?1, ?2, ?3, ?4, 'new')
             ON CONFLICT(plaud_id) DO NOTHING",
            params![l.id, l.name, l.start.unix_millis(), l.duration_ms as i64],
        )?;
        if changed == 0 {
            self.db.execute(
                "UPDATE recordings SET plaud_name = ?2, start_ms = ?3, duration_ms = ?4 WHERE plaud_id = ?1",
                params![l.id, l.name, l.start.unix_millis(), l.duration_ms as i64],
            )?;
        }
        Ok(changed == 1)
    }

    pub fn get(&self, id: &str) -> Result<Option<Record>> {
        Ok(self
            .db
            .query_row(&format!("SELECT {COLUMNS} FROM recordings WHERE plaud_id = ?1"), [id], Store::row)
            .optional()?)
    }

    /// Every recording, newest first.
    pub fn all(&self) -> Result<Vec<Record>> {
        let mut stmt = self.db.prepare(&format!("SELECT {COLUMNS} FROM recordings ORDER BY start_ms DESC, plaud_id"))?;
        let rows = stmt.query_map([], Store::row)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn set_state(&self, id: &str, state: State) -> Result<()> {
        self.db.execute("UPDATE recordings SET state = ?2 WHERE plaud_id = ?1", params![id, state.as_str()])?;
        Ok(())
    }

    pub fn set_location(&self, id: &str, location: Option<&str>) -> Result<()> {
        self.db.execute("UPDATE recordings SET location = ?2 WHERE plaud_id = ?1", params![id, location])?;
        Ok(())
    }

    pub fn set_parts(&self, id: &str, audio: bool, transcript: bool, summary: bool) -> Result<()> {
        self.db.execute(
            "UPDATE recordings SET has_audio = ?2, has_transcript = ?3, has_summary = ?4 WHERE plaud_id = ?1",
            params![id, audio, transcript, summary],
        )?;
        Ok(())
    }

    pub fn set_local_name(&self, id: &str, name: Option<&str>) -> Result<()> {
        self.db.execute("UPDATE recordings SET local_name = ?2 WHERE plaud_id = ?1", params![id, name])?;
        Ok(())
    }

    pub fn set_checked(&self, id: &str, when_ms: i64) -> Result<()> {
        self.db.execute("UPDATE recordings SET last_checked_ms = ?2 WHERE plaud_id = ?1", params![id, when_ms])?;
        Ok(())
    }

    pub fn set_error(&self, id: &str, error: Option<&str>) -> Result<()> {
        self.db.execute("UPDATE recordings SET error = ?2 WHERE plaud_id = ?1", params![id, error])?;
        Ok(())
    }

    fn meta(&self, key: &str) -> Result<Option<String>> {
        Ok(self.db.query_row("SELECT value FROM meta WHERE key = ?1", [key], |r| r.get(0)).optional()?)
    }

    fn set_meta(&self, key: &str, value: &str) -> Result<()> {
        self.db.execute(
            "INSERT INTO meta (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    /// The first-sync choice: sync downloads by itself only recordings that
    /// started at or after this moment (Unix milliseconds). `None` means the
    /// user has not chosen yet. SPEC.md "Sync".
    pub fn sync_from(&self) -> Result<Option<i64>> {
        Ok(self.meta("sync_from")?.and_then(|v| v.parse().ok()))
    }

    pub fn set_sync_from(&self, from_ms: i64) -> Result<()> {
        self.set_meta("sync_from", &from_ms.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::Stamp;

    fn listed(id: &str, name: &str, start: &str) -> Listed {
        Listed { id: id.into(), name: name.into(), start: Stamp::parse(start).unwrap(), duration_ms: 60_000 }
    }

    #[test]
    fn new_recordings_are_added_once() {
        let s = Store::in_memory();
        assert!(s.upsert_listed(&listed("a", "One", "2026-09-01T10:00:00")).unwrap());
        assert!(!s.upsert_listed(&listed("a", "One", "2026-09-01T10:00:00")).unwrap());
        assert_eq!(s.all().unwrap().len(), 1);
        assert_eq!(s.get("a").unwrap().unwrap().state, State::New);
    }

    #[test]
    fn listing_never_overrides_the_users_choices() {
        let s = Store::in_memory();
        s.upsert_listed(&listed("a", "One", "2026-09-01T10:00:00")).unwrap();
        s.upsert_listed(&listed("b", "Two", "2026-09-02T10:00:00")).unwrap();
        s.set_state("a", State::Deleted).unwrap();
        s.set_state("b", State::Skipped).unwrap();
        s.set_local_name("a", Some("My name")).unwrap();
        s.upsert_listed(&listed("a", "Renamed in Plaud", "2026-09-01T10:00:00")).unwrap();
        s.upsert_listed(&listed("b", "Two", "2026-09-02T10:00:00")).unwrap();
        let a = s.get("a").unwrap().unwrap();
        assert_eq!(a.state, State::Deleted);
        assert_eq!(a.local_name.as_deref(), Some("My name"));
        assert_eq!(a.plaud_name, "Renamed in Plaud");
        assert_eq!(s.get("b").unwrap().unwrap().state, State::Skipped);
    }

    #[test]
    fn newest_first() {
        let s = Store::in_memory();
        s.upsert_listed(&listed("old", "Old", "2026-02-11T10:00:00")).unwrap();
        s.upsert_listed(&listed("new", "New", "2026-09-24T08:00:00")).unwrap();
        let ids: Vec<String> = s.all().unwrap().into_iter().map(|r| r.plaud_id).collect();
        assert_eq!(ids, ["new", "old"]);
    }

    #[test]
    fn statuses_in_words() {
        let s = Store::in_memory();
        s.upsert_listed(&listed("a", "One", "2026-09-01T10:00:00")).unwrap();
        let status = |s: &Store| s.get("a").unwrap().unwrap().status();
        assert_eq!(status(&s), "Not downloaded");
        s.set_state("a", State::Downloaded).unwrap();
        s.set_parts("a", true, false, false).unwrap();
        assert_eq!(status(&s), "Audio only");
        s.set_parts("a", true, true, false).unwrap();
        assert_eq!(status(&s), "No summary");
        s.set_parts("a", true, false, true).unwrap();
        assert_eq!(status(&s), "No transcript");
        s.set_parts("a", true, true, true).unwrap();
        assert_eq!(status(&s), "Downloaded");
        s.set_state("a", State::Failed).unwrap();
        assert_eq!(status(&s), "Failed");
    }

    #[test]
    fn first_sync_choice_is_remembered() {
        let s = Store::in_memory();
        assert_eq!(s.sync_from().unwrap(), None);
        s.set_sync_from(123).unwrap();
        s.set_sync_from(456).unwrap();
        assert_eq!(s.sync_from().unwrap(), Some(456));
    }

    #[test]
    fn migrations_run_once() {
        let path = std::env::temp_dir().join(format!("notefetch-test-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        {
            let s = Store::open(&path).unwrap();
            s.upsert_listed(&listed("a", "One", "2026-09-01T10:00:00")).unwrap();
        }
        let s = Store::open(&path).unwrap();
        assert_eq!(s.all().unwrap().len(), 1);
        drop(s);
        let _ = std::fs::remove_file(&path);
    }
}
