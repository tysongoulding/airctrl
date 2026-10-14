use std::collections::HashMap;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
pub use crate::types::LiveTranscriptChunk;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneratedSummaryRecord {
    pub template_id: String,
    pub template_name: String,
    pub provider: String,
    pub model: String,
    pub content: String,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeetingArchiveEntry {
    pub id: String,
    pub title: String,
    #[serde(alias = "start_time")]
    pub date_iso: String,
    #[serde(alias = "duration_seconds")]
    pub duration_secs: u64,
    #[serde(alias = "transcript")]
    pub transcript_chunks: Vec<LiveTranscriptChunk>,
    #[serde(default)]
    pub summary_markdown: String,
    #[serde(default, alias = "audio_file_path")]
    pub audio_path: String,
    #[serde(default)]
    pub participants: Vec<String>,
    #[serde(default)]
    pub platform: Option<String>,
    #[serde(default)]
    pub summaries: HashMap<String, GeneratedSummaryRecord>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub notes: String,
    #[serde(default = "Utc::now")]
    pub created_at: DateTime<Utc>,
    #[serde(default = "Utc::now")]
    pub updated_at: DateTime<Utc>,
}

impl MeetingArchiveEntry {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        date_iso: impl Into<String>,
        duration_secs: u64,
        transcript_chunks: Vec<LiveTranscriptChunk>,
        summary_markdown: impl Into<String>,
        audio_path: impl Into<String>,
        participants: Vec<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: id.into(),
            title: title.into(),
            date_iso: date_iso.into(),
            duration_secs,
            transcript_chunks,
            summary_markdown: summary_markdown.into(),
            audio_path: audio_path.into(),
            participants,
            platform: None,
            summaries: HashMap::new(),
            tags: Vec::new(),
            notes: String::new(),
            created_at: now,
            updated_at: now,
        }
    }
}
