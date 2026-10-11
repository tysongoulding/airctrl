use crate::config::TextInjectionConfig;
use crate::error::SttError;
use airctrl_core::platform::traits::{PlatformInputOps, PlatformWindowOps};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

/// Result status of a text injection operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InjectionStatus {
    Success,
    Ignored,
    Failed,
}

/// Delivery mechanism utilized to inject text into the target application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InjectionMethod {
    None,
    SendInputUnicode,
    ClipboardPaste,
}

/// Structured report summarizing the outcome of a text injection attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InjectionReport {
    pub status: InjectionStatus,
    pub method: InjectionMethod,
    pub text_length: usize,
    pub target_window_id: Option<u64>,
}

/// Active window text injector with automatic focus restoration, direct Unicode typing,
/// and clipboard paste fallback.
pub struct ActiveWindowTextInjector {
    window_ops: Arc<dyn PlatformWindowOps>,
    input_ops: Arc<dyn PlatformInputOps>,
    config: TextInjectionConfig,
}

impl ActiveWindowTextInjector {
    pub fn new(
        window_ops: Arc<dyn PlatformWindowOps>,
        input_ops: Arc<dyn PlatformInputOps>,
        config: Option<TextInjectionConfig>,
    ) -> Self {
        Self {
            window_ops,
            input_ops,
            config: config.unwrap_or_default(),
        }
    }

    /// Injects text into the specified target window (or active foreground window).
    pub async fn inject(
        &self,
        text: &str,
        target_window_id: Option<u64>,
    ) -> Result<InjectionReport, SttError> {
        // 1. Empty string guard: ignore without firing keystrokes
        if text.is_empty() {
            return Ok(InjectionReport {
                status: InjectionStatus::Ignored,
                method: InjectionMethod::None,
                text_length: 0,
                target_window_id,
            });
        }

        // 2. Focus restoration guard: restore focus if window lost active focus
        if let Some(target_id) = target_window_id {
            let needs_refocus = match self.window_ops.get_active_window() {
                Ok(active) => active.id != target_id,
                Err(_) => true,
            };

            if needs_refocus {
                tracing::info!(
                    "Target window {} lost focus; restoring focus before text injection",
                    target_id
                );
                self.window_ops
                    .focus_window(target_id)
                    .map_err(|e| SttError::WindowFocus(target_id, e.to_string()))?;

                if self.config.refocus_settle_delay_ms > 0 {
                    tokio::time::sleep(Duration::from_millis(self.config.refocus_settle_delay_ms))
                        .await;
                }
            }
        }

        // 3. Strategy selection: large payloads (> threshold) bypass typing and use clipboard
        if text.len() > self.config.clipboard_fallback_threshold {
            return self.execute_clipboard_fallback(text, target_window_id);
        }

        // 4. Direct Unicode typing attempt
        match self.input_ops.type_text(text) {
            Ok(()) => Ok(InjectionReport {
                status: InjectionStatus::Success,
                method: InjectionMethod::SendInputUnicode,
                text_length: text.len(),
                target_window_id,
            }),
            Err(e) => {
                tracing::warn!(
                    "Direct typing simulation failed ({}). Falling back to clipboard paste.",
                    e
                );
                self.execute_clipboard_fallback(text, target_window_id)
            }
        }
    }

    /// Executes clipboard fallback by setting system clipboard and typing paste shortcut.
    fn execute_clipboard_fallback(
        &self,
        text: &str,
        target_window_id: Option<u64>,
    ) -> Result<InjectionReport, SttError> {
        self.input_ops
            .paste_text(text)
            .map_err(|e| SttError::Clipboard(e.to_string()))?;

        Ok(InjectionReport {
            status: InjectionStatus::Success,
            method: InjectionMethod::ClipboardPaste,
            text_length: text.len(),
            target_window_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use airctrl_core::platform::mock::{create_mock_platform, MockPlatformControllers};
    use airctrl_core::types::{WindowInfo, WindowRect};

    fn setup_test_environment() -> (ActiveWindowTextInjector, MockPlatformControllers) {
        let (provider, controllers) = create_mock_platform();
        let injector =
            ActiveWindowTextInjector::new(provider.window, provider.input, None);
        (injector, controllers)
    }

    #[tokio::test]
    async fn test_empty_string_ignored() {
        let (injector, controllers) = setup_test_environment();
        let report = injector.inject("", Some(1)).await.unwrap();

        assert_eq!(report.status, InjectionStatus::Ignored);
        assert_eq!(report.method, InjectionMethod::None);
        assert_eq!(controllers.input.get_typed_strings().len(), 0);
    }

    #[tokio::test]
    async fn test_direct_typing_happy_path() {
        let (injector, controllers) = setup_test_environment();
        controllers.window.add_window(WindowInfo {
            id: 1001,
            title: "Editor".to_string(),
            process_name: "code".to_string(),
            process_id: 100,
            bounds: WindowRect::default(),
            is_focused: true,
            is_minimized: false,
        });

        let report = injector
            .inject("Build airctrl desktop app", Some(1001))
            .await
            .unwrap();

        assert_eq!(report.status, InjectionStatus::Success);
        assert_eq!(report.method, InjectionMethod::SendInputUnicode);
        assert_eq!(
            controllers.input.get_typed_strings(),
            vec!["Build airctrl desktop app"]
        );
    }

    #[tokio::test]
    async fn test_large_payload_clipboard_fallback() {
        let (injector, controllers) = setup_test_environment();
        controllers.window.add_window(WindowInfo {
            id: 1001,
            title: "Editor".to_string(),
            process_name: "code".to_string(),
            process_id: 100,
            bounds: WindowRect::default(),
            is_focused: true,
            is_minimized: false,
        });

        let large_text = "A".repeat(500); // > 200 threshold
        let report = injector.inject(&large_text, Some(1001)).await.unwrap();

        assert_eq!(report.status, InjectionStatus::Success);
        assert_eq!(report.method, InjectionMethod::ClipboardPaste);
        assert_eq!(controllers.input.get_pasted_strings(), vec![large_text]);
    }

    #[tokio::test]
    async fn test_window_focus_restoration() {
        let (injector, controllers) = setup_test_environment();
        let win1 = WindowInfo {
            id: 1,
            title: "Target".to_string(),
            process_name: "app1".to_string(),
            process_id: 10,
            bounds: WindowRect::default(),
            is_focused: false,
            is_minimized: false,
        };
        let win2 = WindowInfo {
            id: 2,
            title: "Other".to_string(),
            process_name: "app2".to_string(),
            process_id: 20,
            bounds: WindowRect::default(),
            is_focused: true,
            is_minimized: false,
        };
        controllers.window.add_window(win1);
        controllers.window.add_window(win2);

        // Win 2 is active, target is Win 1
        let report = injector.inject("Refocus me", Some(1)).await.unwrap();
        assert_eq!(report.status, InjectionStatus::Success);

        // Verify window 1 was refocused
        assert_eq!(controllers.window.get_focus_history(), vec![1]);
    }
}
