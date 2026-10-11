use crate::config::AudioStreamConfig;

/// A segmented chunk of audio emitted by the audio chunker.
#[derive(Debug, Clone, PartialEq)]
pub struct UtteranceChunk {
    pub samples: Vec<f32>,
    pub duration_ms: u64,
    pub is_forced_split: bool,
}

/// Chunks streaming audio based on utterance boundaries (silence timeout)
/// and hard maximum chunk limits (Whisper 30-second context window).
pub struct AudioChunker {
    config: AudioStreamConfig,
    accumulator: Vec<f32>,
    trailing_silence_samples: usize,
    has_detected_speech: bool,
}

impl AudioChunker {
    pub fn new(config: AudioStreamConfig) -> Self {
        Self {
            config,
            accumulator: Vec::new(),
            trailing_silence_samples: 0,
            has_detected_speech: false,
        }
    }

    /// Pushes an incoming audio chunk into the accumulator.
    /// `is_speech` indicates whether the newly pushed chunk contains active speech per VAD.
    /// Returns `Some(UtteranceChunk)` if an utterance ended or the maximum chunk duration was reached.
    pub fn push(&mut self, samples: &[f32], is_speech: bool) -> Option<UtteranceChunk> {
        self.accumulator.extend_from_slice(samples);

        let max_samples =
            (self.config.max_chunk_duration_secs * self.config.target_sample_rate as f32) as usize;
        let silence_timeout_samples =
            (self.config.silence_timeout_secs * self.config.target_sample_rate as f32) as usize;
        let min_utterance_samples =
            (self.config.min_utterance_duration_secs * self.config.target_sample_rate as f32)
                as usize;

        if is_speech {
            self.has_detected_speech = true;
            self.trailing_silence_samples = 0;
        } else if self.has_detected_speech {
            self.trailing_silence_samples += samples.len();
        }

        // 1. Force split on reaching maximum duration ceiling (default 30s)
        if self.accumulator.len() >= max_samples {
            let chunk_samples = std::mem::take(&mut self.accumulator);
            self.has_detected_speech = false;
            self.trailing_silence_samples = 0;
            let duration_ms = (chunk_samples.len() as u64 * 1000)
                / self.config.target_sample_rate as u64;

            return Some(UtteranceChunk {
                samples: chunk_samples,
                duration_ms,
                is_forced_split: true,
            });
        }

        // 2. Utterance completion on silence timeout after speech
        if self.has_detected_speech
            && self.trailing_silence_samples >= silence_timeout_samples
            && self.accumulator.len() >= min_utterance_samples
        {
            let chunk_samples = std::mem::take(&mut self.accumulator);
            self.has_detected_speech = false;
            self.trailing_silence_samples = 0;
            let duration_ms = (chunk_samples.len() as u64 * 1000)
                / self.config.target_sample_rate as u64;

            return Some(UtteranceChunk {
                samples: chunk_samples,
                duration_ms,
                is_forced_split: false,
            });
        }

        None
    }

    /// Flushes accumulated audio immediately (e.g. on Push-to-Talk key release).
    pub fn flush(&mut self) -> Option<UtteranceChunk> {
        if self.accumulator.is_empty() {
            return None;
        }

        let chunk_samples = std::mem::take(&mut self.accumulator);
        let duration_ms =
            (chunk_samples.len() as u64 * 1000) / self.config.target_sample_rate as u64;
        self.has_detected_speech = false;
        self.trailing_silence_samples = 0;

        Some(UtteranceChunk {
            samples: chunk_samples,
            duration_ms,
            is_forced_split: false,
        })
    }

    /// Clears accumulated audio without emitting a chunk.
    pub fn clear(&mut self) {
        self.accumulator.clear();
        self.trailing_silence_samples = 0;
        self.has_detected_speech = false;
    }

    pub fn is_empty(&self) -> bool {
        self.accumulator.is_empty()
    }

    pub fn accumulated_samples(&self) -> usize {
        self.accumulator.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunker_forced_split_at_max_duration() {
        let config = AudioStreamConfig {
            target_sample_rate: 1000,
            max_chunk_duration_secs: 2.0, // 2000 samples
            min_utterance_duration_secs: 0.1,
            silence_timeout_secs: 1.0,
            ..Default::default()
        };
        let mut chunker = AudioChunker::new(config);

        let chunk1 = vec![0.5; 1500];
        assert!(chunker.push(&chunk1, true).is_none());

        let chunk2 = vec![0.5; 600];
        let result = chunker.push(&chunk2, true);
        assert!(result.is_some());
        let emitted = result.unwrap();
        assert!(emitted.is_forced_split);
        assert_eq!(emitted.samples.len(), 2100);
    }

    #[test]
    fn test_chunker_silence_timeout_split() {
        let config = AudioStreamConfig {
            target_sample_rate: 1000,
            max_chunk_duration_secs: 10.0,
            min_utterance_duration_secs: 0.5,
            silence_timeout_secs: 0.3, // 300 samples
            ..Default::default()
        };
        let mut chunker = AudioChunker::new(config);

        // 600 samples of speech
        let speech = vec![0.5; 600];
        assert!(chunker.push(&speech, true).is_none());

        // 350 samples of silence -> triggers utterance end
        let silence = vec![0.0; 350];
        let result = chunker.push(&silence, false);
        assert!(result.is_some());
        let emitted = result.unwrap();
        assert!(!emitted.is_forced_split);
        assert_eq!(emitted.samples.len(), 950);
    }
}
