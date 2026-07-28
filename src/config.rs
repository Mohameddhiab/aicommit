//! Configuration loading and merging.
//!
//! Resolution order (highest priority first):
//!   1. CLI flags (--provider, --model, --lang, --api-key)
//!   2. Environment variables (OPENAI_API_KEY, ANTHROPIC_API_KEY, …)
//!   3. Project config: ./.doctor.toml
//!   4. Global config: ~/.doctor.toml
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

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct OllamaConfig {
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
}

fn default_ollama_url() -> String {
    "http://localhost:11434".to_string()
}
fn default_ollama_model() -> String {
    "qwen2.5-coder:7b".to_string()
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct RemoteProviderConfig {
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub model: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct CommitConfig {
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub max_commits: Option<usize>,
    #[serde(default)]
    pub timeout_secs: Option<u64>,
    #[serde(default)]
    pub exclude_patterns: Option<Vec<String>>,
    #[serde(default)]
    pub template: Option<String>,
}

fn default_lang() -> String {
    "en".to_string()
}
fn default_max_commits() -> usize {
    3
}
fn default_timeout() -> u64 {
    120
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
    pub ollama_url: String,
    pub ollama_model: String,
    pub remote: RemoteProviderConfig,
    pub commit_language: String,
    pub commit_max_commits: usize,
    pub commit_timeout_secs: u64,
    pub exclude_patterns: Vec<String>,
    pub commit_template: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            provider: ProviderKind::Mock,
            ollama_url: default_ollama_url(),
            ollama_model: default_ollama_model(),
            remote: RemoteProviderConfig::default(),
            commit_language: default_lang(),
            commit_max_commits: default_max_commits(),
            commit_timeout_secs: default_timeout(),
            exclude_patterns: vec![],
            commit_template: default_template(),
        }
    }
}

fn default_template() -> String {
    "{type}{scope}: {description}".to_string()
}

/// Load and merge configuration from all sources.
pub fn load(
    cli_provider: Option<String>,
    cli_model: Option<String>,
    cli_lang: Option<String>,
    cli_api_key: Option<String>,
    cli_base_url: Option<String>,
    cli_timeout: u64,
) -> Result<Config> {
    let file = load_file()?;
    let provider = resolve_provider(&file, cli_provider.as_deref(), cli_api_key.as_deref())?;
    let remote_base = remote_section(&file, provider);
    let remote = RemoteProviderConfig {
        api_key: cli_api_key
            .or_else(|| env_api_key(provider))
            .or_else(|| remote_base.api_key.clone()),
        base_url: cli_base_url.clone().or(remote_base.base_url.clone()),
        model: cli_model.clone().or(remote_base.model.clone()),
    };
    let ollama_url = cli_base_url
        .clone()
        .or(file.ollama.url)
        .unwrap_or_else(default_ollama_url);
    let ollama_model = cli_model
        .clone()
        .or(file.ollama.model)
        .unwrap_or_else(default_ollama_model);
    let commit_language = cli_lang
        .or(file.commit.language)
        .unwrap_or_else(default_lang);
    let commit_max_commits = file.commit.max_commits.unwrap_or_else(default_max_commits);
    let commit_timeout_secs = cli_timeout;
    let exclude_patterns = file.commit.exclude_patterns.unwrap_or_default();
    let commit_template = file.commit.template.unwrap_or_else(default_template);
    Ok(Config {
        provider,
        ollama_url,
        ollama_model,
        remote,
        commit_language,
        commit_max_commits,
        commit_timeout_secs,
        exclude_patterns,
        commit_template,
    })
}

fn load_file() -> Result<ConfigFile> {
    let local = std::env::current_dir()
        .unwrap_or_default()
        .join(".doctor.toml");
    let global = global_config_path();
    let merged = merge_files(local, global)?;
    Ok(merged)
}

fn global_config_path() -> PathBuf {
    dirs::config_dir()
        .map(|d| d.join("doctor").join("config.toml"))
        .or_else(|| dirs::home_dir().map(|h| h.join(".doctor.toml")))
        .unwrap_or_else(|| PathBuf::from(".doctor.toml"))
}

fn read_file(path: &PathBuf) -> Result<ConfigFile> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let parsed: ConfigFile =
        toml::from_str(&raw).with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(parsed)
}

fn merge_files(local: PathBuf, global: PathBuf) -> Result<ConfigFile> {
    let mut acc = if global.exists() {
        read_file(&global)?
    } else {
        ConfigFile::default()
    };
    if local.exists() {
        let l = read_file(&local)?;
        if l.provider.default.is_some() {
            acc.provider.default = l.provider.default;
        }
        merge_optional(&mut acc.ollama.url, l.ollama.url);
        merge_optional(&mut acc.ollama.model, l.ollama.model);
        merge_remote(&mut acc.openai, l.openai);
        merge_remote(&mut acc.anthropic, l.anthropic);
        merge_remote(&mut acc.groq, l.groq);
        merge_remote(&mut acc.deepseek, l.deepseek);
        merge_remote(&mut acc.mistral, l.mistral);
        merge_remote(&mut acc.gemini, l.gemini);
        merge_remote(&mut acc.openrouter, l.openrouter);
        merge_optional(&mut acc.commit.language, l.commit.language);
        merge_optional(&mut acc.commit.max_commits, l.commit.max_commits);
        merge_optional(&mut acc.commit.timeout_secs, l.commit.timeout_secs);
    }
    Ok(acc)
}

fn merge_optional<T>(dst: &mut Option<T>, src: Option<T>) {
    if src.is_some() {
        *dst = src;
    }
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
    let ollama_url = file
        .ollama
        .url
        .as_deref()
        .unwrap_or("http://localhost:11434");
    if ollama_reachable(ollama_url) {
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
        "no AI provider configured\n\
          → run: doctor config --provider <name> --api-key <key>\n\
         → or install Ollama from https://ollama.com"
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

/// Handle `doctor config` subcommand.
pub async fn handle_config_command(
    api_key: Option<String>,
    provider: Option<String>,
    model: Option<String>,
    base_url: Option<String>,
    show: bool,
) -> Result<()> {
    if show {
        let cfg = load(None, None, None, None, None, 120)?;
        println!("{cfg:#?}");
        return Ok(());
    }
    let provider_kind = match provider {
        Some(p) => p.parse::<ProviderKind>()?,
        None => return Err(anyhow!("--provider is required")),
    };
    let path = global_config_path();
    let mut file = if path.exists() {
        read_file(&path)?
    } else {
        ConfigFile::default()
    };
    // Write the section for this provider.
    let section = match provider_kind {
        ProviderKind::Openai => &mut file.openai,
        ProviderKind::Anthropic => &mut file.anthropic,
        ProviderKind::Groq => &mut file.groq,
        ProviderKind::Deepseek => &mut file.deepseek,
        ProviderKind::Mistral => &mut file.mistral,
        ProviderKind::Gemini => &mut file.gemini,
        ProviderKind::Openrouter => &mut file.openrouter,
        ProviderKind::Mock | ProviderKind::Ollama => {
            return Err(anyhow!(
                "{} does not use remote config",
                provider_kind.as_str()
            ))
        }
    };
    if let Some(k) = api_key {
        section.api_key = Some(k);
    }
    if let Some(m) = model {
        section.model = Some(m);
    }
    if let Some(u) = base_url {
        section.base_url = Some(u);
    }
    let serialized = toml::to_string_pretty(&file).context("serialize config")?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    std::fs::write(&path, serialized).with_context(|| format!("write {}", path.display()))?;
    restrict_file_permissions(&path);
    println!(
        "Saved config for {} in {}",
        provider_kind.as_str(),
        path.display()
    );
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
