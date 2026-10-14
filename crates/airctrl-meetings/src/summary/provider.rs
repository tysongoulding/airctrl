use crate::error::{MeetingError, Result};
use super::types::{SummaryProvider, SummaryProviderConfig};
use serde_json::json;
use std::time::Duration;

pub struct HttpSummaryClient;

impl HttpSummaryClient {
    /// Calls the configured remote AI provider endpoint asynchronously.
    pub async fn call_provider(
        config: &SummaryProviderConfig,
        prompt: &str,
    ) -> Result<String> {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_millis(1500))
            .timeout(Duration::from_secs(config.timeout_secs.unwrap_or(30)))
            .build()
            .map_err(|e| MeetingError::ProviderError(format!("Failed to build HTTP client: {e}")))?;

        match config.provider {
            SummaryProvider::Ollama => {
                let endpoint = config
                    .endpoint
                    .as_deref()
                    .unwrap_or("http://localhost:11434");
                let url = format!("{}/api/generate", endpoint.trim_end_matches('/'));

                let body = json!({
                    "model": config.model,
                    "prompt": prompt,
                    "stream": false,
                    "options": {
                        "temperature": config.temperature.unwrap_or(0.2)
                    }
                });

                let res = client.post(&url)
                    .json(&body)
                    .send()
                    .await
                    .map_err(|e| {
                        if e.is_connect() {
                            MeetingError::OllamaNotRunning(format!(
                                "Ollama daemon unreachable at {endpoint}: {e}"
                            ))
                        } else {
                            MeetingError::ProviderError(format!("Ollama HTTP request failed: {e}"))
                        }
                    })?;

                if !res.status().is_success() {
                    let status = res.status();
                    let text = res.text().await.unwrap_or_default();
                    return Err(MeetingError::ProviderError(format!(
                        "Ollama returned HTTP {status}: {text}"
                    )));
                }

                let parsed: serde_json::Value = res.json().await.map_err(|e| {
                    MeetingError::ProviderError(format!("Failed to parse Ollama response: {e}"))
                })?;

                parsed.get("response")
                    .and_then(|v| v.as_str())
                    .map(String::from)
                    .ok_or_else(|| MeetingError::ProviderError("Missing 'response' field in Ollama output".into()))
            }

            SummaryProvider::Claude => {
                let endpoint = config
                    .endpoint
                    .as_deref()
                    .unwrap_or("https://api.anthropic.com/v1/messages");
                let api_key = config.api_key.as_deref().unwrap_or("");
                if api_key.is_empty() {
                    return Err(MeetingError::AuthenticationFailed(
                        "Claude API key is missing or empty".into(),
                    ));
                }

                let body = json!({
                    "model": config.model,
                    "max_tokens": config.max_tokens.unwrap_or(4096),
                    "temperature": config.temperature.unwrap_or(0.2),
                    "messages": [
                        { "role": "user", "content": prompt }
                    ]
                });

                let res = client.post(endpoint)
                    .header("x-api-key", api_key)
                    .header("anthropic-version", "2023-06-01")
                    .header("content-type", "application/json")
                    .json(&body)
                    .send()
                    .await
                    .map_err(|e| MeetingError::ProviderError(format!("Claude HTTP request failed: {e}")))?;

                let status = res.status();
                if status.as_u16() == 401 {
                    return Err(MeetingError::AuthenticationFailed("Invalid Anthropic API key".into()));
                }
                if status.as_u16() == 429 {
                    return Err(MeetingError::RateLimitExceeded("Anthropic rate limit reached".into()));
                }
                if !status.is_success() {
                    let text = res.text().await.unwrap_or_default();
                    return Err(MeetingError::ProviderError(format!("Claude error {status}: {text}")));
                }

                let parsed: serde_json::Value = res.json().await.map_err(|e| {
                    MeetingError::ProviderError(format!("Failed to parse Claude JSON: {e}"))
                })?;

                parsed["content"][0]["text"]
                    .as_str()
                    .map(String::from)
                    .ok_or_else(|| MeetingError::ProviderError("Missing content.text in Claude output".into()))
            }

            SummaryProvider::Groq | SummaryProvider::OpenRouter | SummaryProvider::CustomOpenAi => {
                let default_endpoint = match config.provider {
                    SummaryProvider::Groq => "https://api.groq.com/openai/v1/chat/completions",
                    SummaryProvider::OpenRouter => "https://openrouter.ai/api/v1/chat/completions",
                    _ => "http://localhost:8000/v1/chat/completions",
                };
                let url = config.endpoint.as_deref().unwrap_or(default_endpoint);

                let mut req = client.post(url).header("content-type", "application/json");
                if let Some(ref key) = config.api_key {
                    req = req.header("authorization", format!("Bearer {key}"));
                }
                if config.provider == SummaryProvider::OpenRouter {
                    req = req.header("http-referer", "https://airctrl.local")
                        .header("x-title", "airctrl");
                }

                let body = json!({
                    "model": config.model,
                    "messages": [
                        { "role": "system", "content": "You are a professional meeting assistant summarizing transcripts in Markdown." },
                        { "role": "user", "content": prompt }
                    ],
                    "temperature": config.temperature.unwrap_or(0.2),
                    "max_tokens": config.max_tokens.unwrap_or(4096)
                });

                let res = req.json(&body)
                    .send()
                    .await
                    .map_err(|e| MeetingError::ProviderError(format!("HTTP completion request failed: {e}")))?;

                let status = res.status();
                if status.as_u16() == 401 {
                    return Err(MeetingError::AuthenticationFailed("Authentication failed (HTTP 401)".into()));
                }
                if status.as_u16() == 429 {
                    return Err(MeetingError::RateLimitExceeded("Rate limit exceeded (HTTP 429)".into()));
                }
                if !status.is_success() {
                    let text = res.text().await.unwrap_or_default();
                    return Err(MeetingError::ProviderError(format!("Endpoint error {status}: {text}")));
                }

                let parsed: serde_json::Value = res.json().await.map_err(|e| {
                    MeetingError::ProviderError(format!("Failed to parse completions JSON: {e}"))
                })?;

                parsed["choices"][0]["message"]["content"]
                    .as_str()
                    .map(String::from)
                    .ok_or_else(|| MeetingError::ProviderError("Missing choices[0].message.content in output".into()))
            }
        }
    }
}
