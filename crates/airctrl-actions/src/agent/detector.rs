use airctrl_core::types::AgentTarget;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentEnvironmentStatus {
    Available {
        binary_path: PathBuf,
        version: Option<String>,
    },
    MissingEnvironment {
        target: AgentTarget,
        searched_paths: Vec<PathBuf>,
        install_command: String,
        documentation_url: String,
    },
}

pub struct AgentEnvironmentDetector;

impl AgentEnvironmentDetector {
    /// Inspects PATH and operating system binary directories to detect agent CLI presence.
    pub fn detect(target: AgentTarget) -> AgentEnvironmentStatus {
        let binary_names = Self::binary_names_for_target(target);
        for bin in &binary_names {
            if let Some(path) = Self::find_in_path(bin) {
                return AgentEnvironmentStatus::Available {
                    binary_path: path,
                    version: None,
                };
            }
        }

        AgentEnvironmentStatus::MissingEnvironment {
            target,
            searched_paths: Self::get_search_paths(),
            install_command: Self::install_command_for_target(target),
            documentation_url: Self::docs_for_target(target),
        }
    }

    fn binary_names_for_target(target: AgentTarget) -> Vec<&'static str> {
        match target {
            AgentTarget::Cursor => {
                if cfg!(windows) {
                    vec!["cursor.cmd", "cursor.exe", "cursor"]
                } else {
                    vec!["cursor"]
                }
            }
            AgentTarget::Claude => {
                if cfg!(windows) {
                    vec!["claude.cmd", "claude.exe", "claude"]
                } else {
                    vec!["claude"]
                }
            }
            AgentTarget::Antigravity => vec!["antigravity"],
            AgentTarget::Grokbot => vec!["grokbot", "grok"],
            AgentTarget::Dots => vec!["dots"],
            AgentTarget::Muse => vec!["muse"],
            AgentTarget::Viktor => vec!["viktor"],
            AgentTarget::Gemini => vec!["gemini"],
        }
    }

    fn find_in_path(binary_name: &str) -> Option<PathBuf> {
        let path_var = std::env::var_os("PATH")?;
        for dir in std::env::split_paths(&path_var) {
            let candidate = dir.join(binary_name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
        None
    }

    fn get_search_paths() -> Vec<PathBuf> {
        std::env::var_os("PATH")
            .map(|paths| std::env::split_paths(&paths).collect())
            .unwrap_or_default()
    }

    fn install_command_for_target(target: AgentTarget) -> String {
        match target {
            AgentTarget::Cursor => "https://cursor.com/download".to_string(),
            AgentTarget::Claude => "npm install -g @anthropic-ai/claude-code".to_string(),
            AgentTarget::Antigravity => "cargo install antigravity-cli".to_string(),
            AgentTarget::Grokbot => "pip install grokbot".to_string(),
            AgentTarget::Dots => "npm install -g dots-cli".to_string(),
            AgentTarget::Muse => "pip install muse-agent".to_string(),
            AgentTarget::Viktor => "pip install viktor-ai".to_string(),
            AgentTarget::Gemini => "npm install -g gemini-cli".to_string(),
        }
    }

    fn docs_for_target(target: AgentTarget) -> String {
        match target {
            AgentTarget::Cursor => "https://docs.cursor.com".to_string(),
            AgentTarget::Claude => "https://docs.anthropic.com/en/docs/claude-code".to_string(),
            AgentTarget::Antigravity => "https://github.com/google/antigravity".to_string(),
            _ => "https://github.com/tysongoulding/airctrl".to_string(),
        }
    }
}
