//! OpenAI-compatible chat completions client.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

pub struct OpenAiCompatProvider {
    base_url: String,
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl OpenAiCompatProvider {
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
        struct ChatRequest<'a> {
            model: &'a str,
            stream: bool,
            response_format: ResponseFormat,
            messages: Vec<ChatMessage<'a>>,
        }
        #[derive(Serialize)]
        struct ResponseFormat {
            #[serde(rename = "type")]
            kind: &'static str,
        }
        #[derive(Serialize)]
        struct ChatMessage<'a> {
            role: &'a str,
            content: &'a str,
        }
        #[derive(Deserialize)]
        struct ChatResponse {
            choices: Vec<Choice>,
        }
        #[derive(Deserialize)]
        struct Choice {
            message: ChoiceMessage,
        }
        #[derive(Deserialize)]
        struct ChoiceMessage {
            content: String,
        }

        let body = ChatRequest {
            model: &self.model,
            stream: false,
            response_format: ResponseFormat {
                kind: "json_object",
            },
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
        let endpoint = format!("{}/chat/completions", self.base_url);
        let resp = self
            .client
            .post(&endpoint)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .with_context(|| format!("POST {endpoint}"))?;
        let status = resp.status();
        if !status.is_success() {
            let txt = resp.text().await.unwrap_or_default();
            if status == 401 {
                anyhow::bail!(
                    "invalid API key (401 Unauthorized).\n\
                     Set the correct key via:\n  \
                     aicommit config --provider <name> --api-key <key>\n  \
                     or the corresponding environment variable (e.g. OPENAI_API_KEY)"
                );
            }
            anyhow::bail!("provider returned {status}: {txt}");
        }
        let parsed: ChatResponse = resp.json().await.context("parse provider response")?;
        parsed
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .ok_or_else(|| anyhow::anyhow!("provider response had no choices"))
    }

    pub async fn list_models(&self) -> Result<Vec<String>> {
        let url = format!("{}/models", self.base_url);
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
            .bearer_auth(&self.api_key)
            .send()
            .await
            .with_context(|| format!("GET {url}"))?;
        let parsed: ListResponse = resp.json().await?;
        Ok(parsed.data.into_iter().map(|m| m.id).collect())
    }
}
