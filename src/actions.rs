//! What the user does to recordings and folders on disk, keeping the database
//! in step. SPEC.md "Playing, renaming and deleting" and "Folders".
//!
//! None of this needs Plaud, so none of it needs a sign-in: it works on the
//! database and the output folder alone. Each returns the line for the Log,
//! saying what happened in words.

use crate::clock::Stamp;
use crate::error::{Error, Result};
use crate::library::{self, Library};
use crate::naming;
use crate::store::{Record, State, Store};

/// The database and the output folder, for the user's own changes.
pub struct Local<'a> {
    pub store: &'a Store,
    pub library: &'a Library,
}

impl Local<'_> {
    fn record(&self, id: &str) -> Result<Record> {
        self.store.get(id)?.ok_or_else(|| Error::Database("that recording is not in the list any more.".into()))
    }

    fn on_disk(&self, r: &Record) -> Result<String> {
        r.location
            .clone()
            .filter(|l| self.library.path_of(l).is_dir())
            .ok_or_else(|| Error::Disk(format!("\"{}\" is not downloaded, so there is nothing on disk to change.", library::display_name(r))))
    }

    /// Rename locally: folder, files, database and `metadata.json`.
    pub fn rename(&self, id: &str, new_name: &str) -> Result<String> {
        let r = self.record(id)?;
        let before = library::display_name(&r);
        let start = Stamp::from_unix_millis(r.start_ms);
        let clean = naming::clean(new_name);
        if let Some(location) = r.location.clone().filter(|l| self.library.path_of(l).is_dir()) {
            let new_location = self.library.rename(&location, &clean, &start)?;
            self.store.set_location(id, Some(&new_location))?;
        }
        self.store.set_local_name(id, Some(&clean))?;
        let after = library::display_name(&self.record(id)?);
        if after == before {
            return Ok(format!("\"{before}\" already has that name, so nothing was changed."));
        }
        Ok(format!("Renamed \"{before}\" to \"{after}\". The name in Plaud is unchanged."))
    }

    /// Move a recording into a category folder, or into Uncategorised with
    /// `None`.
    pub fn move_to_folder(&self, id: &str, folder: Option<&str>) -> Result<String> {
        let r = self.record(id)?;
        let location = self.on_disk(&r)?;
        let name = library::display_name(&r);
        let folder = folder.filter(|f| !library::means_uncategorised(f));
        let already = match folder {
            Some(f) => library::category_of(&location).is_some_and(|c| c.eq_ignore_ascii_case(f)),
            None => library::folder_of(&location).is_some_and(library::is_uncategorised),
        };
        if already {
            return Ok(format!("\"{name}\" is already in the folder \"{}\".", folder.unwrap_or(library::UNCATEGORISED)));
        }
        let new_location = self.library.move_to(&location, folder)?;
        self.store.set_location(id, Some(&new_location))?;
        Ok(format!("Moved \"{name}\" to the folder \"{}\".", folder.unwrap_or(library::UNCATEGORISED)))
    }

    /// Move a recording to the Recycle Bin and remember it as deleted.
    pub fn delete(&self, id: &str) -> Result<String> {
        let r = self.record(id)?;
        let location = self.on_disk(&r)?;
        self.library.recycle(&location)?;
        self.store.set_state(id, State::Deleted)?;
        self.store.set_location(id, None)?;
        self.store.set_parts(id, false, false, false)?;
        Ok(format!(
            "Moved \"{}\" to the Recycle Bin. It will not be downloaded again unless you choose Download again in the Deleted view.",
            library::display_name(&r)
        ))
    }

    /// "Don't download", for a recording never downloaded.
    pub fn skip(&self, id: &str) -> Result<String> {
        let r = self.record(id)?;
        match r.state {
            State::Downloaded => {
                Err(Error::Disk(format!("\"{}\" is already downloaded. Use Delete instead.", library::display_name(&r))))
            }
            State::Deleted | State::Skipped => {
                Ok(format!("\"{}\" is already in the Deleted view, so nothing was changed.", library::display_name(&r)))
            }
            State::New | State::Failed => {
                self.store.set_state(id, State::Skipped)?;
                Ok(format!("\"{}\" will not be downloaded. It is listed in the Deleted view.", library::display_name(&r)))
            }
        }
    }

    pub fn create_folder(&self, name: &str) -> Result<String> {
        let made = self.library.create_folder(name)?;
        Ok(format!("Created the folder \"{made}\"."))
    }

    pub fn rename_folder(&self, old: &str, new: &str) -> Result<String> {
        let made = self.library.rename_folder(old, new)?;
        if made == old {
            return Ok(format!("The folder \"{old}\" already has that name, so nothing was changed."));
        }
        self.relocate_folder(old, &made)?;
        Ok(format!("Renamed the folder \"{old}\" to \"{made}\"."))
    }

    pub fn delete_folder(&self, name: &str) -> Result<String> {
        let moved = self.library.delete_folder(name);
        // Update the database for whatever moved, even if the folder stayed.
        if let Some(scan) = self.library.scan()? {
            self.library.reconcile(self.store, &scan)?;
        }
        let moved = moved?;
        Ok(match moved.len() {
            0 => format!("Deleted the folder \"{name}\"."),
            1 => format!("Deleted the folder \"{name}\". Its recording is now in {}.", library::UNCATEGORISED),
            n => format!("Deleted the folder \"{name}\". Its {n} recordings are now in {}.", library::UNCATEGORISED),
        })
    }

    /// After a folder is renamed, point its recordings at the new name.
    fn relocate_folder(&self, old: &str, new: &str) -> Result<()> {
        for r in self.store.all()? {
            if let Some(loc) = &r.location
                && library::folder_of(loc) == Some(old)
            {
                let moved = format!("{new}\\{}", library::base_of(loc));
                self.store.set_location(&r.plaud_id, Some(&moved))?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::Metadata;
    use crate::plaud::Listed;
    use std::path::PathBuf;

    struct TempDir(PathBuf);
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn setup(label: &str) -> (TempDir, Store, Library) {
        let dir = std::env::temp_dir().join(format!("notefetch-actions-{label}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let store = Store::in_memory();
        let library = Library::new(dir.clone());
        (TempDir(dir), store, library)
    }

    fn start() -> Stamp {
        Stamp::parse("2026-09-24T13:30:00").unwrap()
    }

    fn listed(store: &Store, id: &str) {
        store.upsert_listed(&Listed { id: id.into(), name: id.into(), start: start(), duration_ms: 1 }).unwrap();
    }

    /// A recording on disk and in the database, as a download leaves it.
    fn downloaded(store: &Store, library: &Library, id: &str, name: &str) -> String {
        listed(store, id);
        let loc = library.create_recording_folder(id, name, &start()).unwrap();
        let m = Metadata {
            plaud_id: id.into(),
            plaud_name: name.into(),
            local_name: name.into(),
            start: start().iso_utc(),
            duration_ms: 1,
            parts: Default::default(),
        };
        library.write_metadata(&loc, &m).unwrap();
        store.set_state(id, State::Downloaded).unwrap();
        store.set_location(id, Some(&loc)).unwrap();
        loc
    }

    #[test]
    fn dont_download_is_only_for_recordings_never_downloaded() {
        let (_t, store, library) = setup("skip");
        let local = Local { store: &store, library: &library };
        listed(&store, "new");
        downloaded(&store, &library, "done", "Lecture");
        assert!(local.skip("new").is_ok());
        assert_eq!(store.get("new").unwrap().unwrap().state, State::Skipped);
        assert!(local.skip("new").unwrap().contains("already in the Deleted view"));
        assert!(local.skip("done").is_err());
    }

    #[test]
    fn rename_to_the_same_name_says_so() {
        let (_t, store, library) = setup("samename");
        let local = Local { store: &store, library: &library };
        downloaded(&store, &library, "a", "Lecture");
        assert!(local.rename("a", "Lecture").unwrap().contains("already has that name"));
        let line = local.rename("a", "Seminar").unwrap();
        assert_eq!(line, "Renamed \"Lecture, 24 Sep 2026, 1430\" to \"Seminar, 24 Sep 2026, 1430\". The name in Plaud is unchanged.");
        assert_eq!(store.get("a").unwrap().unwrap().location.as_deref(), Some("Uncategorised\\Seminar, 24 Sep 2026, 1430"));
    }

    #[test]
    fn moving_and_renaming_a_folder_keep_the_database_in_step() {
        let (_t, store, library) = setup("folders");
        let local = Local { store: &store, library: &library };
        downloaded(&store, &library, "a", "Lecture");
        local.create_folder("Course").unwrap();
        local.move_to_folder("a", Some("Course")).unwrap();
        assert!(local.move_to_folder("a", Some("Course")).unwrap().contains("already in the folder"));
        local.rename_folder("Course", "Neuroscience").unwrap();
        let loc = store.get("a").unwrap().unwrap().location.unwrap();
        assert_eq!(loc, "Neuroscience\\Lecture, 24 Sep 2026, 1430");
        assert!(library.path_of(&loc).is_dir());
        local.delete_folder("Neuroscience").unwrap();
        assert_eq!(store.get("a").unwrap().unwrap().location.as_deref(), Some("Uncategorised\\Lecture, 24 Sep 2026, 1430"));
        assert!(local.move_to_folder("a", None).unwrap().contains("already in the folder \"Uncategorised\""));
        assert!(local.create_folder("uncategorised").is_err(), "the name is reserved in any case");
    }
}
