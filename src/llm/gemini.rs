//! Google Gemini provider.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

pub struct GeminiProvider {
    base_url: String,
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl GeminiProvider {
    pub fn new(base_url: String, api_key: String, model: String) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
            model,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .expect("reqwest client"),
        }
    }

    pub async fn chat(&self, system: &str, user: &str) -> Result<String> {
        #[derive(Serialize)]
        struct GenerateContentRequest<'a> {
            contents: Vec<ContentPart<'a>>,
            #[serde(skip_serializing_if = "Option::is_none")]
            system_instruction: Option<ContentPart<'a>>,
            generation_config: GenerationConfig,
        }
        #[derive(Serialize)]
        struct ContentPart<'a> {
            role: &'a str,
            parts: Vec<TextPart<'a>>,
        }
        #[derive(Serialize)]
        struct TextPart<'a> {
            text: &'a str,
        }
        #[derive(Serialize)]
        struct GenerationConfig {
            response_mime_type: &'static str,
        }
        #[derive(Deserialize)]
        struct GenerateContentResponse {
            candidates: Vec<Candidate>,
        }
        #[derive(Deserialize)]
        struct Candidate {
            content: CandidateContent,
        }
        #[derive(Deserialize)]
        struct CandidateContent {
            parts: Vec<TextPartResp>,
        }
        #[derive(Deserialize)]
        struct TextPartResp {
            text: String,
        }

        let body = GenerateContentRequest {
            contents: vec![ContentPart {
                role: "user",
                parts: vec![TextPart { text: user }],
            }],
            system_instruction: Some(ContentPart {
                role: "system",
                parts: vec![TextPart { text: system }],
            }),
            generation_config: GenerationConfig {
                response_mime_type: "application/json",
            },
        };
        let endpoint = format!("{}/models/{}:generateContent", self.base_url, self.model);
        let resp = self
            .client
            .post(&endpoint)
            .query(&[("key", &self.api_key)])
            .json(&body)
            .send()
            .await
            .with_context(|| format!("POST {endpoint}"))?;
        let status = resp.status();
        if !status.is_success() {
            let txt = resp.text().await.unwrap_or_default();
            anyhow::bail!("gemini returned {status}: {txt}");
        }
        let parsed: GenerateContentResponse = resp.json().await.context("parse gemini response")?;
        parsed
            .candidates
            .into_iter()
            .next()
            .and_then(|c| c.content.parts.into_iter().next().map(|p| p.text))
            .ok_or_else(|| anyhow::anyhow!("gemini response had no text"))
    }
}
