use std::path::Path;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use airctrl_core::types::meeting::MeetingTranscriptSegment;
use airctrl_stt::SpeechToTextEngine;
use crate::error::Result;
use super::decoder::AudioDecoder;
use super::preprocessor::{downmix_to_mono, resample_to_16k};
use super::transcriber::ChunkedTranscriber;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportOptions {
    pub model: Option<String>,
    pub language: Option<String>,
    pub title: Option<String>,
}

impl Default for ImportOptions {
    fn default() -> Self {
        Self {
            model: Some("whisper-base.en".to_string()),
            language: None,
            title: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportAndTranscribeResult {
    pub source_path: String,
    pub format: String,
    pub model_used: String,
    pub transcript: String,
    pub duration_secs: u64,
    pub sample_rate: u32,
    pub segments: Vec<MeetingTranscriptSegment>,
}

pub struct AudioImportService {
    engine: Arc<dyn SpeechToTextEngine + Send + Sync>,
}

impl AudioImportService {
    pub fn new(engine: Arc<dyn SpeechToTextEngine + Send + Sync>) -> Self {
        Self { engine }
    }

    pub fn import_and_transcribe(
        &self,
        file_path: &Path,
        options: ImportOptions,
        progress_cb: Option<Arc<dyn Fn(f32) + Send + Sync>>,
    ) -> Result<ImportAndTranscribeResult> {
        // 1. Decode audio container
        let decoded = AudioDecoder::decode_file(file_path)?;

        // 2. Downmix multi-channel to mono
        let mono = downmix_to_mono(&decoded.samples, decoded.channels);

        // 3. Resample to 16,000 Hz
        let resampled = resample_to_16k(&mono, decoded.sample_rate);

        // 4. Chunked re-transcription
        let meeting_id = format!("import_{}", uuid::Uuid::new_v4().simple());
        let (segments, full_text) = ChunkedTranscriber::transcribe_stream(
            &resampled,
            &meeting_id,
            self.engine.as_ref(),
            progress_cb,
        )?;

        let model_used = options
            .model
            .or_else(|| {
                self.engine
                    .model_info()
                    .map(|m| format!("{:?}", m.variant))
            })
            .unwrap_or_else(|| "whisper-base.en".to_string());

        let transcript = full_text;

        Ok(ImportAndTranscribeResult {
            source_path: file_path.display().to_string(),
            format: decoded.format,
            model_used,
            transcript,
            duration_secs: decoded.duration_secs,
            sample_rate: 16_000,
            segments,
        })
    }
}
