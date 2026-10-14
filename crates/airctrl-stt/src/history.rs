use crate::error::{Result, SttError};
use crate::types::TranscriptItem;
use parking_lot::RwLock;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};

/// Thread-safe local transcript history store with optional crash-safe on-disk persistence.
pub struct TranscriptHistoryStore {
    items: RwLock<Vec<TranscriptItem>>,
    file_path: Option<PathBuf>,
}

impl Default for TranscriptHistoryStore {
    fn default() -> Self {
        Self::new(None)
    }
}

impl TranscriptHistoryStore {
    pub fn new(file_path: Option<PathBuf>) -> Self {
        let items = if let Some(path) = &file_path {
            if path.exists() {
                Self::load_from_disk(path).unwrap_or_default()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        Self {
            items: RwLock::new(items),
            file_path,
        }
    }

    fn load_from_disk(path: &Path) -> Result<Vec<TranscriptItem>> {
        let file = File::open(path).map_err(|e| SttError::InferenceError(e.to_string()))?;
        let reader = BufReader::new(file);
        let items: Vec<TranscriptItem> = serde_json::from_reader(reader)
            .map_err(|e| SttError::InferenceError(e.to_string()))?;
        Ok(items)
    }

    /// Stores a new transcript item with atomic crash-safe file persistence.
    pub fn store(&self, item: TranscriptItem) -> Result<()> {
        let mut guard = self.items.write();
        guard.push(item);

        if let Some(path) = &self.file_path {
            self.persist_atomically(path, &guard)?;
        }
        Ok(())
    }

    /// Searches transcripts by query substring and optional model filter, returning
    /// results in reverse-chronological order (most recent first).
    pub fn search(&self, query: &str, model_filter: Option<&str>, limit: usize) -> Vec<TranscriptItem> {
        let q = query.to_lowercase().trim().to_string();
        let guard = self.items.read();
        let mut results = Vec::new();

        for item in guard.iter().rev() {
            if let Some(model) = model_filter {
                if !model.is_empty() && !item.model_used.to_lowercase().contains(&model.to_lowercase()) {
                    continue;
                }
            }

            if q.is_empty()
                || item.text.to_lowercase().contains(&q)
                || item.raw_text.to_lowercase().contains(&q)
                || item.target_app.as_deref().unwrap_or("").to_lowercase().contains(&q)
                || item.target_title.as_deref().unwrap_or("").to_lowercase().contains(&q)
            {
                results.push(item.clone());
            }

            if results.len() >= limit {
                break;
            }
        }

        results
    }

    /// Returns a copy of all transcript items in reverse chronological order.
    pub fn all(&self) -> Vec<TranscriptItem> {
        self.items.read().iter().rev().cloned().collect()
    }

    /// Returns the total number of stored transcripts.
    pub fn count(&self) -> usize {
        self.items.read().len()
    }

    /// Clears all stored transcripts.
    pub fn clear(&self) -> Result<()> {
        let mut guard = self.items.write();
        guard.clear();
        if let Some(path) = &self.file_path {
            self.persist_atomically(path, &guard)?;
        }
        Ok(())
    }

    fn persist_atomically(&self, target_path: &Path, items: &[TranscriptItem]) -> Result<()> {
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent).map_err(|e| SttError::InferenceError(e.to_string()))?;
        }
        let temp_path = target_path.with_extension("tmp");
        {
            let file = File::create(&temp_path).map_err(|e| SttError::InferenceError(e.to_string()))?;
            let writer = BufWriter::new(file);
            serde_json::to_writer_pretty(writer, items)
                .map_err(|e| SttError::InferenceError(e.to_string()))?;
        }
        fs::rename(&temp_path, target_path)
            .map_err(|e| SttError::InferenceError(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transcript_history_store_and_search() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("transcripts.json");
        let store = TranscriptHistoryStore::new(Some(path.clone()));

        let item1 = TranscriptItem {
            id: "1".into(),
            timestamp: 1000.0,
            text: "first standup meeting".into(),
            raw_text: "first standup meeting".into(),
            duration_ms: 1200,
            model_used: "whisper-tiny.en".into(),
            confidence: 0.95,
            session_id: "s1".into(),
            target_app: Some("slack".into()),
            target_title: None,
            injected: true,
            starred: false,
        };

        let item2 = TranscriptItem {
            id: "2".into(),
            timestamp: 2000.0,
            text: "second review session".into(),
            raw_text: "second review session".into(),
            duration_ms: 2500,
            model_used: "whisper-small.en".into(),
            confidence: 0.98,
            session_id: "s1".into(),
            target_app: Some("code".into()),
            target_title: None,
            injected: true,
            starred: true,
        };

        store.store(item1).unwrap();
        store.store(item2).unwrap();

        assert_eq!(store.count(), 2);

        // Search query
        let results = store.search("standup", None, 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "1");

        // Model filter
        let results = store.search("", Some("small"), 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "2");

        // Reverse chronological order
        let all = store.search("", None, 10);
        assert_eq!(all[0].id, "2");
        assert_eq!(all[1].id, "1");

        // Reload from disk
        let reloaded = TranscriptHistoryStore::new(Some(path));
        assert_eq!(reloaded.count(), 2);

        // Clear
        reloaded.clear().unwrap();
        assert_eq!(reloaded.count(), 0);
    }
}
