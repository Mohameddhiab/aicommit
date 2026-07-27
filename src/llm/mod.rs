pub mod anthropic;
pub mod gemini;
pub mod mock;
pub mod ollama;
pub mod openai_compat;

use crate::config::{Config, ProviderKind};
use crate::prompt;
use anyhow::{anyhow, Context, Result};

/// Concrete provider enum — each variant wraps one backend.
/// Avoids trait-object complexity with async_trait.
pub enum AnyProvider {
    Mock(mock::MockProvider),
    Ollama(ollama::OllamaProvider),
    OpenAiCompat(openai_compat::OpenAiCompatProvider),
    Anthropic(anthropic::AnthropicProvider),
    Gemini(gemini::GeminiProvider),
}

impl AnyProvider {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Mock(_) => "mock",
            Self::Ollama(_) => "ollama",
            Self::OpenAiCompat(_) => "openai-compatible",
            Self::Anthropic(_) => "anthropic",
            Self::Gemini(_) => "gemini",
        }
    }

    pub async fn chat(&self, system: &str, user: &str) -> Result<String> {
        match self {
            Self::Mock(p) => p.chat(system, user).await,
            Self::Ollama(p) => p.chat(system, user).await,
            Self::OpenAiCompat(p) => p.chat(system, user).await,
            Self::Anthropic(p) => p.chat(system, user).await,
            Self::Gemini(p) => p.chat(system, user).await,
        }
    }

    pub async fn list_models(&self) -> Result<Vec<String>> {
        match self {
            Self::Mock(p) => p.list_models().await,
            Self::Ollama(p) => p.list_models().await,
            Self::OpenAiCompat(p) => p.list_models().await,
            Self::Anthropic(p) => p.list_models().await,
            Self::Gemini(p) => p.list_models().await,
        }
    }
}

/// Build the concrete provider from the resolved config.
pub fn build_provider(cfg: &Config) -> Result<AnyProvider> {
    let timeout = cfg.commit.timeout_secs;
    match cfg.provider {
        ProviderKind::Mock => Ok(AnyProvider::Mock(mock::MockProvider::new())),
        ProviderKind::Ollama => Ok(AnyProvider::Ollama(ollama::OllamaProvider::new(
            cfg.ollama.clone(),
            timeout,
        ))),
        ProviderKind::Openai
        | ProviderKind::Groq
        | ProviderKind::Deepseek
        | ProviderKind::Mistral
        | ProviderKind::Openrouter => {
            let base_url = cfg
                .remote
                .base_url
                .clone()
                .unwrap_or_else(|| default_base_url(cfg.provider));
            let api_key = cfg
                .remote
                .api_key
                .clone()
                .ok_or_else(|| anyhow!("missing API key for {}", cfg.provider.as_str()))?;
            let model = cfg
                .remote
                .model
                .clone()
                .ok_or_else(|| anyhow!("missing model for {}", cfg.provider.as_str()))?;
            Ok(AnyProvider::OpenAiCompat(
                openai_compat::OpenAiCompatProvider::new(base_url, api_key, model, timeout),
            ))
        }
        ProviderKind::Anthropic => {
            let base_url = cfg
                .remote
                .base_url
                .clone()
                .unwrap_or_else(|| "https://api.anthropic.com".into());
            let api_key = cfg
                .remote
                .api_key
                .clone()
                .ok_or_else(|| anyhow!("missing ANTHROPIC_API_KEY"))?;
            let model = cfg
                .remote
                .model
                .clone()
                .unwrap_or_else(|| "claude-3-5-sonnet-20241022".into());
            Ok(AnyProvider::Anthropic(anthropic::AnthropicProvider::new(
                base_url, api_key, model, timeout,
            )))
        }
        ProviderKind::Gemini => {
            let base_url = cfg
                .remote
                .base_url
                .clone()
                .unwrap_or_else(|| "https://generativelanguage.googleapis.com/v1beta".into());
            let api_key = cfg
                .remote
                .api_key
                .clone()
                .ok_or_else(|| anyhow!("missing GOOGLE_API_KEY"))?;
            let model = cfg
                .remote
                .model
                .clone()
                .unwrap_or_else(|| "gemini-1.5-flash".into());
            Ok(AnyProvider::Gemini(gemini::GeminiProvider::new(
                base_url, api_key, model, timeout,
            )))
        }
    }
}

fn default_base_url(p: ProviderKind) -> String {
    match p {
        ProviderKind::Openai => "https://api.openai.com/v1".into(),
        ProviderKind::Groq => "https://api.groq.com/openai/v1".into(),
        ProviderKind::Deepseek => "https://api.deepseek.com/v1".into(),
        ProviderKind::Mistral => "https://api.mistral.ai/v1".into(),
        ProviderKind::Openrouter => "https://openrouter.ai/api/v1".into(),
        _ => String::new(),
    }
}

/// High-level helper: build prompt, ask the provider, parse the response.
/// Retries once on parse failure with a stricter prompt.
pub async fn generate_commit_message(
    provider: &AnyProvider,
    diff: &str,
    cfg: &Config,
) -> Result<String> {
    let system = prompt::system_prompt(cfg);
    let user = prompt::user_prompt(diff);
    do_generate(provider, &system, &user).await
}

async fn do_generate(provider: &AnyProvider, system: &str, user: &str) -> Result<String> {
    let raw = provider
        .chat(system, user)
        .await
        .context("provider chat call")?;
    match crate::parser::parse_commit_message(&raw) {
        Ok(parsed) => Ok(parsed.to_conventional()),
        Err(first_err) => {
            let retry_system = format!(
                "Your previous response was not valid JSON. \
                 Reply with ONLY a single JSON object, no other text.\n\
                 Schema: {{\"type\": \"<type>\", \"scope\": \"<scope>\", \
                 \"description\": \"<description>\", \"body\": \"<body>\"}}\n\
                 Error was: {first_err}"
            );
            let raw2 = provider
                .chat(&retry_system, user)
                .await
                .context("provider retry call")?;
            let parsed2 = crate::parser::parse_commit_message(&raw2)
                .context("parse commit message after retry")?;
            Ok(parsed2.to_conventional())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn retry_rejects_garbage_then_accepts_valid() {
        let bad = "this is not JSON at all".to_string();
        let good = "{\"type\":\"fix\",\"scope\":\"retry\",\"description\":\"handle parse error\",\"body\":\"retried once\"}".to_string();
        let provider = AnyProvider::Mock(mock::MockProvider::with_responses(vec![bad, good]));
        let cfg = Config::default();

        let result = generate_commit_message(&provider, "fake diff", &cfg).await;
        assert!(result.is_ok(), "retry should succeed: {:?}", result.err());
        let msg = result.unwrap();
        assert_eq!(msg, "fix(retry): handle parse error\n\nretried once");
    }

    #[tokio::test]
    async fn retry_fails_on_two_garbage_responses() {
        let bad = "definitely not json".to_string();
        let provider =
            AnyProvider::Mock(mock::MockProvider::with_responses(vec![bad.clone(), bad]));
        let cfg = Config::default();

        let result = generate_commit_message(&provider, "fake diff", &cfg).await;
        assert!(result.is_err(), "two garbage responses should fail");
    }
}
