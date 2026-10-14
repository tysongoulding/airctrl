use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SummaryProvider {
    #[serde(rename = "Ollama")]
    Ollama,
    #[serde(rename = "Claude")]
    Claude,
    #[serde(rename = "Groq")]
    Groq,
    #[serde(rename = "OpenRouter")]
    OpenRouter,
    #[serde(rename = "CustomOpenAI")]
    CustomOpenAi,
}

impl SummaryProvider {
    pub const ALL: [SummaryProvider; 5] = [
        Self::Ollama,
        Self::Claude,
        Self::Groq,
        Self::OpenRouter,
        Self::CustomOpenAi,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ollama => "Ollama",
            Self::Claude => "Claude",
            Self::Groq => "Groq",
            Self::OpenRouter => "OpenRouter",
            Self::CustomOpenAi => "CustomOpenAI",
        }
    }
}

impl std::str::FromStr for SummaryProvider {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().replace(['_', '-'], "").as_str() {
            "ollama" => Ok(Self::Ollama),
            "claude" | "anthropic" => Ok(Self::Claude),
            "groq" => Ok(Self::Groq),
            "openrouter" => Ok(Self::OpenRouter),
            "customopenai" | "openai" | "custom" => Ok(Self::CustomOpenAi),
            _ => Err(format!("Unknown summary provider: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SummaryTemplate {
    #[serde(rename = "ExecutiveSummary")]
    ExecutiveSummary,
    #[serde(rename = "ActionItems")]
    ActionItems,
    #[serde(rename = "TechnicalDecisions")]
    TechnicalDecisions,
    #[serde(rename = "Custom")]
    Custom,
}

impl SummaryTemplate {
    pub const ALL: [SummaryTemplate; 4] = [
        Self::ExecutiveSummary,
        Self::ActionItems,
        Self::TechnicalDecisions,
        Self::Custom,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ExecutiveSummary => "ExecutiveSummary",
            Self::ActionItems => "ActionItems",
            Self::TechnicalDecisions => "TechnicalDecisions",
            Self::Custom => "Custom",
        }
    }
}

impl std::str::FromStr for SummaryTemplate {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().replace(['_', '-'], "").as_str() {
            "executivesummary" | "executive" => Ok(Self::ExecutiveSummary),
            "actionitems" | "actions" => Ok(Self::ActionItems),
            "technicaldecisions" | "decisions" | "tdr" => Ok(Self::TechnicalDecisions),
            "custom" => Ok(Self::Custom),
            _ => Err(format!("Unknown summary template: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryProviderConfig {
    pub provider: SummaryProvider,
    pub endpoint: Option<String>,
    pub api_key: Option<String>,
    pub model: String,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub timeout_secs: Option<u64>,
}

impl Default for SummaryProviderConfig {
    fn default() -> Self {
        Self {
            provider: SummaryProvider::Ollama,
            endpoint: Some("http://localhost:11434".to_string()),
            api_key: None,
            model: "llama3.2".to_string(),
            max_tokens: Some(4096),
            temperature: Some(0.2),
            timeout_secs: Some(30),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryResult {
    pub provider: String,
    pub template: String,
    pub summary_markdown: String,
    pub generated_at: String,
    pub token_count: usize,
}
