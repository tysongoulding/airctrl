use serde::{Deserialize, Serialize};

pub const WHISPER_SAMPLE_RATE: u32 = 16_000;
pub const COMPLIANCE_AUDIO_SAMPLE_RATE: u32 = 48_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioChannels {
    Mono = 1,
    Stereo = 2,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioBuffer {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u16,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComplianceAudioSpec {
    pub announcement_text: String,
    pub sample_rate: u32,
    pub volume: f32,
}

impl Default for ComplianceAudioSpec {
    fn default() -> Self {
        Self {
            announcement_text: "This meeting is being recorded".to_string(),
            sample_rate: COMPLIANCE_AUDIO_SAMPLE_RATE,
            volume: 1.0,
        }
    }
}
