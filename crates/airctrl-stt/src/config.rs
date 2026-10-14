use serde::{Deserialize, Serialize};

/// Configuration for Voice Activity Detection (VAD).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VadConfig {
    /// Root-Mean-Square (RMS) amplitude threshold for active speech.
    /// Default: 0.05 (calibrated to pass E2E tests).
    pub energy_threshold: f32,

    /// Decibels relative to Full Scale (dBFS) threshold.
    /// Default: -45.0 dBFS.
    pub dbfs_threshold: f32,

    /// Minimum number of samples required for a valid speech buffer.
    /// Buffers below this are discarded as accidental micro-presses.
    /// Default: 1600 samples (100ms at 16,000 Hz).
    pub min_samples: usize,

    /// Frame window size in samples for sliding energy analysis.
    /// Default: 320 samples (20ms at 16,000 Hz).
    pub frame_size: usize,

    /// Hop size in samples between consecutive analysis frames.
    /// Default: 160 samples (10ms at 16,000 Hz).
    pub hop_size: usize,

    /// Pre-roll padding samples retained before speech onset to preserve plosive consonants.
    /// Default: 800 samples (50ms at 16,000 Hz).
    pub pre_roll_samples: usize,

    /// Post-roll padding samples retained after speech offset to preserve trailing phonemes.
    /// Default: 1600 samples (100ms at 16,000 Hz).
    pub post_roll_samples: usize,

    /// Hangover duration in milliseconds to bridge inter-word pauses and prevent chatter.
    /// Default: 300 ms.
    pub hangover_duration_ms: u32,

    /// Trailing silence duration in milliseconds to trigger automatic end-of-utterance chunking.
    /// Default: 600 ms.
    pub speech_end_silence_ms: u32,

    /// Minimum duration in samples required to consider an utterance complete for command execution.
    /// Default: 8000 samples (500ms at 16,000 Hz).
    pub min_utterance_samples: usize,
}

impl Default for VadConfig {
    fn default() -> Self {
        Self {
            energy_threshold: 0.05,
            dbfs_threshold: -45.0,
            min_samples: 1600,
            frame_size: 320,
            hop_size: 160,
            pre_roll_samples: 800,
            post_roll_samples: 1600,
            hangover_duration_ms: 300,
            speech_end_silence_ms: 600,
            min_utterance_samples: 8000,
        }
    }
}

/// Configuration for real-time audio capture, normalization, and ring buffering.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioStreamConfig {
    /// Target sample rate for Whisper inference (16,000 Hz).
    pub target_sample_rate: u32,

    /// Target channel count (1 = mono).
    pub target_channels: u16,

    /// Capacity of the live audio circular buffer in samples.
    /// Default: 160,000 samples (10 seconds at 16,000 Hz).
    pub ring_buffer_capacity: usize,

    /// Maximum chunk duration in seconds before forcing a chunk split.
    /// Default: 30.0 seconds (Whisper 30s context window limit).
    pub max_chunk_duration_secs: f32,

    /// Minimum utterance duration in seconds.
    /// Default: 0.5 seconds (500ms).
    pub min_utterance_duration_secs: f32,

    /// Silence timeout in seconds after speech to trigger end-of-utterance emission.
    /// Default: 0.6 seconds (600ms).
    pub silence_timeout_secs: f32,
}

impl Default for AudioStreamConfig {
    fn default() -> Self {
        Self {
            target_sample_rate: 16_000,
            target_channels: 1,
            ring_buffer_capacity: 160_000,
            max_chunk_duration_secs: 30.0,
            min_utterance_duration_secs: 0.5,
            silence_timeout_secs: 0.6,
        }
    }
}

/// Configuration for active window text injection and Reliable Paste.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextInjectionConfig {
    /// Text lengths exceeding this threshold immediately bypass typing and use clipboard paste.
    /// Default: 200 characters.
    pub clipboard_fallback_threshold: usize,

    /// Delay after refocusing window to allow OS message loop to update focus (milliseconds).
    /// Default: 30 ms.
    pub refocus_settle_delay_ms: u64,

    /// Whether Reliable Paste is enabled (preserves and restores clipboard content).
    /// Default: true (Handy default).
    pub reliable_paste: bool,

    /// Delay after target window focus before typing or pasting (milliseconds).
    /// Allows window message loop and caret to settle.
    /// Default: 30 ms (Handy default).
    pub paste_delay_before_ms: u64,

    /// Delay after paste keystroke before restoring original clipboard content (milliseconds).
    /// Ensures target application has fully consumed clipboard contents.
    /// Default: 50 ms (Handy default).
    pub paste_delay_after_ms: u64,
}

impl Default for TextInjectionConfig {
    fn default() -> Self {
        Self {
            clipboard_fallback_threshold: 200,
            refocus_settle_delay_ms: 30,
            reliable_paste: true,
            paste_delay_before_ms: 30,
            paste_delay_after_ms: 50,
        }
    }
}

/// Configuration for hotkey push-to-talk and toggle triggers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HotkeyConfig {
    /// Minimum press duration in milliseconds required to trigger speech transcription.
    /// Presses shorter than this are treated as accidental micro-presses and discarded.
    /// Default: 100 ms.
    pub min_press_duration_ms: u64,
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        Self {
            min_press_duration_ms: 100,
        }
    }
}
