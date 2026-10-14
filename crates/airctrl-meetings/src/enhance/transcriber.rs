use std::sync::Arc;
use airctrl_core::types::meeting::MeetingTranscriptSegment;
use airctrl_stt::SpeechToTextEngine;
use crate::error::{MeetingError, Result};

pub struct ChunkedTranscriber;

impl ChunkedTranscriber {
    /// Transcribes a 16kHz mono audio buffer in 30-second windows with progress reporting.
    pub fn transcribe_stream(
        samples: &[f32],
        meeting_id: &str,
        engine: &dyn SpeechToTextEngine,
        progress_cb: Option<Arc<dyn Fn(f32) + Send + Sync>>,
    ) -> Result<(Vec<MeetingTranscriptSegment>, String)> {
        if samples.is_empty() {
            return Ok((Vec::new(), String::new()));
        }

        let chunk_size = 30 * 16_000; // 30-second chunks at 16kHz = 480,000 samples
        let total_chunks = samples.len().div_ceil(chunk_size);
        let mut segments = Vec::with_capacity(total_chunks);
        let mut full_text_parts = Vec::with_capacity(total_chunks);

        for chunk_idx in 0..total_chunks {
            let start_sample = chunk_idx * chunk_size;
            let end_sample = (start_sample + chunk_size).min(samples.len());
            let chunk_slice = &samples[start_sample..end_sample];

            let start_ms = ((start_sample as f64 / 16_000.0) * 1000.0) as u64;
            let duration_ms = ((chunk_slice.len() as f64 / 16_000.0) * 1000.0) as u64;
            let end_ms = start_ms + duration_ms;

            let stt_res = engine.transcribe(chunk_slice).map_err(|e| {
                MeetingError::AudioCaptureFailed(format!("Offline STT transcription failed: {e}"))
            })?;

            let text = stt_res.text.trim().to_string();
            if !text.is_empty() {
                full_text_parts.push(text.clone());
            }

            let segment = MeetingTranscriptSegment {
                id: format!("seg_{}_{}", meeting_id, chunk_idx + 1),
                meeting_id: meeting_id.to_string(),
                start_ms,
                end_ms,
                speaker: "Speaker".to_string(),
                text,
                confidence: stt_res.confidence,
                is_final: true,
            };

            segments.push(segment);

            if let Some(ref cb) = progress_cb {
                let progress = ((chunk_idx + 1) as f32) / (total_chunks as f32);
                cb(progress.clamp(0.0, 1.0));
            }
        }

        let full_text = full_text_parts.join(" ");
        Ok((segments, full_text))
    }
}
