# aicommit — Project Brief

CLI Rust ultra-rapide qui génère des commits parfaits et atomiques en 1 seconde.
Fini les "wip" et "fix bug". Une seule commande. Des commits conventionnels, respectant l'historique Git, propulsés par l'IA locale ou cloud.

## Vue d'ensemble

| Champ | Détail |
|-------|--------|
| Nom | aicommit |
| Type | CLI open source |
| Langage | Rust |
| Distribution | crates.io · Homebrew · npx shim |
| Cible | Tout développeur utilisant Git (donc 100% des développeurs) |
| Concurrent direct | aicommits (Node.js, ~7k stars) |
| Avantage clé | 10x plus rapide · Commits atomiques (split de diff) · 100% Offline (Ollama) |
| Durée V1 | 3 semaines |

## Problème résolu

Faire un bon commit prend du temps :

- `git add -p` (sélectionner manuellement les blocs de code).
- Réfléchir à un message clair et conventionnel (`feat: ...` vs `fix: ...`).
- Écrire la commande `git commit -m "..."`.

Les développeurs pressés font ça 10 fois par jour :

```bash
git commit -m "wip"
git commit -m "fix bug auth"
git commit -m "update stuff"
```

Cela détruit l'historique du projet, empêche les LLMs de comprendre les évolutions, et bloque les revues de code.

aicommit résout ce workflow en une commande :

```bash
git add .
aicommit
# > Analyzing diff... (340 lines)
# > 2 logical changes detected.
# 1. feat(auth): implement JWT token expiration
# 2. refactor(db): remove unused Redis client pool
# Select commit [1/2] or generate all [a]:
```

Il lit le `git diff staged`, le compresse intelligemment, gère le découpage en commits atomiques (un commit par fonctionnalité), génère le message parfait, et fait le commit pour toi.

## Commandes principales

```bash
# L'action par défaut : lit le diff staged, propose et commit
aicommit

# Forcer un seul gros commit (mode classique)
aicommit --single

# Mode interactif avec édition du message avant commit
aicommit --interactive

# Spécifier le provider (local par défaut)
aicommit --provider ollama --model llama3

# Spécifier le provider (Cloud)
aicommit --provider openai --model gpt-4o

# Override de la config
aicommit --lang fr  # génère les commits en français
```

## Architecture du projet

```text
aicommit/
├── src/
│   ├── main.rs           # CLI entry point (clap, tokio)
│   ├── git.rs            # wrapper git2: diff, add, commit, status
│   ├── llm.rs            # abstraction API (Ollama, OpenAI, Anthropic)
│   ├── prompt.rs         # construction du prompt system + user
│   ├── parser.rs         # parsing de la réponse LLM (JSON/Texte)
│   ├── splitter.rs       # logique de split pour commits atomiques
│   └── config.rs         # .aicommit.toml
├── tests/
│   └── fixtures/         # faux repos git pour tests
├── Cargo.toml
└── README.md
```

## Cargo.toml

```toml
[package]
name = "aicommit"
version = "0.1.0"
edition = "2021"
description = "Generates perfect, atomic Git commits using AI — fast, local, reliable"
license = "MIT"
repository = "https://github.com/Mohameddhiab/aicommit"
keywords = ["git", "ai", "cli", "commit", "developer-tools"]

[[bin]]
name = "aicommit"
path = "src/main.rs"

[dependencies]
clap = { version = "4", features = ["derive"] }
git2 = "0.18"
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.11", features = ["json", "rustls-tls"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
colored = "2"
dialoguer = "0.11"
anyhow = "1"

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true
```

## La Killer Feature : Commits Atomiques (splitter.rs)

La plupart des outils actuels prennent tout le `git diff` et font un seul énorme commit. aicommit se démarque en créant des commits atomiques.

Comment ça marche ?

1. aicommit lit les fichiers modifiés.
2. Il groupe les fichiers par domaine logique (ex: `src/auth/*`, `src/db/*`, `package.json`).
3. Si le diff est trop gros, il propose à l'utilisateur de splitter en plusieurs commits.
4. Il exécute le LLM pour chaque groupe, génère le message, et fait `git commit` groupe par groupe.

Exemple : tu as modifié `auth.rs`, `db.rs` et `README.md` en même temps. aicommit va te proposer :

- Commit 1: `feat(auth): add JWT token validation`
- Commit 2: `fix(db): handle connection timeout on large pool`
- Commit 3: `docs: update README with new auth flow`

## Gestion des LLMs (llm.rs)

Contrairement à aicommits (Node), aicommit est Privacy-First. Par défaut, l'outil cherche un serveur local Ollama. Si trouvé, Ollama est utilisé (`llama3` ou `qwen2.5-coder`). Zéro données envoyées sur internet.

Si l'utilisateur a configuré une clé API OpenAI/Anthropic, `aicommit config --api-key sk-...`, il utilisera le cloud.

### Providers supportés (V1)

| Provider | Modèles notables | Auth | Local/Cloud |
|----------|------------------|------|-------------|
| Ollama | `qwen2.5-coder:7b`, `llama3`, `deepseek-coder` | Aucune | Local |
| OpenAI | `gpt-4o`, `gpt-4o-mini`, `o1-mini` | `OPENAI_API_KEY` | Cloud |
| Anthropic | `claude-3.5-sonnet`, `claude-3-haiku` | `ANTHROPIC_API_KEY` | Cloud |
| Mistral | `mistral-large`, `codestral` | `MISTRAL_API_KEY` | Cloud |
| Groq | `llama-3.3-70b`, `mixtral-8x7b` | `GROQ_API_KEY` | Cloud (rapide) |
| DeepSeek | `deepseek-chat`, `deepseek-coder` | `DEEPSEEK_API_KEY` | Cloud |
| Gemini | `gemini-2.0-flash`, `gemini-1.5-pro` | `GOOGLE_API_KEY` | Cloud |
| OpenRouter | agrégateur (accès à tous) | `OPENROUTER_API_KEY` | Cloud |

## Stratégie de Prompt (prompt.rs)

Le prompt est optimisé pour forcer le format Conventional Commits :

```text
You are an AI expert in Git.
Generate a commit message following the Conventional Commits specification.
Format: <type>(<scope>): <description>

Types: feat, fix, docs, style, refactor, perf, test, build, ci, chore
Output ONLY the commit message, 1 or 2 lines max, no markdown, no quotes.
```

## Configuration (.aicommit.toml)

Placé à la racine du projet (ou global dans `~/.aicommit.toml`) :

```toml
[provider]
default = "ollama"  # ou "openai", "anthropic", "groq", "deepseek", "mistral", "gemini", "openrouter"

[ollama]
url = "http://localhost:11434"
model = "qwen2.5-coder:7b"

[openai]
api_key = "sk-..."
model = "gpt-4o"

[anthropic]
api_key = "sk-ant-..."
model = "claude-3.5-sonnet"

[groq]
api_key = "gsk_..."
model = "llama-3.3-70b-versatile"

[deepseek]
api_key = "sk-..."
model = "deepseek-coder"

[mistral]
api_key = "..."
model = "codestral"

[gemini]
api_key = "..."
model = "gemini-2.0-flash"

[openrouter]
api_key = "..."
model = "anthropic/claude-3.5-sonnet"

[commit]
language = "en"  # ou "fr"
max_commits = 3
```

## Plan 3 semaines

### Semaine 1 — Core & Git Introspection

- `cargo init aicommit` · Structure des modules
- `git.rs` — Utilisation de `git2` pour lire le diff staged sans dépendre au CLI externe (git)
- `llm.rs` — Appel basique à Ollama (Local) et OpenAI (Cloud). Retourne un String ou JSON.
- `main.rs` — CLI Clap. Commande `aicommit` et `aicommit config`.
- Output en stdout: le message généré.

Objectif fin S1 : aicommit lit le git diff staged et fait un commit basique via Ollama.

### Semaine 2 — Atomic Splits & Interactive

- `splitter.rs` — Logique de groupage par fichier (`src/auth/`, `src/db/`).
- `parser.rs` — Parsing strict de la réponse LLM.
- `dialoguer` — Menu interactif pour choisir quel commit appliquer.

Objectif fin S2 : L'outil gère les gros diffs et ne plante pas le LLM (Réponse JSON).

### Semaine 3 — Polish & Cloud Providers

- Support OpenAI (`gpt-4o`) et Claude 3.5 Sonnet.
- Support Anthropic API.
- Fichier de config global `~/.aicommit.toml`.
- CI GitHub Actions (tests, build, release).
- Gestion d'erreurs anyhow.

Objectif fin S3 : Outil stable, testé, prêt pour Homebrew/Cargo.

## Stratégie launch détaillée

Le marché cible n'est pas les développeurs Rust (ils aiment la perf), mais tous les développeurs. Tout le monde fait des commits.

### Hook principal (README + posts)

"J'ai 3 commits à écrire. wip, update, fix. L'écriture manuelle des messages de commit, une perte de temps de 15 minutes par jour. aicommit le fait en 1 seconde."

### Benchmark Table

| Tool | Repo size | Time | Langage |
|------|-----------|------|---------|
| aicommits (Node) | 5k files | ~8s | Node bundle |
| aicommit (Rust) | 5k files | ~0.4s | ~2 MB |

Le gain de vitesse (Rust + requêtes asynchrones + binaire natif) justifie l'utilisation de aicommit même pour les petits repos.

## Canaux de distribution

| Canal | Message |
|-------|---------|
| r/rust | Focus sur la vitesse et la privacy (Ollama par défaut) |
| Show HN | Focus sur le workflow quotidien (Commit) et l'automatisation |
| X/Twitter | GIF de l'outil en action |
| Reddit (r/LocalLLaMA) | Focus sur les modèles locaux (Ollama) |

## Install one-liner

```bash
# Cargo
cargo install aicommit

# Homebrew
brew install Mohameddhiab/tap/aicommit

# npx
npx aicommit
```

## Différenciateurs vs aicommits (Node.js)

| Feature | aicommits | aicommit |
|---------|-----------|----------|
| Langage | Node.js | Rust |
| Privacy | Cloud seulement | Local First (Ollama) |
| Vitesse | ~8s (5k files) | ~0.4s |
| Commits Atomiques | Non | Oui (Split par domaine) |
| Conventionnel Commits | Basique | Oui (Strict Conventional Commits) |
| Multi-provider | Non | Oui (Ollama, OpenAI, Anthropic, Groq, DeepSeek, Mistral, Gemini, OpenRouter) |
| Config globale | Local seulement | Global + Projet |

## Ce qui fait la viralité (Pourquoi les devs vont l'adopter)

- **Local-First** : Avec Ollama par défaut, le dev moyen n'a pas besoin de payer OpenAI. C'est un énorme avantage pour les entreprises qui interdisent l'envoi de code source sur le cloud.
- **Atomic Commits** : C'est la feature qui va faire parler d'elle sur Reddit/HN. 100% des devs font des commits "wip" plusieurs fois par jour. Le gain de temps est immédiat.
- **Vitesse** : Lancer aicommit sur un repo de 10k lignes prend 0.4s. Ça donne l'impression d'un outil "binaire" (C/C++).

## Métriques de succès

| Horizon | Stars cible |
|---------|-------------|
| Fin semaine 3 | 1k–3k |
| +1 mois | 5k–10k |
| +6 mois | 20k–40k |
| 100k | Scénario viral (Ollama local + HN front page) |

## Liens utiles

- Repo : https://github.com/Mohameddhiab/aicommit
- Concurrent : https://github.com/di-suk/aicommits
