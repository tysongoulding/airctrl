use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::fmt;
use std::str::FromStr;

/// Supported binary formats for local speech-to-text model weights.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModelFormat {
    #[serde(rename = "ggml_bin")]
    GgmlBin,
    #[serde(rename = "gguf")]
    Gguf,
}

impl ModelFormat {
    /// Resolves model format from a file extension.
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_ascii_lowercase().as_str() {
            "bin" => Some(Self::GgmlBin),
            "gguf" => Some(Self::Gguf),
            _ => None,
        }
    }

    /// Returns the canonical extension string.
    pub fn extension(&self) -> &'static str {
        match self {
            Self::GgmlBin => "bin",
            Self::Gguf => "gguf",
        }
    }
}

/// Known variants of speech recognition models supported for offline transcription.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WhisperModelVariant {
    #[serde(rename = "tiny.en")]
    TinyEn,
    #[serde(rename = "base.en")]
    BaseEn,
    #[serde(rename = "small.en")]
    SmallEn,
    #[serde(rename = "medium.en")]
    MediumEn,
    #[serde(rename = "turbo.en")]
    TurboEn,
    #[serde(rename = "large.en")]
    LargeEn,
    #[serde(rename = "parakeet-v3")]
    ParakeetV3,
    #[serde(rename = "custom")]
    Custom(String),
}

impl WhisperModelVariant {
    /// Returns the standard filename for standard variants on disk.
    pub fn default_filename(&self) -> &'static str {
        match self {
            Self::TinyEn => "whisper-tiny.en.bin",
            Self::BaseEn => "whisper-base.en.bin",
            Self::SmallEn => "whisper-small.en.bin",
            Self::MediumEn => "whisper-medium.en.bin",
            Self::TurboEn => "whisper-turbo.en.bin",
            Self::LargeEn => "whisper-large.en.bin",
            Self::ParakeetV3 => "parakeet-unified-en-0.6b-Q8_0.gguf",
            Self::Custom(_) => "custom-model.bin",
        }
    }

    /// Returns the filename as a Cow string, preserving custom names.
    pub fn filename(&self) -> Cow<'_, str> {
        match self {
            Self::Custom(name) => Cow::Borrowed(name.as_str()),
            _ => Cow::Borrowed(self.default_filename()),
        }
    }

    /// Approximate size in bytes for model weights.
    pub fn approximate_size_bytes(&self) -> u64 {
        match self {
            Self::TinyEn => 77_700_000,
            Self::BaseEn => 142_000_000,
            Self::SmallEn => 488_000_000,
            Self::MediumEn => 1_530_000_000,
            Self::TurboEn => 1_620_000_000,
            Self::LargeEn => 3_090_000_000,
            Self::ParakeetV3 => 640_000_000,
            Self::Custom(_) => 0,
        }
    }

    /// Known official SHA-256 checksum for official release weights.
    pub fn official_sha256(&self) -> Option<&'static str> {
        match self {
            Self::TinyEn => Some("bd577a113a8644458634039ac77fb8d66a33c02c582b240d84a5ffc6c6bfaeee"),
            Self::BaseEn => Some("137c40409d784d83bd7a8f62c08ea2773d700f83c63dec438e149a3775f970aa"),
            Self::SmallEn => Some("553563b735b81045070293de58431f471272429d9361da2a612da40365a788fa"),
            Self::MediumEn => Some("fd9727b6e1217c2f614f9b698455c4777609895ac90479d20c574044566580f4"),
            Self::TurboEn => Some("271d53aa4b6113b28b61c7784f15dff72ba8985fd0521e11e858db41f02ec3b7"),
            Self::LargeEn => Some("ad82bf6a9043c0fba7b8f0adb4567d1be14b679460342371c67d1656a84f33b1"),
            Self::ParakeetV3 => Some("c3840e698888b50f75e2f759c5d14df5a9a46f254e47a5f6e80b435ff2ebbe14"),
            Self::Custom(_) => None,
        }
    }

    /// Returns whether this model is GGUF format.
    pub fn is_gguf(&self) -> bool {
        matches!(self, Self::ParakeetV3)
            || matches!(self, Self::Custom(s) if s.to_ascii_lowercase().ends_with(".gguf"))
    }

    /// Returns whether this model is a Whisper architecture model.
    pub fn is_whisper(&self) -> bool {
        !matches!(self, Self::ParakeetV3)
    }

    /// Returns the model binary format.
    pub fn format(&self) -> ModelFormat {
        if self.is_gguf() {
            ModelFormat::Gguf
        } else {
            ModelFormat::GgmlBin
        }
    }

    /// Identifies model variant from filename heuristics.
    pub fn from_filename(name: &str) -> Self {
        let lower = name.to_ascii_lowercase();
        if lower.contains("parakeet") {
            Self::ParakeetV3
        } else if lower.contains("turbo") || lower.contains("large-v3-turbo") {
            Self::TurboEn
        } else if lower.contains("large-v3") || lower.contains("large.en") || lower.contains("whisper-large") {
            Self::LargeEn
        } else if lower.contains("medium.en") || lower.contains("ggml-medium") {
            Self::MediumEn
        } else if lower.contains("small.en") || lower.contains("ggml-small") {
            Self::SmallEn
        } else if lower.contains("base.en") || lower.contains("ggml-base") {
            Self::BaseEn
        } else if lower.contains("tiny.en") || lower.contains("ggml-tiny") {
            Self::TinyEn
        } else {
            Self::Custom(name.to_string())
        }
    }

    /// Returns all standard predefined variants for testing and iteration.
    pub fn standard_variants() -> &'static [Self] {
        &[
            Self::TinyEn,
            Self::BaseEn,
            Self::SmallEn,
            Self::MediumEn,
            Self::TurboEn,
            Self::LargeEn,
            Self::ParakeetV3,
        ]
    }
}

/// Recording and hotkey dispatch modes matching Handy 1:1 specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum HotkeyMode {
    /// Push-to-Talk (Held while speaking, stops on release).
    PushToTalk,

    /// Toggle (First tap starts, second tap stops).
    Toggle,

    /// Smart Dual-Mode: <30ms jitter rejection, 30-249ms toggle latch, >=250ms PTT.
    SmartDual,

    /// Strictly push-to-talk (canonical M8 Handy name).
    HoldOnly,

    /// Strictly tap-to-start, tap-to-stop (canonical M8 Handy name).
    ToggleOnly,

    /// Canonical M8 Handy Smart Dual-Mode.
    #[default]
    SmartDualMode,

    /// Hands-free background VAD mode (speech auto-detected without hotkey).
    ContinuousHandsFree,
}

/// Type alias for domain clarity.
pub type RecordingMode = HotkeyMode;

/// 4-State Floating Pill Overlay Placement conforming to Handy specifications.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum OverlayPosition {
    /// Positioned horizontally centered, 40 physical pixels above active taskbar/dock.
    #[default]
    Bottom,
    /// Positioned horizontally centered, 20 physical pixels below top screen boundary.
    Top,
    /// Positioned dead-center within active display work area.
    Center,
    /// Completely invisible/unmapped overlay window for guaranteed zero focus stealing.
    None,
}

impl fmt::Display for OverlayPosition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bottom => write!(f, "Bottom"),
            Self::Top => write!(f, "Top"),
            Self::Center => write!(f, "Center"),
            Self::None => write!(f, "None"),
        }
    }
}

impl FromStr for OverlayPosition {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "bottom" => Ok(Self::Bottom),
            "top" => Ok(Self::Top),
            "center" => Ok(Self::Center),
            "none" => Ok(Self::None),
            _ => Err(format!(
                "Invalid overlay position '{}'. Expected Bottom, Top, Center, or None",
                s
            )),
        }
    }
}

/// Domain model for an individual transcript entry stored in local history.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TranscriptItem {
    pub id: String,
    pub timestamp: f64,               // Seconds since Unix epoch
    pub text: String,                 // Final dictionary-replaced & post-processed text
    pub raw_text: String,             // Original verbatim Whisper/Parakeet text
    pub duration_ms: u64,             // Speech audio duration in ms
    pub model_used: String,           // Model variant name
    pub confidence: f32,              // 0.00 to 1.00
    pub session_id: String,           // Grouping session identifier (default: "default_session")
    pub target_app: Option<String>,   // Injected application executable name
    pub target_title: Option<String>, // Active window title
    pub injected: bool,               // Whether active window text injection succeeded
    pub starred: bool,                // User bookmark flag
}

/// Types of synthesized acoustic audio feedback cues.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioCueType {
    RecordingStart,
    RecordingStop,
    Cancel,
    Error,
}

/// Serializable dictionary replacement rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DictionaryRule {
    pub trigger: String,
    pub replacement: String,
    pub is_regex: bool,
    pub preserve_case: bool,
}

impl DictionaryRule {
    pub fn literal(trigger: impl Into<String>, replacement: impl Into<String>) -> Self {
        Self {
            trigger: trigger.into(),
            replacement: replacement.into(),
            is_regex: false,
            preserve_case: true,
        }
    }

    pub fn regex(pattern: impl Into<String>, replacement: impl Into<String>) -> Self {
        Self {
            trigger: pattern.into(),
            replacement: replacement.into(),
            is_regex: true,
            preserve_case: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SttState {
    Idle,
    Recording,
    Processing,
    Injecting,
    Error,
}

/// High-level speech recognition state machine states matching M2 specification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpeechState {
    Idle,
    Listening,
    Transcribing,
    Injecting,
    Error(String),
}

impl From<SpeechState> for SttState {
    fn from(state: SpeechState) -> Self {
        match state {
            SpeechState::Idle => SttState::Idle,
            SpeechState::Listening => SttState::Recording,
            SpeechState::Transcribing => SttState::Processing,
            SpeechState::Injecting => SttState::Injecting,
            SpeechState::Error(_) => SttState::Error,
        }
    }
}

impl From<SttState> for SpeechState {
    fn from(state: SttState) -> Self {
        match state {
            SttState::Idle => SpeechState::Idle,
            SttState::Recording => SpeechState::Listening,
            SttState::Processing => SpeechState::Transcribing,
            SttState::Injecting => SpeechState::Injecting,
            SttState::Error => SpeechState::Error("STT Pipeline Error".to_string()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TranscriptionPayload {
    pub text: String,
    pub duration_ms: u64,
    pub confidence: f32,
    pub target_window_title: Option<String>,
    pub target_process_name: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_speech_state_bidirectional_conversion() {
        assert_eq!(SttState::from(SpeechState::Idle), SttState::Idle);
        assert_eq!(SttState::from(SpeechState::Listening), SttState::Recording);
        assert_eq!(SttState::from(SpeechState::Transcribing), SttState::Processing);
        assert_eq!(SttState::from(SpeechState::Injecting), SttState::Injecting);
        assert_eq!(SttState::from(SpeechState::Error("fail".into())), SttState::Error);

        assert_eq!(SpeechState::from(SttState::Idle), SpeechState::Idle);
        assert_eq!(SpeechState::from(SttState::Recording), SpeechState::Listening);
        assert_eq!(SpeechState::from(SttState::Processing), SpeechState::Transcribing);
        assert_eq!(SpeechState::from(SttState::Injecting), SpeechState::Injecting);
        assert_eq!(
            SpeechState::from(SttState::Error),
            SpeechState::Error("STT Pipeline Error".to_string())
        );
    }

    #[test]
    fn test_speech_state_serialization() {
        let state = SpeechState::Listening;
        let json = serde_json::to_string(&state).unwrap();
        let decoded: SpeechState = serde_json::from_str(&json).unwrap();
        assert_eq!(state, decoded);

        let err_state = SpeechState::Error("mic disconnected".into());
        let err_json = serde_json::to_string(&err_state).unwrap();
        let err_decoded: SpeechState = serde_json::from_str(&err_json).unwrap();
        assert_eq!(err_state, err_decoded);
    }

    #[test]
    fn test_overlay_position_parsing_and_display() {
        assert_eq!(OverlayPosition::from_str("bottom").unwrap(), OverlayPosition::Bottom);
        assert_eq!(OverlayPosition::from_str("TOP").unwrap(), OverlayPosition::Top);
        assert_eq!(OverlayPosition::from_str("center").unwrap(), OverlayPosition::Center);
        assert_eq!(OverlayPosition::from_str("None").unwrap(), OverlayPosition::None);
        assert!(OverlayPosition::from_str("invalid").is_err());
        assert_eq!(OverlayPosition::Bottom.to_string(), "Bottom");
    }

    #[test]
    fn test_whisper_model_variant_heuristics() {
        assert_eq!(WhisperModelVariant::from_filename("whisper-tiny.en.bin"), WhisperModelVariant::TinyEn);
        assert_eq!(WhisperModelVariant::from_filename("parakeet-unified-en-0.6b-Q8_0.gguf"), WhisperModelVariant::ParakeetV3);
        assert_eq!(WhisperModelVariant::from_filename("ggml-large-v3-turbo.bin"), WhisperModelVariant::TurboEn);
        assert_eq!(WhisperModelVariant::from_filename("my-custom-model.bin"), WhisperModelVariant::Custom("my-custom-model.bin".to_string()));
    }
}

