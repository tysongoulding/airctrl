use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use crate::error::{MeetingError, Result};
use super::entry::MeetingArchiveEntry;

pub struct MeetingArchiveStore {
    entries: RwLock<Vec<MeetingArchiveEntry>>,
    lookup: RwLock<HashMap<String, usize>>,
    base_dir: Option<PathBuf>,
}

impl Default for MeetingArchiveStore {
    fn default() -> Self {
        Self::new(None)
    }
}

impl MeetingArchiveStore {
    pub fn new(base_dir: Option<PathBuf>) -> Self {
        let store = Self {
            entries: RwLock::new(Vec::new()),
            lookup: RwLock::new(HashMap::new()),
            base_dir,
        };

        if let Some(ref dir) = store.base_dir {
            let _ = store.load_and_reconcile(dir);
        }

        store
    }

    pub fn save_meeting(&self, mut entry: MeetingArchiveEntry) -> Result<()> {
        entry.updated_at = chrono::Utc::now();

        let mut entries = self.entries.write().map_err(|e| {
            MeetingError::ArchiveError(format!("RwLock poisoned: {e}"))
        })?;
        let mut lookup = self.lookup.write().map_err(|e| {
            MeetingError::ArchiveError(format!("RwLock poisoned: {e}"))
        })?;

        if let Some(&idx) = lookup.get(&entry.id) {
            entries[idx] = entry.clone();
        } else {
            let idx = entries.len();
            lookup.insert(entry.id.clone(), idx);
            entries.push(entry.clone());
        }

        if let Some(ref dir) = self.base_dir {
            self.persist_meeting_to_disk(dir, &entry)?;
            self.persist_index_to_disk(dir, &entries)?;
        }

        Ok(())
    }

    pub fn get_meeting(&self, id: &str) -> Option<MeetingArchiveEntry> {
        let entries = self.entries.read().ok()?;
        let lookup = self.lookup.read().ok()?;
        lookup.get(id).and_then(|&idx| entries.get(idx).cloned())
    }

    pub fn list_meetings(&self) -> Vec<MeetingArchiveEntry> {
        self.entries
            .read()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }

    pub fn delete_meeting(&self, id: &str) -> Result<bool> {
        let mut entries = self.entries.write().map_err(|e| {
            MeetingError::ArchiveError(format!("RwLock poisoned: {e}"))
        })?;
        let mut lookup = self.lookup.write().map_err(|e| {
            MeetingError::ArchiveError(format!("RwLock poisoned: {e}"))
        })?;

        if let Some(&idx) = lookup.get(id) {
            entries.remove(idx);
            lookup.clear();
            for (i, e) in entries.iter().enumerate() {
                lookup.insert(e.id.clone(), i);
            }

            if let Some(ref dir) = self.base_dir {
                let meeting_dir = dir.join("meetings").join(id);
                let _ = fs::remove_dir_all(meeting_dir);
                self.persist_index_to_disk(dir, &entries)?;
            }

            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn clear(&self) -> Result<()> {
        let mut entries = self.entries.write().map_err(|e| {
            MeetingError::ArchiveError(format!("RwLock poisoned: {e}"))
        })?;
        let mut lookup = self.lookup.write().map_err(|e| {
            MeetingError::ArchiveError(format!("RwLock poisoned: {e}"))
        })?;

        entries.clear();
        lookup.clear();

        if let Some(ref dir) = self.base_dir {
            let meetings_dir = dir.join("meetings");
            let _ = fs::remove_dir_all(&meetings_dir);
            let _ = fs::create_dir_all(&meetings_dir);
            self.persist_index_to_disk(dir, &entries)?;
        }

        Ok(())
    }

    // --- On-Disk Crash-Consistency and Reconciliation ---

    fn persist_meeting_to_disk(&self, base_dir: &Path, entry: &MeetingArchiveEntry) -> Result<()> {
        let meeting_dir = base_dir.join("meetings").join(&entry.id);
        fs::create_dir_all(&meeting_dir).map_err(|e| {
            MeetingError::ArchiveError(format!("Failed to create meeting dir: {e}"))
        })?;

        let target_file = meeting_dir.join("entry.json");
        Self::atomic_write_json(&target_file, entry)
    }

    fn persist_index_to_disk(&self, base_dir: &Path, entries: &[MeetingArchiveEntry]) -> Result<()> {
        fs::create_dir_all(base_dir).map_err(|e| {
            MeetingError::ArchiveError(format!("Failed to create base archive dir: {e}"))
        })?;

        let target_file = base_dir.join("index.json");
        Self::atomic_write_json(&target_file, entries)
    }

    fn atomic_write_json<T: serde::Serialize + ?Sized>(target_path: &Path, value: &T) -> Result<()> {
        let parent = target_path.parent().ok_or_else(|| {
            MeetingError::ArchiveError("Invalid target path without parent".into())
        })?;

        let tmp_file_name = format!(
            "{}.tmp.{}",
            target_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("file"),
            uuid::Uuid::new_v4().simple()
        );
        let tmp_path = parent.join(tmp_file_name);

        {
            let file = File::create(&tmp_path).map_err(|e| {
                MeetingError::ArchiveError(format!("Failed to create temp file: {e}"))
            })?;
            let mut writer = BufWriter::new(file);

            serde_json::to_writer_pretty(&mut writer, value).map_err(|e| {
                let _ = fs::remove_file(&tmp_path);
                MeetingError::ArchiveError(format!("JSON serialization failed: {e}"))
            })?;

            writer.flush().map_err(|e| {
                let _ = fs::remove_file(&tmp_path);
                MeetingError::ArchiveError(format!("Buffer flush failed: {e}"))
            })?;

            let inner_file = writer.into_inner().map_err(|e| {
                let _ = fs::remove_file(&tmp_path);
                MeetingError::ArchiveError(format!("Failed to extract file handle: {e}"))
            })?;

            inner_file.sync_all().map_err(|e| {
                let _ = fs::remove_file(&tmp_path);
                MeetingError::ArchiveError(format!("fsync failed: {e}"))
            })?;
        }

        fs::rename(&tmp_path, target_path).map_err(|e| {
            let _ = fs::remove_file(&tmp_path);
            MeetingError::ArchiveError(format!("Atomic rename failed: {e}"))
        })?;

        Ok(())
    }

    fn load_and_reconcile(&self, dir: &Path) -> Result<()> {
        let meetings_dir = dir.join("meetings");
        fs::create_dir_all(&meetings_dir).map_err(|e| {
            MeetingError::ArchiveError(format!("Failed to create meetings dir: {e}"))
        })?;

        // 1. Clean up orphaned .tmp files left from prior crashes
        Self::cleanup_temp_files_recursive(dir);

        // 2. Load existing index.json if present
        let index_file = dir.join("index.json");
        let mut loaded_entries: Vec<MeetingArchiveEntry> = Vec::new();
        if index_file.exists() {
            if let Ok(file) = File::open(&index_file) {
                let reader = BufReader::new(file);
                if let Ok(indexed) = serde_json::from_reader::<_, Vec<MeetingArchiveEntry>>(reader) {
                    loaded_entries = indexed;
                }
            }
        }

        let mut known_ids: HashMap<String, usize> = loaded_entries
            .iter()
            .enumerate()
            .map(|(i, e)| (e.id.clone(), i))
            .collect();

        // 3. Scan meetings subdirectories for unindexed entries (self-healing reconciliation)
        let mut reconciled_any = false;
        if let Ok(entries) = fs::read_dir(&meetings_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let entry_json = path.join("entry.json");
                    if entry_json.exists() {
                        if let Ok(file) = File::open(&entry_json) {
                            let reader = BufReader::new(file);
                            if let Ok(meeting) = serde_json::from_reader::<_, MeetingArchiveEntry>(reader) {
                                if let Some(&idx) = known_ids.get(&meeting.id) {
                                    loaded_entries[idx] = meeting;
                                } else {
                                    let new_idx = loaded_entries.len();
                                    known_ids.insert(meeting.id.clone(), new_idx);
                                    loaded_entries.push(meeting);
                                    reconciled_any = true;
                                }
                            }
                        }
                    }
                }
            }
        }

        // If self-healing discovered new entries, update index.json
        if reconciled_any {
            let _ = self.persist_index_to_disk(dir, &loaded_entries);
        }

        let mut entries_guard = self.entries.write().map_err(|e| {
            MeetingError::ArchiveError(format!("RwLock poisoned: {e}"))
        })?;
        let mut lookup_guard = self.lookup.write().map_err(|e| {
            MeetingError::ArchiveError(format!("RwLock poisoned: {e}"))
        })?;

        *entries_guard = loaded_entries;
        *lookup_guard = known_ids;

        Ok(())
    }

    fn cleanup_temp_files_recursive(dir: &Path) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    Self::cleanup_temp_files_recursive(&path);
                } else if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.contains(".tmp.") {
                        let _ = fs::remove_file(&path);
                    }
                }
            }
        }
    }
}
