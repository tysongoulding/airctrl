use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormattedCliPayload {
    pub executable: String,
    pub args: Vec<String>,
    pub stdin: Option<String>,
    pub working_directory: Option<String>,
    pub env_vars: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DispatchPayloadDetails {
    pub instruction: String,
    pub context_window: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DispatchResult {
    pub target: String,
    pub status: String,
    pub prompt_len: usize,
    pub payload: DispatchPayloadDetails,
    pub cli: Option<FormattedCliPayload>,
}
