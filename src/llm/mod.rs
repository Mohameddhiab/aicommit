pub mod anthropic;
pub mod gemini;
pub mod ollama;
pub mod openai_compat;

use crate::config::{Config, ProviderKind};
use crate::parser::CommitMessage;
use crate::prompt;
use anyhow::{anyhow, Context, Result};

/// Concrete provider enum — each variant wraps one backend.
/// Avoids trait-object complexity with async_trait.
pub enum AnyProvider {
    Ollama(ollama::OllamaProvider),
    OpenAiCompat(openai_compat::OpenAiCompatProvider),
    Anthropic(anthropic::AnthropicProvider),
    Gemini(gemini::GeminiProvider),
}

impl AnyProvider {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Ollama(_) => "ollama",
            Self::OpenAiCompat(_) => "openai-compatible",
            Self::Anthropic(_) => "anthropic",
            Self::Gemini(_) => "gemini",
        }
    }

    pub async fn chat(&self, system: &str, user: &str) -> Result<String> {
        match self {
            Self::Ollama(p) => p.chat(system, user).await,
            Self::OpenAiCompat(p) => p.chat(system, user).await,
            Self::Anthropic(p) => p.chat(system, user).await,
            Self::Gemini(p) => p.chat(system, user).await,
        }
    }
}

/// Build the concrete provider from the resolved config.
pub fn build_provider(cfg: &Config) -> Result<AnyProvider> {
    match cfg.provider {
        ProviderKind::Ollama => Ok(AnyProvider::Ollama(ollama::OllamaProvider::new(
            cfg.ollama.clone(),
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
                openai_compat::OpenAiCompatProvider::new(base_url, api_key, model),
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
                base_url, api_key, model,
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
                base_url, api_key, model,
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
pub async fn generate_commit_message(
    provider: &AnyProvider,
    diff: &str,
    cfg: &Config,
) -> Result<String> {
    let system = prompt::system_prompt(cfg);
    let user = prompt::user_prompt(diff);
    let raw = provider
        .chat(&system, &user)
        .await
        .context("provider chat call")?;
    let parsed: CommitMessage =
        crate::parser::parse_commit_message(&raw).context("parse commit message")?;
    Ok(parsed.to_conventional())
}
