//! Anthropic Messages API client.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

const ANTHROPIC_VERSION: &str = "2023-06-01";

pub struct AnthropicProvider {
    base_url: String,
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl AnthropicProvider {
    pub fn new(base_url: String, api_key: String, model: String, timeout_secs: u64) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
            model,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(timeout_secs))
                .build()
                .expect("reqwest client"),
        }
    }

    pub async fn chat(&self, system: &str, user: &str) -> Result<String> {
        #[derive(Serialize)]
        struct MessagesRequest<'a> {
            model: &'a str,
            max_tokens: u32,
            system: &'a str,
            messages: Vec<UserMessage<'a>>,
        }
        #[derive(Serialize)]
        struct UserMessage<'a> {
            role: &'a str,
            content: &'a str,
        }
        #[derive(Deserialize)]
        struct MessagesResponse {
            content: Vec<ContentBlock>,
        }
        #[derive(Deserialize)]
        #[serde(tag = "type")]
        enum ContentBlock {
            #[serde(rename = "text")]
            Text { text: String },
            #[serde(other)]
            Other,
        }

        let body = MessagesRequest {
            model: &self.model,
            max_tokens: 256,
            system,
            messages: vec![UserMessage {
                role: "user",
                content: user,
            }],
        };
        let endpoint = format!("{}/v1/messages", self.base_url);
        let resp = self
            .client
            .post(&endpoint)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .with_context(|| format!("POST {endpoint}"))?;
        let status = resp.status();
        if !status.is_success() {
            let txt = resp.text().await.unwrap_or_default();
            if status == 401 {
                anyhow::bail!(
                    "authentication failed (401 Unauthorized)\n\
                     → check your ANTHROPIC_API_KEY is correct\n\
                     → set it with: doctor config --provider anthropic --api-key <key>"
                );
            }
            anyhow::bail!("anthropic returned {status}: {txt}");
        }
        let parsed: MessagesResponse = resp.json().await.context("parse anthropic response")?;
        parsed
            .content
            .into_iter()
            .find_map(|b| match b {
                ContentBlock::Text { text } => Some(text),
                _ => None,
            })
            .ok_or_else(|| anyhow::anyhow!("anthropic response had no text block"))
    }

    pub async fn list_models(&self) -> Result<Vec<String>> {
        let url = format!("{}/v1/models", self.base_url);
        #[derive(Deserialize)]
        struct ListResponse {
            data: Vec<ModelEntry>,
        }
        #[derive(Deserialize)]
        struct ModelEntry {
            id: String,
        }
        let resp = self
            .client
            .get(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .send()
            .await
            .with_context(|| format!("GET {url}"))?;
        let parsed: ListResponse = resp.json().await?;
        Ok(parsed.data.into_iter().map(|m| m.id).collect())
    }
}
