use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HotkeyMode {
    PushToTalk,
    Toggle,
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
}
