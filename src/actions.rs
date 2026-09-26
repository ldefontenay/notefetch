//! What the user does to recordings and folders on disk, keeping the database
//! in step. SPEC.md "Playing, renaming and deleting" and "Folders".
//!
//! Each returns the line for the Log, saying what happened in words.

use crate::clock::Stamp;
use crate::error::{Error, Result};
use crate::library;
use crate::naming;
use crate::store::{Record, State};
use crate::sync::Engine;

impl Engine {
    fn record(&self, id: &str) -> Result<Record> {
        self.store
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .get(id)?
            .ok_or_else(|| Error::Database("that recording is not in the list any more.".into()))
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
            self.store_do(|s| s.set_location(id, Some(&new_location)))?;
        }
        self.store_do(|s| s.set_local_name(id, Some(&clean)))?;
        let after = library::display_name(&self.record(id)?);
        Ok(format!("Renamed \"{before}\" to \"{after}\". The name in Plaud is unchanged."))
    }

    /// Move a recording into a folder, or out of any folder with `None`.
    pub fn move_to_folder(&self, id: &str, folder: Option<&str>) -> Result<String> {
        let r = self.record(id)?;
        let location = self.on_disk(&r)?;
        let new_location = self.library.move_to(&location, folder)?;
        self.store_do(|s| s.set_location(id, Some(&new_location)))?;
        let name = library::display_name(&r);
        Ok(match folder {
            Some(f) => format!("Moved \"{name}\" to the folder \"{f}\"."),
            None => format!("Moved \"{name}\" out of its folder."),
        })
    }

    /// Move a recording to the Recycle Bin and remember it as deleted.
    pub fn delete(&self, id: &str) -> Result<String> {
        let r = self.record(id)?;
        let location = self.on_disk(&r)?;
        self.library.recycle(&location)?;
        self.store_do(|s| {
            s.set_state(id, State::Deleted)?;
            s.set_location(id, None)?;
            s.set_parts(id, false, false, false)
        })?;
        Ok(format!(
            "Moved \"{}\" to the Recycle Bin. It will not be downloaded again unless you choose Download again.",
            library::display_name(&r)
        ))
    }

    /// "Don't download", for a recording never downloaded.
    pub fn skip(&self, id: &str) -> Result<String> {
        let r = self.record(id)?;
        if r.state == State::Downloaded {
            return Err(Error::Disk(format!("\"{}\" is already downloaded. Use Delete instead.", library::display_name(&r))));
        }
        self.store_do(|s| s.set_state(id, State::Skipped))?;
        Ok(format!("\"{}\" will not be downloaded. It is listed in the Deleted view.", library::display_name(&r)))
    }

    pub fn create_folder(&self, name: &str) -> Result<String> {
        let made = self.library.create_folder(name)?;
        Ok(format!("Created the folder \"{made}\"."))
    }

    pub fn rename_folder(&self, old: &str, new: &str) -> Result<String> {
        let made = self.library.rename_folder(old, new)?;
        self.relocate_folder(old, Some(&made))?;
        Ok(format!("Renamed the folder \"{old}\" to \"{made}\"."))
    }

    pub fn delete_folder(&self, name: &str) -> Result<String> {
        let moved = self.library.delete_folder(name);
        // Update the database for whatever moved, even if the folder stayed.
        let scan = self.library.scan()?;
        if let Some(scan) = scan {
            self.library.reconcile(&self.store.lock().unwrap_or_else(|p| p.into_inner()), &scan)?;
        }
        let moved = moved?;
        Ok(match moved.len() {
            0 => format!("Deleted the folder \"{name}\"."),
            1 => format!("Deleted the folder \"{name}\". Its recording is now in the output folder."),
            n => format!("Deleted the folder \"{name}\". Its {n} recordings are now in the output folder."),
        })
    }

    /// After a folder is renamed, point its recordings at the new name.
    fn relocate_folder(&self, old: &str, new: Option<&str>) -> Result<()> {
        let store = self.store.lock().unwrap_or_else(|p| p.into_inner());
        for r in store.all()? {
            if let Some(loc) = &r.location
                && library::folder_of(loc) == Some(old)
            {
                let base = library::base_of(loc);
                let moved = match new {
                    Some(f) => format!("{f}\\{base}"),
                    None => base.to_string(),
                };
                store.set_location(&r.plaud_id, Some(&moved))?;
            }
        }
        Ok(())
    }

    fn store_do(&self, f: impl FnOnce(&crate::store::Store) -> Result<()>) -> Result<()> {
        f(&self.store.lock().unwrap_or_else(|p| p.into_inner()))
    }
}
