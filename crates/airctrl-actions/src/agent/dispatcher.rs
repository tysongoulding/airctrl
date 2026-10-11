use super::payload::*;
use crate::error::ActionError;
use airctrl_core::types::{AgentPayload, AgentTarget};
use chrono::Utc;

pub struct AgentDispatcher;

impl AgentDispatcher {
    pub const SUPPORTED_AGENTS: [&'static str; 8] = [
        "Cursor", "Claude", "Antigravity", "Grokbot", "Dots", "Muse", "Viktor", "Gemini",
    ];

    /// Dispatches instructions to one of the 8 supported agent targets.
    /// Strictly validates agent name case and target existence.
    pub fn dispatch(
        target_name: &str,
        prompt: &str,
        context: Option<&str>,
    ) -> Result<DispatchResult, ActionError> {
        let agent_target = Self::parse_strict_target(target_name)?;
        let context_str = context.unwrap_or("").to_string();
        let timestamp = Utc::now().to_rfc3339();

        let cli = Self::format_cli(agent_target, prompt, &context_str);

        Ok(DispatchResult {
            target: target_name.to_string(),
            status: "Dispatched".to_string(),
            prompt_len: prompt.chars().count(),
            payload: DispatchPayloadDetails {
                instruction: prompt.to_string(),
                context_window: context_str,
                timestamp,
            },
            cli: Some(cli),
        })
    }

    /// Strict parser enforcing case sensitivity (e.g. "Cursor" succeeds, "cursor" rejected).
    pub fn parse_strict_target(name: &str) -> Result<AgentTarget, ActionError> {
        match name {
            "Cursor" => Ok(AgentTarget::Cursor),
            "Claude" => Ok(AgentTarget::Claude),
            "Antigravity" => Ok(AgentTarget::Antigravity),
            "Grokbot" => Ok(AgentTarget::Grokbot),
            "Dots" => Ok(AgentTarget::Dots),
            "Muse" => Ok(AgentTarget::Muse),
            "Viktor" => Ok(AgentTarget::Viktor),
            "Gemini" => Ok(AgentTarget::Gemini),
            _ => Err(ActionError::UnsupportedAgent(format!(
                "Unsupported agent target: {}",
                name
            ))),
        }
    }

    /// Formats rich CLI execution payload depending on the agent tool.
    pub fn format_cli(target: AgentTarget, prompt: &str, context: &str) -> FormattedCliPayload {
        match target {
            AgentTarget::Cursor => {
                let formatted_prompt = if context.is_empty() {
                    prompt.to_string()
                } else {
                    format!("[Context: {}]\n\n{}", context, prompt)
                };
                FormattedCliPayload {
                    executable: "cursor".to_string(),
                    args: vec![".".to_string()],
                    stdin: Some(formatted_prompt),
                    working_directory: None,
                    env_vars: vec![],
                }
            }
            AgentTarget::Claude => {
                let full_prompt = if context.is_empty() {
                    prompt.to_string()
                } else {
                    format!("[Context: {}]\n\n{}", context, prompt)
                };
                FormattedCliPayload {
                    executable: "claude".to_string(),
                    args: vec!["--prompt".to_string(), full_prompt],
                    stdin: None,
                    working_directory: None,
                    env_vars: vec![],
                }
            }
            AgentTarget::Antigravity => {
                let task_json = serde_json::json!({
                    "protocol": "antigravity-task-v1",
                    "target": "Antigravity",
                    "mission": prompt,
                    "context": context,
                    "timestamp": Utc::now().to_rfc3339()
                })
                .to_string();
                FormattedCliPayload {
                    executable: "antigravity".to_string(),
                    args: vec!["run".to_string(), "--task".to_string(), task_json],
                    stdin: None,
                    working_directory: None,
                    env_vars: vec![],
                }
            }
            AgentTarget::Grokbot => FormattedCliPayload {
                executable: "grokbot".to_string(),
                args: vec![
                    "exec".to_string(),
                    "--query".to_string(),
                    prompt.to_string(),
                    "--context".to_string(),
                    context.to_string(),
                ],
                stdin: None,
                working_directory: None,
                env_vars: vec![],
            },
            AgentTarget::Dots => FormattedCliPayload {
                executable: "dots".to_string(),
                args: vec![
                    "run".to_string(),
                    "--instruction".to_string(),
                    prompt.to_string(),
                    "--json".to_string(),
                ],
                stdin: None,
                working_directory: None,
                env_vars: vec![],
            },
            AgentTarget::Muse => FormattedCliPayload {
                executable: "muse".to_string(),
                args: vec!["generate".to_string(), "--prompt".to_string(), prompt.to_string()],
                stdin: None,
                working_directory: None,
                env_vars: vec![],
            },
            AgentTarget::Viktor => FormattedCliPayload {
                executable: "viktor".to_string(),
                args: vec!["dispatch".to_string(), "--task".to_string(), prompt.to_string()],
                stdin: None,
                working_directory: None,
                env_vars: vec![],
            },
            AgentTarget::Gemini => FormattedCliPayload {
                executable: "gemini".to_string(),
                args: vec!["query".to_string(), "--prompt".to_string(), prompt.to_string()],
                stdin: None,
                working_directory: None,
                env_vars: vec![],
            },
        }
    }
}

/// Type alias for AgentDispatcher
pub type AgenticHarness = AgentDispatcher;

/// Public dispatch function fulfilling PROJECT.md interface contract.
pub fn dispatch_agentic_instruction(
    target: AgentTarget,
    payload: &AgentPayload,
) -> Result<DispatchResult, ActionError> {
    let target_name = match target {
        AgentTarget::Cursor => "Cursor",
        AgentTarget::Claude => "Claude",
        AgentTarget::Antigravity => "Antigravity",
        AgentTarget::Grokbot => "Grokbot",
        AgentTarget::Dots => "Dots",
        AgentTarget::Muse => "Muse",
        AgentTarget::Viktor => "Viktor",
        AgentTarget::Gemini => "Gemini",
    };

    AgentDispatcher::dispatch(
        target_name,
        &payload.prompt,
        payload.context_window.as_deref(),
    )
}
