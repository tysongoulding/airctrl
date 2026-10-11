use super::traits::{PlatformAudioOps, PlatformInputOps, PlatformWindowOps};
use crate::error::CoreError;
use crate::types::*;

pub struct LinuxWindowOps;
impl PlatformWindowOps for LinuxWindowOps {
    fn get_active_window(&self) -> Result<WindowInfo, CoreError> {
        Ok(WindowInfo {
            id: 1,
            process_id: std::process::id(),
            process_name: "airctrl".to_string(),
            title: "airctrl Desktop".to_string(),
            bounds: WindowRect { x: 0, y: 0, width: 1920, height: 1080 },
            is_focused: true,
            is_minimized: false,
        })
    }

    fn focus_window(&self, _id: u64) -> Result<(), CoreError> {
        Ok(())
    }

    fn enumerate_windows(&self) -> Result<Vec<WindowInfo>, CoreError> {
        Ok(vec![self.get_active_window()?])
    }

    fn minimize_window(&self, _id: u64) -> Result<(), CoreError> {
        Ok(())
    }

    fn close_window(&self, _id: u64) -> Result<(), CoreError> {
        Ok(())
    }
}

pub struct LinuxInputOps;
impl PlatformInputOps for LinuxInputOps {
    fn type_text(&self, text: &str) -> Result<(), CoreError> {
        tracing::debug!("LinuxInputOps: typing text ({} chars)", text.len());
        Ok(())
    }

    fn paste_text(&self, text: &str) -> Result<(), CoreError> {
        tracing::debug!("LinuxInputOps: pasting text ({} chars)", text.len());
        Ok(())
    }

    fn send_key_combination(
        &self,
        modifiers: &[ModifierKey],
        key: InputKey,
    ) -> Result<(), CoreError> {
        tracing::debug!("LinuxInputOps: key combo {:?} + {:?}", modifiers, key);
        Ok(())
    }

    fn move_pointer(&self, x: i32, y: i32) -> Result<(), CoreError> {
        tracing::debug!("LinuxInputOps: move pointer ({}, {})", x, y);
        Ok(())
    }

    fn click_pointer(&self, action: ClickAction) -> Result<(), CoreError> {
        tracing::debug!("LinuxInputOps: click pointer {:?}", action);
        Ok(())
    }
}

pub struct LinuxAudioOps;
impl PlatformAudioOps for LinuxAudioOps {
    fn play_audio_stream(&self, samples: &[f32], sample_rate: u32) -> Result<(), CoreError> {
        tracing::debug!("LinuxAudioOps: playing {} samples at {}Hz", samples.len(), sample_rate);
        Ok(())
    }

    fn capture_audio_chunk(&self) -> Result<Vec<f32>, CoreError> {
        Ok(vec![0.0f32; 1600])
    }

    fn is_audio_output_active(&self) -> Result<bool, CoreError> {
        Ok(false)
    }

    fn list_audio_devices(&self) -> Result<Vec<String>, CoreError> {
        Ok(vec!["PipeWire Default Sink".to_string()])
    }
}
