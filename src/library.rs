//! The recordings on disk: finding them, writing them, renaming, moving,
//! folders and the Recycle Bin. SPEC.md "What is saved", "Folders",
//! "Renaming" and "Deleting".
//!
//! The disk is the truth for where things are. A recording is a folder holding
//! `metadata.json`, found by the Plaud ID inside it, either directly in the
//! output folder or one level down inside a folder of the user's. Anything
//! File Explorer does to those folders is picked up on the next scan.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::clock::{self, Stamp};
use crate::error::{Error, Result, disk};
use crate::naming::{self, METADATA, Part};
use crate::store::{Record, State, Store};

/// `metadata.json` in every recording folder.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Metadata {
    pub plaud_id: String,
    pub plaud_name: String,
    pub local_name: String,
    /// ISO 8601 UTC.
    pub start: String,
    pub duration_ms: u64,
    /// Part name to when it was downloaded, ISO 8601 UTC. Part names:
    /// "audio", "transcript", "cleaned transcript", "outline", "highlights",
    /// and "note: <tab>" for each note.
    #[serde(default)]
    pub parts: BTreeMap<String, String>,
}

impl Metadata {
    pub fn has_summary(&self) -> bool {
        self.parts.keys().any(|k| k.starts_with("note: "))
    }
}

/// What a scan of the output folder found.
#[derive(Debug, Default)]
pub struct Scan {
    /// Plaud ID to the recording's folder, relative to the output folder.
    pub recordings: HashMap<String, Found>,
    /// Folder names, sorted.
    pub folders: Vec<String>,
    /// Folders found deeper than NoteFetch looks, relative to the output folder.
    pub too_deep: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Found {
    pub location: String,
    pub metadata: Metadata,
    pub has_audio: bool,
    pub has_transcript: bool,
}

pub struct Library {
    pub root: PathBuf,
}

impl Library {
    pub fn new(root: PathBuf) -> Library {
        Library { root }
    }

    pub fn path_of(&self, location: &str) -> PathBuf {
        self.root.join(location)
    }

    /// Read the output folder. `Ok(None)` if the output folder does not exist,
    /// for example a drive not plugged in: then nothing may be marked deleted.
    pub fn scan(&self) -> Result<Option<Scan>> {
        if !self.root.is_dir() {
            return Ok(None);
        }
        let mut scan = Scan::default();
        for (name, path) in subfolders(&self.root)? {
            if let Some(found) = read_recording(&path, &name) {
                scan.recordings.insert(found.metadata.plaud_id.clone(), found);
                continue;
            }
            for (inner, inner_path) in subfolders(&path)? {
                let location = format!("{name}\\{inner}");
                match read_recording(&inner_path, &location) {
                    Some(found) => {
                        scan.recordings.insert(found.metadata.plaud_id.clone(), found);
                    }
                    None => scan.too_deep.push(location),
                }
            }
            scan.folders.push(name);
        }
        scan.folders.sort_by_key(|f| f.to_lowercase());
        Ok(Some(scan))
    }

    /// Bring the database into line with the disk, and return what changed,
    /// in words for the Log. SPEC.md "The Deleted view".
    pub fn reconcile(&self, store: &Store, scan: &Scan) -> Result<Vec<String>> {
        let mut said = Vec::new();
        for r in store.all()? {
            let name = display_name(&r);
            match scan.recordings.get(&r.plaud_id) {
                Some(found) => {
                    if matches!(r.state, State::Deleted | State::New | State::Failed | State::Skipped) {
                        store.set_state(&r.plaud_id, State::Downloaded)?;
                        if r.state == State::Deleted {
                            said.push(format!("\"{name}\" is back in the output folder, so it is marked as downloaded again."));
                        }
                    }
                    if r.location.as_deref() != Some(found.location.as_str()) {
                        store.set_location(&r.plaud_id, Some(&found.location))?;
                    }
                    store.set_parts(&r.plaud_id, found.has_audio, found.has_transcript, found.metadata.has_summary())?;
                }
                None if r.state == State::Downloaded => {
                    store.set_state(&r.plaud_id, State::Deleted)?;
                    store.set_location(&r.plaud_id, None)?;
                    store.set_parts(&r.plaud_id, false, false, false)?;
                    said.push(format!(
                        "\"{name}\" is no longer in the output folder, so it is marked as deleted and will not be downloaded again unless you choose Download again."
                    ));
                }
                None => {
                    if r.location.is_some() {
                        store.set_location(&r.plaud_id, None)?;
                    }
                }
            }
        }
        for place in &scan.too_deep {
            said.push(format!(
                "The folder \"{place}\" is more than one level inside the output folder, so NoteFetch does not look inside it. Move recordings in it up a level to see them."
            ));
        }
        Ok(said)
    }

    /// Choose and create the folder for a recording not yet on disk. The base
    /// name is numbered if a different recording already has it.
    pub fn create_recording_folder(&self, id: &str, name_part: &str, start_utc: &Stamp) -> Result<String> {
        std::fs::create_dir_all(&self.root).map_err(|e| disk("create the output folder", &self.root, &e))?;
        let local = clock::to_local(start_utc);
        let base = naming::fit_base(&self.root, name_part, &local).ok_or_else(|| {
            Error::Disk(format!(
                "The output folder \"{}\" is too deeply nested to save recordings in. Choose a shorter output folder in Settings.",
                self.root.display()
            ))
        })?;
        for n in 1..1000 {
            let candidate = naming::numbered(&base, n);
            let path = self.root.join(&candidate);
            match read_metadata(&path) {
                Some(m) if m.plaud_id == id => return Ok(candidate),
                Some(_) => continue,
                None if path.exists() => continue,
                None => {
                    std::fs::create_dir(&path).map_err(|e| disk("create the folder", &path, &e))?;
                    return Ok(candidate);
                }
            }
        }
        Err(Error::Disk(format!("Too many recordings are called \"{base}\". Rename some of them.")))
    }

    /// Write a text part if Plaud has content for it and the file is not
    /// already there. Returns true if the file now exists.
    pub fn write_text(&self, location: &str, part: Part, text: &str) -> Result<bool> {
        let path = self.file_path(location, part);
        if path.exists() {
            return Ok(true);
        }
        if text.trim().is_empty() {
            return Ok(false);
        }
        crate::config::write_atomically(&path, text.as_bytes()).map_err(|e| disk("save", &path, &e))?;
        Ok(true)
    }

    pub fn file_path(&self, location: &str, part: Part) -> PathBuf {
        self.path_of(location).join(part.file_name(base_of(location)))
    }

    pub fn read_metadata(&self, location: &str) -> Option<Metadata> {
        read_metadata(&self.path_of(location))
    }

    pub fn write_metadata(&self, location: &str, m: &Metadata) -> Result<()> {
        let path = self.path_of(location).join(METADATA);
        let text = serde_json::to_string_pretty(m).map_err(|e| Error::Disk(format!("Could not prepare metadata.json: {e}.")))?;
        crate::config::write_atomically(&path, text.as_bytes()).map_err(|e| disk("save", &path, &e))
    }

    /// Give the folder and every file in it a modified date equal to the
    /// recording's start, so File Explorer's Date modified sort gives date
    /// order. The folder goes last, because writing files changes its date.
    pub fn set_dates(&self, location: &str, start_utc: &Stamp) {
        let when = start_utc.system_time();
        let dir = self.path_of(location);
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for e in entries.flatten() {
                if e.file_type().is_ok_and(|t| t.is_file()) {
                    let _ = set_modified(&e.path(), when, false);
                }
            }
        }
        let _ = set_modified(&dir, when, true);
    }

    /// Rename a recording: its folder and every file that carries its base
    /// name. Either everything is renamed or everything is put back.
    /// Returns the new location. SPEC.md "Renaming".
    pub fn rename(&self, location: &str, new_name_part: &str, start_utc: &Stamp) -> Result<String> {
        let old_dir = self.path_of(location);
        let old_base = base_of(location).to_string();
        let parent = old_dir.parent().map(Path::to_path_buf).unwrap_or_else(|| self.root.clone());
        let local = clock::to_local(start_utc);
        let clean = naming::clean(new_name_part);
        let new_base = naming::fit_base(&parent, &clean, &local)
            .ok_or_else(|| Error::Disk("The folder is too deeply nested for that name. Try a shorter one.".into()))?;
        // Keep a clash number if the old name had one.
        let new_base = match old_base.rfind(" (") {
            Some(i) if old_base.ends_with(')') && old_base[i + 2..old_base.len() - 1].parse::<u32>().is_ok() => {
                format!("{new_base}{}", &old_base[i..])
            }
            _ => new_base,
        };
        if new_base == old_base {
            return Ok(location.to_string());
        }
        let new_dir = parent.join(&new_base);
        if new_dir.exists() && !same_folder(&new_dir, &old_dir) {
            return Err(Error::Disk(format!("There is already a recording called \"{new_base}\" there. Choose a different name.")));
        }

        let mut done: Vec<(PathBuf, PathBuf)> = Vec::new();
        let result = (|| -> Result<()> {
            for (name, path) in files(&old_dir)? {
                if let Some(renamed) = naming::rebase(&name, &old_base, &new_base) {
                    let to = old_dir.join(renamed);
                    std::fs::rename(&path, &to).map_err(|e| disk("rename", &path, &e))?;
                    done.push((to, path));
                }
            }
            std::fs::rename(&old_dir, &new_dir).map_err(|e| disk("rename", &old_dir, &e))?;
            Ok(())
        })();
        if let Err(e) = result {
            for (to, from) in done.into_iter().rev() {
                let _ = std::fs::rename(&to, &from);
            }
            return Err(e);
        }
        let new_location = relative(&self.root, &new_dir);
        if let Some(mut m) = self.read_metadata(&new_location) {
            m.local_name = clean;
            let _ = self.write_metadata(&new_location, &m);
            self.set_dates(&new_location, start_utc);
        }
        Ok(new_location)
    }

    /// Move a recording into a folder, or back to the output folder with
    /// `None`. Returns the new location.
    pub fn move_to(&self, location: &str, folder: Option<&str>) -> Result<String> {
        let from = self.path_of(location);
        let parent = match folder {
            Some(f) => self.root.join(f),
            None => self.root.clone(),
        };
        let to = parent.join(base_of(location));
        if same_folder(&from, &to) {
            return Ok(location.to_string());
        }
        if !parent.is_dir() {
            return Err(Error::Disk(format!("The folder \"{}\" no longer exists.", folder.unwrap_or(""))));
        }
        if to.exists() {
            return Err(Error::Disk(format!(
                "\"{}\" already holds a recording called \"{}\". Rename one of them first.",
                folder.unwrap_or("The output folder"),
                base_of(location)
            )));
        }
        std::fs::rename(&from, &to).map_err(|e| disk("move", &from, &e))?;
        Ok(relative(&self.root, &to))
    }

    pub fn create_folder(&self, name: &str) -> Result<String> {
        let name = naming::clean(name);
        let path = self.root.join(&name);
        if path.exists() {
            return Err(Error::Disk(format!("There is already a folder called \"{name}\".")));
        }
        std::fs::create_dir_all(&path).map_err(|e| disk("create the folder", &path, &e))?;
        Ok(name)
    }

    pub fn rename_folder(&self, old: &str, new: &str) -> Result<String> {
        let new = naming::clean(new);
        let from = self.root.join(old);
        let to = self.root.join(&new);
        if to.exists() && !same_folder(&from, &to) {
            return Err(Error::Disk(format!("There is already a folder called \"{new}\".")));
        }
        std::fs::rename(&from, &to).map_err(|e| disk("rename the folder", &from, &e))?;
        Ok(new)
    }

    /// Delete a folder of the user's: its recordings move back to the output
    /// folder first, never to the Recycle Bin. A recording that cannot move,
    /// for example because a file in it is open, stops the deletion and stays
    /// in the folder. Returns the recordings moved, as (old, new) locations.
    pub fn delete_folder(&self, name: &str) -> Result<Vec<(String, String)>> {
        let dir = self.root.join(name);
        let mut moved = Vec::new();
        for (inner, _) in subfolders(&dir)? {
            let location = format!("{name}\\{inner}");
            if self.read_metadata(&location).is_some() {
                let to = self.move_to(&location, None)?;
                moved.push((location, to));
            }
        }
        match std::fs::remove_dir(&dir) {
            Ok(()) => Ok(moved),
            Err(_) => Err(Error::Disk(format!(
                "Its recordings were moved back to the output folder, but the folder \"{name}\" still holds other files, so it was left in place."
            ))),
        }
    }

    /// Move a recording's folder to the Recycle Bin. SPEC.md "Deleting".
    pub fn recycle(&self, location: &str) -> Result<()> {
        let path = self.path_of(location);
        recycle(&path)
    }
}

/// The name shown for a recording in messages.
pub fn display_name(r: &Record) -> String {
    let name = r.local_name.clone().unwrap_or_else(|| naming::name_from_plaud(&r.plaud_name));
    let local = clock::to_local(&Stamp::from_unix_millis(r.start_ms));
    format!("{name}, {}", local.name_suffix())
}

/// The base name of a recording from its location: its folder's own name.
pub fn base_of(location: &str) -> &str {
    location.rsplit('\\').next().unwrap_or(location)
}

/// The folder a recording is in, if any, from its location.
pub fn folder_of(location: &str) -> Option<&str> {
    location.rsplit_once('\\').map(|(f, _)| f)
}

fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root).unwrap_or(path).to_string_lossy().replace('/', "\\")
}

fn same_folder(a: &Path, b: &Path) -> bool {
    // Windows names are not case sensitive: renaming only the case of a name
    // is allowed.
    a.to_string_lossy().to_lowercase() == b.to_string_lossy().to_lowercase()
}

fn subfolders(dir: &Path) -> Result<Vec<(String, PathBuf)>> {
    let entries = std::fs::read_dir(dir).map_err(|e| disk("read the folder", dir, &e))?;
    let mut out = Vec::new();
    for e in entries.flatten() {
        if e.file_type().is_ok_and(|t| t.is_dir()) {
            out.push((e.file_name().to_string_lossy().into_owned(), e.path()));
        }
    }
    Ok(out)
}

fn files(dir: &Path) -> Result<Vec<(String, PathBuf)>> {
    let entries = std::fs::read_dir(dir).map_err(|e| disk("read the folder", dir, &e))?;
    let mut out = Vec::new();
    for e in entries.flatten() {
        if e.file_type().is_ok_and(|t| t.is_file()) {
            out.push((e.file_name().to_string_lossy().into_owned(), e.path()));
        }
    }
    Ok(out)
}

fn read_metadata(dir: &Path) -> Option<Metadata> {
    let text = std::fs::read_to_string(dir.join(METADATA)).ok()?;
    serde_json::from_str(&text).ok()
}

fn read_recording(dir: &Path, location: &str) -> Option<Found> {
    let metadata = read_metadata(dir)?;
    let base = base_of(location);
    Some(Found {
        location: location.to_string(),
        has_audio: dir.join(naming::AUDIO.file_name(base)).is_file(),
        has_transcript: dir.join(naming::TRANSCRIPT.file_name(base)).is_file(),
        metadata,
    })
}

fn set_modified(path: &Path, when: std::time::SystemTime, is_dir: bool) -> std::io::Result<()> {
    use std::os::windows::fs::OpenOptionsExt;
    const FILE_WRITE_ATTRIBUTES: u32 = 0x100;
    const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
    let mut opts = std::fs::OpenOptions::new();
    opts.access_mode(FILE_WRITE_ATTRIBUTES);
    if is_dir {
        opts.custom_flags(FILE_FLAG_BACKUP_SEMANTICS);
    }
    let f = opts.open(path)?;
    f.set_times(std::fs::FileTimes::new().set_modified(when))
}

/// Send a file or folder to the Recycle Bin with the shell's own file
/// operation, so Windows' Undo and the Recycle Bin both work.
fn recycle(path: &Path) -> Result<()> {
    use windows::Win32::System::Com::{
        CLSCTX_ALL, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, CoCreateInstance, CoInitializeEx, CoUninitialize,
    };
    use windows::Win32::UI::Shell::{
        FOF_ALLOWUNDO, FOF_NOCONFIRMATION, FOF_NOERRORUI, FOF_SILENT, FOFX_EARLYFAILURE, FOFX_RECYCLEONDELETE, FileOperation,
        IFileOperation, IShellItem, SHCreateItemFromParsingName,
    };
    use windows::core::HSTRING;

    let fail = || Error::Disk(format!(
        "Could not move \"{}\" to the Recycle Bin. If a file in it is open in another program, such as a media player, close it and try again.",
        path.display()
    ));
    // SAFETY: COM is initialised on this thread for the duration of the call
    // and every interface is released before CoUninitialize.
    unsafe {
        let init = CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE);
        let result = (|| -> windows::core::Result<bool> {
            let op: IFileOperation = CoCreateInstance(&FileOperation, None, CLSCTX_ALL)?;
            op.SetOperationFlags(
                FOF_ALLOWUNDO | FOF_NOCONFIRMATION | FOF_SILENT | FOF_NOERRORUI | FOFX_RECYCLEONDELETE | FOFX_EARLYFAILURE,
            )?;
            let item: IShellItem = SHCreateItemFromParsingName(&HSTRING::from(path.as_os_str()), None)?;
            op.DeleteItem(&item, None)?;
            op.PerformOperations()?;
            Ok(op.GetAnyOperationsAborted()?.as_bool())
        })();
        if init.is_ok() {
            CoUninitialize();
        }
        match result {
            Ok(false) if !path.exists() => Ok(()),
            _ => Err(fail()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plaud::Listed;

    struct TempDir(PathBuf);
    impl TempDir {
        fn new(label: &str) -> TempDir {
            let p = std::env::temp_dir().join(format!("notefetch-{label}-{}-{}", std::process::id(), clock::now_utc().unix_millis()));
            let _ = std::fs::remove_dir_all(&p);
            std::fs::create_dir_all(&p).unwrap();
            TempDir(p)
        }
    }
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn start() -> Stamp {
        Stamp::parse("2026-09-24T13:30:00").unwrap()
    }

    fn make(lib: &Library, id: &str, name: &str) -> String {
        let loc = lib.create_recording_folder(id, name, &start()).unwrap();
        lib.write_text(&loc, naming::TRANSCRIPT, "00:00 Hello\r\n").unwrap();
        std::fs::write(lib.file_path(&loc, naming::AUDIO), b"OggS").unwrap();
        let m = Metadata {
            plaud_id: id.into(),
            plaud_name: name.into(),
            local_name: name.into(),
            start: start().iso_utc(),
            duration_ms: 1000,
            parts: BTreeMap::from([("audio".into(), "x".into()), ("transcript".into(), "x".into())]),
        };
        lib.write_metadata(&loc, &m).unwrap();
        loc
    }

    #[test]
    fn folder_and_files_share_the_base_name() {
        let t = TempDir::new("base");
        let lib = Library::new(t.0.clone());
        let loc = make(&lib, "a", "Lecture");
        assert_eq!(loc, "Lecture, 24 Sep 2026, 1430");
        assert!(t.0.join("Lecture, 24 Sep 2026, 1430").join("Transcript, Lecture, 24 Sep 2026, 1430.txt").is_file());
    }

    #[test]
    fn clashes_are_numbered_and_the_same_recording_reuses_its_folder() {
        let t = TempDir::new("clash");
        let lib = Library::new(t.0.clone());
        let a = make(&lib, "a", "Lecture");
        let b = make(&lib, "b", "Lecture");
        assert_eq!(b, "Lecture, 24 Sep 2026, 1430 (2)");
        assert_eq!(lib.create_recording_folder("a", "Lecture", &start()).unwrap(), a);
    }

    #[test]
    fn empty_parts_make_no_file() {
        let t = TempDir::new("empty");
        let lib = Library::new(t.0.clone());
        let loc = lib.create_recording_folder("a", "Lecture", &start()).unwrap();
        assert!(!lib.write_text(&loc, naming::OUTLINE, "  \r\n").unwrap());
        assert!(!lib.file_path(&loc, naming::OUTLINE).exists());
    }

    #[test]
    fn scan_finds_recordings_in_folders_and_notes_deeper_ones() {
        let t = TempDir::new("scan");
        let lib = Library::new(t.0.clone());
        let loc = make(&lib, "a", "Lecture");
        lib.create_folder("Neuroscience").unwrap();
        lib.create_folder("Empty").unwrap();
        let moved = lib.move_to(&loc, Some("Neuroscience")).unwrap();
        assert_eq!(moved, "Neuroscience\\Lecture, 24 Sep 2026, 1430");
        std::fs::create_dir_all(t.0.join("Neuroscience").join("Deeper")).unwrap();
        let scan = lib.scan().unwrap().unwrap();
        assert_eq!(scan.folders, ["Empty", "Neuroscience"]);
        assert_eq!(scan.recordings["a"].location, moved);
        assert!(scan.recordings["a"].has_audio && scan.recordings["a"].has_transcript);
        assert_eq!(scan.too_deep, ["Neuroscience\\Deeper"]);
    }

    #[test]
    fn a_missing_output_folder_is_not_an_empty_one() {
        let lib = Library::new(PathBuf::from(r"Z:\no\such\place"));
        assert!(lib.scan().unwrap().is_none());
    }

    #[test]
    fn reconcile_follows_the_disk() {
        let t = TempDir::new("reconcile");
        let lib = Library::new(t.0.clone());
        let store = Store::in_memory();
        for id in ["a", "b", "c"] {
            store.upsert_listed(&Listed { id: id.into(), name: id.into(), start: start(), duration_ms: 1 }).unwrap();
        }
        let a = make(&lib, "a", "A");
        store.set_state("b", State::Downloaded).unwrap();
        store.set_state("c", State::Deleted).unwrap();
        make(&lib, "c", "C");

        let said = lib.reconcile(&store, &lib.scan().unwrap().unwrap()).unwrap();
        let get = |id| store.get(id).unwrap().unwrap();
        assert_eq!(get("a").state, State::Downloaded, "found on disk");
        assert_eq!(get("a").location.as_deref(), Some(a.as_str()));
        assert_eq!(get("a").status(), "No summary");
        assert_eq!(get("b").state, State::Deleted, "gone from disk");
        assert_eq!(get("c").state, State::Downloaded, "restored from the Recycle Bin");
        assert_eq!(said.len(), 2);
    }

    #[test]
    fn rename_changes_folder_and_files_and_keeps_the_suffix() {
        let t = TempDir::new("rename");
        let lib = Library::new(t.0.clone());
        let loc = make(&lib, "a", "Untitled");
        std::fs::write(t.0.join(&loc).join("notes of mine.txt"), b"x").unwrap();
        let new = lib.rename(&loc, "Neuroscience: week 3", &start()).unwrap();
        assert_eq!(new, "Neuroscience. week 3, 24 Sep 2026, 1430");
        let dir = t.0.join(&new);
        assert!(dir.join("Audio, Neuroscience. week 3, 24 Sep 2026, 1430.ogg").is_file());
        assert!(dir.join("Transcript, Neuroscience. week 3, 24 Sep 2026, 1430.txt").is_file());
        assert!(dir.join("notes of mine.txt").is_file(), "files of the user's are left alone");
        assert_eq!(lib.read_metadata(&new).unwrap().local_name, "Neuroscience. week 3");
    }

    #[test]
    fn rename_is_undone_when_a_file_is_in_use() {
        let t = TempDir::new("rollback");
        let lib = Library::new(t.0.clone());
        let loc = make(&lib, "a", "Lecture");
        // Hold the transcript open without sharing delete access, as an editor
        // might, so Windows refuses to rename it. The audio sorts first, so it
        // is renamed before the refusal and must be put back.
        use std::os::windows::fs::OpenOptionsExt;
        let held = std::fs::OpenOptions::new().read(true).share_mode(1).open(lib.file_path(&loc, naming::TRANSCRIPT)).unwrap();
        assert!(lib.rename(&loc, "New name", &start()).is_err());
        drop(held);
        let dir = t.0.join(&loc);
        assert!(dir.join("Audio, Lecture, 24 Sep 2026, 1430.ogg").is_file(), "the audio was put back");
        assert!(dir.join("Transcript, Lecture, 24 Sep 2026, 1430.txt").is_file());
        assert!(!t.0.join("New name, 24 Sep 2026, 1430").exists());
    }

    #[test]
    fn rename_keeps_a_clash_number() {
        let t = TempDir::new("renameclash");
        let lib = Library::new(t.0.clone());
        make(&lib, "a", "Lecture");
        let b = make(&lib, "b", "Lecture");
        assert_eq!(lib.rename(&b, "Seminar", &start()).unwrap(), "Seminar, 24 Sep 2026, 1430 (2)");
    }

    #[test]
    fn deleting_a_folder_moves_its_recordings_out() {
        let t = TempDir::new("delfolder");
        let lib = Library::new(t.0.clone());
        let loc = make(&lib, "a", "Lecture");
        lib.create_folder("Course").unwrap();
        lib.move_to(&loc, Some("Course")).unwrap();
        let moved = lib.delete_folder("Course").unwrap();
        assert_eq!(moved, [("Course\\Lecture, 24 Sep 2026, 1430".to_string(), "Lecture, 24 Sep 2026, 1430".to_string())]);
        assert!(!t.0.join("Course").exists());
        assert!(t.0.join(&loc).is_dir());
    }

    #[test]
    fn moving_onto_a_same_named_recording_is_refused() {
        let t = TempDir::new("moveclash");
        let lib = Library::new(t.0.clone());
        let loc = make(&lib, "a", "Lecture");
        lib.create_folder("Course").unwrap();
        std::fs::create_dir_all(t.0.join("Course").join(&loc)).unwrap();
        assert!(lib.move_to(&loc, Some("Course")).is_err());
        assert!(t.0.join(&loc).is_dir());
    }

    #[test]
    fn dates_follow_the_recording_start() {
        let t = TempDir::new("dates");
        let lib = Library::new(t.0.clone());
        let loc = make(&lib, "a", "Lecture");
        lib.set_dates(&loc, &start());
        let want = start().system_time();
        let file_time = std::fs::metadata(lib.file_path(&loc, naming::AUDIO)).unwrap().modified().unwrap();
        let dir_time = std::fs::metadata(t.0.join(&loc)).unwrap().modified().unwrap();
        assert_eq!(file_time, want);
        assert_eq!(dir_time, want);
    }

    #[test]
    #[ignore = "touches the real machine: sends a folder to the Recycle Bin"]
    fn recycle_bin() {
        let t = TempDir::new("recycle");
        let lib = Library::new(t.0.clone());
        let loc = make(&lib, "a", "NoteFetch recycle test");
        lib.recycle(&loc).unwrap();
        assert!(!t.0.join(&loc).exists());
    }
}
