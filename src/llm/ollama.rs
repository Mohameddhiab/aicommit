//! Ollama provider — local, privacy-first.

use crate::config::OllamaConfig;
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};

pub struct OllamaProvider {
    url: String,
    model: String,
    client: reqwest::Client,
}

impl OllamaProvider {
    pub fn new(cfg: OllamaConfig, timeout_secs: u64) -> Self {
        Self {
            url: cfg
                .url
                .unwrap_or_else(|| "http://localhost:11434".into())
                .trim_end_matches('/')
                .to_string(),
            model: cfg.model.unwrap_or_else(|| "qwen2.5-coder:7b".into()),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(timeout_secs))
                .build()
                .expect("reqwest client"),
        }
    }

    pub async fn chat(&self, system: &str, user: &str) -> Result<String> {
        #[derive(Serialize)]
        struct ChatRequest<'a> {
            model: &'a str,
            stream: bool,
            format: &'a str,
            messages: Vec<ChatMessage<'a>>,
        }
        #[derive(Serialize)]
        struct ChatMessage<'a> {
            role: &'a str,
            content: &'a str,
        }
        #[derive(Deserialize)]
        struct ChatResponse {
            message: Option<RespMessage>,
        }
        #[derive(Deserialize)]
        struct RespMessage {
            content: String,
        }

        let body = ChatRequest {
            model: &self.model,
            stream: false,
            format: "json",
            messages: vec![
                ChatMessage {
                    role: "system",
                    content: system,
                },
                ChatMessage {
                    role: "user",
                    content: user,
                },
            ],
        };
        let endpoint = format!("{}/api/chat", self.url);
        let resp = self
            .client
            .post(&endpoint)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() || e.is_timeout() {
                    anyhow!(
                        "cannot reach Ollama at {}.\n\
                         Ensure Ollama is installed and running:\n  \
                         https://ollama.com\n  \
                         `ollama serve`",
                        self.url
                    )
                } else {
                    anyhow!("POST {endpoint}: {e}")
                }
            })?;
        let status = resp.status();
        if !status.is_success() {
            let txt = resp.text().await.unwrap_or_default();
            anyhow::bail!("ollama returned {status}: {txt}");
        }
        let parsed: ChatResponse = resp.json().await.context("parse ollama response")?;
        parsed
            .message
            .map(|m| m.content)
            .ok_or_else(|| anyhow::anyhow!("ollama response missing message"))
    }

    pub async fn list_models(&self) -> Result<Vec<String>> {
        let url = format!("{}/api/tags", self.url);
        #[derive(Deserialize)]
        struct TagResponse {
            models: Vec<TagModel>,
        }
        #[derive(Deserialize)]
        struct TagModel {
            name: String,
        }
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .with_context(|| format!("GET {url}"))?;
        let parsed: TagResponse = resp.json().await?;
        Ok(parsed.models.into_iter().map(|m| m.name).collect())
    }
}
