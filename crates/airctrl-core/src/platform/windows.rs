use super::traits::{PlatformAudioOps, PlatformInputOps, PlatformWindowOps};
use crate::error::CoreError;
use crate::types::*;

pub struct WindowsWindowOps;
impl PlatformWindowOps for WindowsWindowOps {
    fn get_active_window(&self) -> Result<WindowInfo, CoreError> {
        #[cfg(target_os = "windows")]
        {
            Ok(WindowInfo {
                id: 1,
                process_id: std::process::id(),
                process_name: "airctrl.exe".to_string(),
                title: "airctrl Windows Host".to_string(),
                bounds: WindowRect { x: 0, y: 0, width: 1920, height: 1080 },
                is_focused: true,
                is_minimized: false,
            })
        }
        #[cfg(not(target_os = "windows"))]
        {
            Err(CoreError::PlatformUnsupported("WindowsWindowOps requires Windows OS".into()))
        }
    }

    fn focus_window(&self, _id: u64) -> Result<(), CoreError> {
        Ok(())
    }

    fn enumerate_windows(&self) -> Result<Vec<WindowInfo>, CoreError> {
        Ok(vec![])
    }

    fn minimize_window(&self, _id: u64) -> Result<(), CoreError> {
        Ok(())
    }

    fn close_window(&self, _id: u64) -> Result<(), CoreError> {
        Ok(())
    }
}

pub struct WindowsInputOps;
impl PlatformInputOps for WindowsInputOps {
    fn type_text(&self, _text: &str) -> Result<(), CoreError> {
        Ok(())
    }

    fn paste_text(&self, _text: &str) -> Result<(), CoreError> {
        Ok(())
    }

    fn send_key_combination(
        &self,
        _modifiers: &[ModifierKey],
        _key: InputKey,
    ) -> Result<(), CoreError> {
        Ok(())
    }

    fn move_pointer(&self, _x: i32, _y: i32) -> Result<(), CoreError> {
        Ok(())
    }

    fn click_pointer(&self, _action: ClickAction) -> Result<(), CoreError> {
        Ok(())
    }
}

pub struct WindowsAudioOps;
impl PlatformAudioOps for WindowsAudioOps {
    fn play_audio_stream(&self, _samples: &[f32], _sample_rate: u32) -> Result<(), CoreError> {
        Ok(())
    }

    fn capture_audio_chunk(&self) -> Result<Vec<f32>, CoreError> {
        Ok(vec![0.0f32; 1600])
    }

    fn is_audio_output_active(&self) -> Result<bool, CoreError> {
        Ok(false)
    }

    fn list_audio_devices(&self) -> Result<Vec<String>, CoreError> {
        Ok(vec!["Windows Default Audio Device".to_string()])
    }
}
