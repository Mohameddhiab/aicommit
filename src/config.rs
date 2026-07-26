//! Configuration loading and merging.
//!
//! Resolution order (highest priority first):
//!   1. CLI flags (--provider, --model, --lang, --api-key)
//!   2. Environment variables (OPENAI_API_KEY, ANTHROPIC_API_KEY, …)
//!   3. Project config: ./.aicommit.toml
//!   4. Global config: ~/.aicommit.toml
//!   5. Hardcoded defaults

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// All supported providers.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProviderKind {
    Mock,
    Ollama,
    Openai,
    Anthropic,
    Groq,
    Deepseek,
    Mistral,
    Gemini,
    Openrouter,
}

impl ProviderKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mock => "mock",
            Self::Ollama => "ollama",
            Self::Openai => "openai",
            Self::Anthropic => "anthropic",
            Self::Groq => "groq",
            Self::Deepseek => "deepseek",
            Self::Mistral => "mistral",
            Self::Gemini => "gemini",
            Self::Openrouter => "openrouter",
        }
    }

    /// Environment variable read for the API key of this provider.
    pub fn env_var(self) -> Option<&'static str> {
        match self {
            Self::Mock => None,
            Self::Ollama => None,
            Self::Openai => Some("OPENAI_API_KEY"),
            Self::Anthropic => Some("ANTHROPIC_API_KEY"),
            Self::Groq => Some("GROQ_API_KEY"),
            Self::Deepseek => Some("DEEPSEEK_API_KEY"),
            Self::Mistral => Some("MISTRAL_API_KEY"),
            Self::Gemini => Some("GOOGLE_API_KEY"),
            Self::Openrouter => Some("OPENROUTER_API_KEY"),
        }
    }

    /// True if this provider speaks the OpenAI-compatible chat completions API.
    pub fn is_openai_compatible(self) -> bool {
        matches!(
            self,
            Self::Openai | Self::Groq | Self::Deepseek | Self::Mistral | Self::Openrouter
        )
    }
}

impl std::str::FromStr for ProviderKind {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        match s.to_ascii_lowercase().as_str() {
            "mock" => Ok(Self::Mock),
            "ollama" => Ok(Self::Ollama),
            "openai" => Ok(Self::Openai),
            "anthropic" => Ok(Self::Anthropic),
            "groq" => Ok(Self::Groq),
            "deepseek" => Ok(Self::Deepseek),
            "mistral" => Ok(Self::Mistral),
            "gemini" => Ok(Self::Gemini),
            "openrouter" => Ok(Self::Openrouter),
            other => Err(anyhow!("unknown provider '{other}'")),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ProviderConfig {
    pub default: Option<ProviderKind>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OllamaConfig {
    #[serde(default = "default_ollama_url")]
    pub url: String,
    #[serde(default = "default_ollama_model")]
    pub model: String,
}

fn default_ollama_url() -> String {
    "http://localhost:11434".to_string()
}
fn default_ollama_model() -> String {
    "qwen2.5-coder:7b".to_string()
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            url: default_ollama_url(),
            model: default_ollama_model(),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct RemoteProviderConfig {
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub model: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct CommitConfig {
    #[serde(default = "default_lang")]
    pub language: String,
    #[serde(default = "default_max_commits")]
    pub max_commits: usize,
}

fn default_lang() -> String {
    "en".to_string()
}
fn default_max_commits() -> usize {
    3
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ConfigFile {
    pub provider: ProviderConfig,
    #[serde(default)]
    pub ollama: OllamaConfig,
    #[serde(default)]
    pub openai: RemoteProviderConfig,
    #[serde(default)]
    pub anthropic: RemoteProviderConfig,
    #[serde(default)]
    pub groq: RemoteProviderConfig,
    #[serde(default)]
    pub deepseek: RemoteProviderConfig,
    #[serde(default)]
    pub mistral: RemoteProviderConfig,
    #[serde(default)]
    pub gemini: RemoteProviderConfig,
    #[serde(default)]
    pub openrouter: RemoteProviderConfig,
    #[serde(default)]
    pub commit: CommitConfig,
}

/// Fully-resolved configuration used across the app.
#[derive(Clone, Debug)]
pub struct Config {
    pub provider: ProviderKind,
    pub ollama: OllamaConfig,
    pub remote: RemoteProviderConfig,
    pub commit: CommitConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            provider: ProviderKind::Mock,
            ollama: OllamaConfig::default(),
            remote: RemoteProviderConfig::default(),
            commit: CommitConfig::default(),
        }
    }
}

/// Load and merge configuration from all sources.
///
/// `cli_provider`, `cli_model`, `cli_lang`, `cli_api_key` are the override values
/// coming from the command line (highest priority).
pub fn load(
    cli_provider: Option<String>,
    cli_model: Option<String>,
    cli_lang: Option<String>,
    cli_api_key: Option<String>,
) -> Result<Config> {
    let file = load_file()?;
    let provider = resolve_provider(&file, cli_provider.as_deref(), cli_api_key.as_deref())?;
    let remote_base = remote_section(&file, provider);
    let remote = RemoteProviderConfig {
        api_key: cli_api_key
            .or_else(|| env_api_key(provider))
            .or_else(|| remote_base.api_key.clone()),
        base_url: remote_base.base_url.clone(),
        model: cli_model.clone().or(remote_base.model.clone()),
    };
    let ollama = OllamaConfig {
        url: file.ollama.url.clone(),
        model: cli_model.unwrap_or_else(|| file.ollama.model.clone()),
    };
    let commit = CommitConfig {
        language: cli_lang.unwrap_or(file.commit.language.clone()),
        max_commits: file.commit.max_commits,
    };
    Ok(Config {
        provider,
        ollama,
        remote,
        commit,
    })
}

fn load_file() -> Result<ConfigFile> {
    let local = std::env::current_dir()
        .unwrap_or_default()
        .join(".aicommit.toml");
    let global = global_config_path();
    let merged = merge_files(local, global)?;
    Ok(merged)
}

fn global_config_path() -> PathBuf {
    dirs::config_dir()
        .map(|d| d.join("aicommit").join("config.toml"))
        .or_else(|| dirs::home_dir().map(|h| h.join(".aicommit.toml")))
        .unwrap_or_else(|| PathBuf::from(".aicommit.toml"))
}

fn read_file(path: &PathBuf) -> Result<ConfigFile> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let parsed: ConfigFile =
        toml::from_str(&raw).with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(parsed)
}

fn merge_files(local: PathBuf, global: PathBuf) -> Result<ConfigFile> {
    let mut acc = ConfigFile::default();
    if global.exists() {
        acc = read_file(&global)?;
    }
    if local.exists() {
        let l = read_file(&local)?;
        // Local overrides global where present.
        if l.provider.default.is_some() {
            acc.provider.default = l.provider.default;
        }
        if !l.ollama.url.is_empty() {
            acc.ollama = l.ollama;
        }
        merge_remote(&mut acc.openai, l.openai);
        merge_remote(&mut acc.anthropic, l.anthropic);
        merge_remote(&mut acc.groq, l.groq);
        merge_remote(&mut acc.deepseek, l.deepseek);
        merge_remote(&mut acc.mistral, l.mistral);
        merge_remote(&mut acc.gemini, l.gemini);
        merge_remote(&mut acc.openrouter, l.openrouter);
        if !l.commit.language.is_empty() || l.commit.max_commits != 0 {
            acc.commit = l.commit;
        }
    }
    Ok(acc)
}

fn merge_remote(dst: &mut RemoteProviderConfig, src: RemoteProviderConfig) {
    if src.api_key.is_some() {
        dst.api_key = src.api_key;
    }
    if src.base_url.is_some() {
        dst.base_url = src.base_url;
    }
    if src.model.is_some() {
        dst.model = src.model;
    }
}

fn env_api_key(provider: ProviderKind) -> Option<String> {
    match provider.env_var() {
        Some(name) => std::env::var(name).ok().filter(|s| !s.is_empty()),
        None => None,
    }
}

fn remote_section(file: &ConfigFile, provider: ProviderKind) -> RemoteProviderConfig {
    match provider {
        ProviderKind::Mock => RemoteProviderConfig::default(),
        ProviderKind::Ollama => RemoteProviderConfig::default(),
        ProviderKind::Openai => file.openai.clone(),
        ProviderKind::Anthropic => file.anthropic.clone(),
        ProviderKind::Groq => file.groq.clone(),
        ProviderKind::Deepseek => file.deepseek.clone(),
        ProviderKind::Mistral => file.mistral.clone(),
        ProviderKind::Gemini => file.gemini.clone(),
        ProviderKind::Openrouter => file.openrouter.clone(),
    }
}

/// Resolve provider with auto-detection:
///   CLI > file.default > Ollama if reachable > first env-var match > error.
fn resolve_provider(
    file: &ConfigFile,
    cli: Option<&str>,
    _cli_api_key: Option<&str>,
) -> Result<ProviderKind> {
    if let Some(name) = cli {
        return name.parse::<ProviderKind>().context("invalid --provider");
    }
    if let Some(default) = file.provider.default {
        return Ok(default);
    }
    // Auto-detect.
    if ollama_reachable(&file.ollama.url) {
        return Ok(ProviderKind::Ollama);
    }
    for kind in [
        ProviderKind::Anthropic,
        ProviderKind::Openai,
        ProviderKind::Groq,
        ProviderKind::Deepseek,
        ProviderKind::Mistral,
        ProviderKind::Gemini,
        ProviderKind::Openrouter,
    ] {
        if env_api_key(kind).is_some() {
            return Ok(kind);
        }
    }
    Err(anyhow!(
        "no AI provider configured. \
         Run `aicommit config --provider <name> --api-key <key>` to set one, \
         or install Ollama from https://ollama.com"
    ))
}

fn ollama_reachable(url: &str) -> bool {
    // Sync check via a minimal HTTP request.
    let endpoint = format!("{}/api/tags", url.trim_end_matches('/'));
    reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .ok()
        .and_then(|client| client.get(&endpoint).send().ok())
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

/// Handle `aicommit config` subcommand.
pub async fn handle_config_command(
    api_key: Option<String>,
    provider: Option<String>,
    show: bool,
) -> Result<()> {
    if show {
        let cfg = load(None, None, None, None)?;
        println!("{cfg:#?}");
        return Ok(());
    }
    let provider_kind = match provider {
        Some(p) => p.parse::<ProviderKind>()?,
        None => return Err(anyhow!("--provider is required when setting an API key")),
    };
    let key = match api_key {
        Some(k) => k,
        None => return Err(anyhow!("--api-key is required")),
    };
    write_api_key(provider_kind, &key)?;
    println!(
        "Saved API key for {} in {}",
        provider_kind.as_str(),
        global_config_path().display()
    );
    Ok(())
}

fn write_api_key(provider: ProviderKind, key: &str) -> Result<()> {
    let path = global_config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let mut file = if path.exists() {
        read_file(&path)?
    } else {
        ConfigFile::default()
    };
    match provider {
        ProviderKind::Openai => file.openai.api_key = Some(key.to_string()),
        ProviderKind::Anthropic => file.anthropic.api_key = Some(key.to_string()),
        ProviderKind::Groq => file.groq.api_key = Some(key.to_string()),
        ProviderKind::Deepseek => file.deepseek.api_key = Some(key.to_string()),
        ProviderKind::Mistral => file.mistral.api_key = Some(key.to_string()),
        ProviderKind::Gemini => file.gemini.api_key = Some(key.to_string()),
        ProviderKind::Openrouter => file.openrouter.api_key = Some(key.to_string()),
        ProviderKind::Mock => return Err(anyhow!("Mock provider does not need an API key")),
        ProviderKind::Ollama => return Err(anyhow!("Ollama does not need an API key")),
    }
    let serialized = toml::to_string_pretty(&file).context("serialize config")?;
    std::fs::write(&path, serialized).with_context(|| format!("write {}", path.display()))?;
    restrict_file_permissions(&path);
    Ok(())
}

#[cfg(unix)]
fn restrict_file_permissions(path: &PathBuf) {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(meta) = std::fs::metadata(path) {
        let mut perm = meta.permissions();
        perm.set_mode(0o600);
        let _ = std::fs::set_permissions(path, perm);
    }
}

#[cfg(not(unix))]
fn restrict_file_permissions(_path: &PathBuf) {
    // No-op on non-Unix: file ACLs are user-managed by the OS.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_kind_roundtrip() {
        for p in [
            ProviderKind::Mock,
            ProviderKind::Ollama,
            ProviderKind::Openai,
            ProviderKind::Anthropic,
            ProviderKind::Groq,
            ProviderKind::Deepseek,
            ProviderKind::Mistral,
            ProviderKind::Gemini,
            ProviderKind::Openrouter,
        ] {
            assert_eq!(p.as_str().parse::<ProviderKind>().unwrap(), p);
        }
    }

    #[test]
    fn openai_compatible_set() {
        assert!(ProviderKind::Openai.is_openai_compatible());
        assert!(ProviderKind::Groq.is_openai_compatible());
        assert!(ProviderKind::Deepseek.is_openai_compatible());
        assert!(ProviderKind::Mistral.is_openai_compatible());
        assert!(ProviderKind::Openrouter.is_openai_compatible());
        assert!(!ProviderKind::Ollama.is_openai_compatible());
        assert!(!ProviderKind::Anthropic.is_openai_compatible());
        assert!(!ProviderKind::Gemini.is_openai_compatible());
    }
}
