# aicommit — Design System (CLI UI/UX)

Ce document définit le système visuel et les flux d'interaction du CLI `aicommit` : couleurs, typographie, composants, écrans d'installation, et gestion d'erreurs. Objectif : cohérence entre toutes les commandes, et une première impression (`install` → premier commit) irréprochable.

Basé sur les principes des [Command Line Interface Guidelines](https://clig.dev) : humain d'abord, scriptable ensuite, silencieux par défaut, jamais surprenant.

---

## 1. Principes directeurs

1. **Jamais destructif par surprise** — toute action qui modifie le repo (commit, reset) doit être confirmée explicitement. Annuler = ne rien faire, jamais "tout faire".
2. **Dégradation gracieuse** — fonctionne sans couleur (`NO_COLOR`), sans TTY (CI/scripts), sur terminal étroit.
3. **Un seul système de couleur sémantique** — la couleur porte toujours le même sens, jamais juste décorative.
4. **La densité d'info suit l'intention** — `--dry-run` montre tout, le mode normal montre l'essentiel, `--quiet` ne montre rien sauf erreur.

---

## 2. Palette de couleurs sémantiques

| Rôle | Couleur | Usage `colored` | Exemple |
|---|---|---|---|
| Marque / structure | Cyan | `.cyan()` | Bordures de boîtes, banner, titres de section |
| Succès | Vert | `.green()` | Commit réussi, check de connexion OK |
| Erreur | Rouge | `.red().bold()` | Échec, refus, config invalide |
| Avertissement | Jaune | `.yellow()` | Diff volumineux, provider non testé, fallback |
| Info / secondaire | Gris (bright black) | `.dimmed()` | Métadonnées, hints, stats de fichiers |
| Sélection active | Magenta | `.magenta().bold()` | Item survolé/coché dans un `MultiSelect` |
| Texte principal | Blanc/défaut | — | Messages de commit, contenu |

**Règle stricte** : jamais deux couleurs différentes pour le même type d'info entre deux commandes. Si `git.rs` affiche une erreur, elle doit avoir le même style qu'une erreur de `llm/mod.rs`.

**Accessibilité** : respecter `NO_COLOR` (variable d'env standard) et détecter le TTY. Ajouter au démarrage dans `main.rs` :

```rust
if std::env::var("NO_COLOR").is_ok() || !std::io::stdout().is_terminal() {
    colored::control::set_override(false);
}
```

Et exposer un flag explicite `--no-color` dans `clap` qui fait pareil (override manuel prioritaire sur l'auto-détection).

---

## 3. Iconographie

| Symbole | Sens | Fallback ASCII (si `--ascii` ou terminal non-Unicode) |
|---|---|---|
| `✓` | Succès | `[OK]` |
| `✗` | Erreur | `[X]` |
| `⚠` | Avertissement | `[!]` |
| `ℹ` | Info | `[i]` |
| `●` / `○` | Sélectionné / non-sélectionné | `[x]` / `[ ]` |
| `→` | Action suivante / hint | `->` |
| `⋯` (via `indicatif`) | Chargement | `...` |

---

## 4. Composant Boîte (fix du bug d'alignement)

Le composant actuel (`display.rs`) casse sur Unicode multi-byte et ne s'adapte pas à la largeur du terminal. Nouvelle spec :

- Ajouter la dépendance `unicode-width = "0.1"` pour calculer la largeur d'affichage réelle, pas `.len()` en bytes.
- Ajouter `terminal_size = "0.3"` pour lire la largeur du terminal ; largeur de boîte = `min(terminal_width - 2, 76)`, minimum `40`.
- Si le contenu dépasse la largeur dispo, tronquer avec `…` plutôt que de casser l'alignement.

```
╭─ Dry run — 3 groups detected ──────────────────────────────╮
│  ● feat(auth)   3 files   +142 -8    Add JWT refresh flow  │
│  ● fix(api)     1 file    +6   -2    Fix null check in...  │
│  ○ chore(deps)  2 files   +12  -0    Bump reqwest to 0.12  │
╰──────────────────────────────────────────────────────────────╯
```

---

## 5. États de sortie (output states)

Chaque état a un format fixe : **symbole + couleur + message court + détail optionnel en dimmed en dessous.**

**Succès**
```
✓ Committed 3 groups (feat(auth), fix(api), chore(deps))
```

**Erreur** — toujours 3 lignes : quoi / pourquoi / comment corriger.
```
✗ Failed to reach Anthropic API
  → Invalid API key (401 Unauthorized)
  → Run `aicommit config set anthropic.api_key <key>` to fix
```

**Avertissement**
```
⚠ Diff is large (2,400 lines) — truncating to fit model context
```

**Chargement** (via `indicatif`, pas de print manuel) — spinner cyan + verbe au gérondif :
```
⠋ Analyzing changes...
⠙ Generating commit messages...
```

---

## 6. Flux interactifs (corrections incluses)

### 6.1 Sélection multi-commit — fix du bug Ctrl+C

Règle : **annuler (Esc/Ctrl+C) doit annuler**, pas "tout sélectionner". Séparer explicitement les 3 issues possibles :

```rust
// Note: Group needs new fields (file_count, insertions, deletions) in splitter.rs
pub fn select_commits(groups: &[Group], msgs: &[String]) -> SelectResult {
    let items: Vec<String> = groups.iter().zip(msgs.iter())
        .map(|(g, m)| format!("{}  {m}  ({} files, +{} -{})",
            g.name, g.file_count, g.insertions, g.deletions))
        .collect();

    match dialoguer::MultiSelect::new()
        .with_prompt("Select commits to apply  [space: toggle, a: all, enter: confirm, esc: cancel]")
        .items(&items)
        .interact_opt()          // <- Option, pas de unwrap silencieux
    {
        Ok(Some(sel)) if sel.is_empty() => SelectResult::None,   // explicitement rien coché
        Ok(Some(sel)) => SelectResult::Some(sel),
        Ok(None) | Err(_) => SelectResult::Cancelled,             // esc/ctrl+c
    }
}
```
- `Cancelled` → sortie propre, code retour non-zéro, rien n'est commité, message `✗ Cancelled — no commits created`.
- `None` (0 coché + confirmé volontairement) → même comportement que `Cancelled`, pas "tout".
- Affichage enrichi : nombre de fichiers + stats +/- pour aider la décision, comme dans la boîte dry-run ci-dessus.

### 6.2 Éditeur de message
Garder `dialoguer::Input` mais ajouter un hint sous le prompt :
```
Commit message (edit or press enter to accept):
> feat(auth): add JWT refresh token flow
```

### 6.3 Confirmation avant action destructive
Toute commande qui touche l'historique (`undo`, `--force`) passe par `dialoguer::Confirm` avec `default(false)` explicite — jamais `default(true)` sur une action destructive.

---

## 7. Expérience d'installation & premier lancement

Objectif : de `cargo install aicommit` / `brew install` au premier commit réussi, sans lire de doc.

**Étape 1 — Install (package manager)**
Sortie standard du gestionnaire de paquets, rien à designer côté aicommit ici.

**Étape 2 — Premier `aicommit` lancé dans un repo git**

```
╭──────────────────────────────────────────╮
│  █████╗ ██╗ ██████╗ ██████╗ ███╗   ███╗ │
│  ...                                      │
│  v0.3.1 — AI-powered Git commits          │
╰──────────────────────────────────────────╯

ℹ No provider configured yet. Let's set one up.

? Choose your provider  [use arrows, enter to select]
❯ ● Ollama (local, free, private) — recommended
  ○ Anthropic (Claude)
  ○ OpenAI (GPT)
  ○ Gemini
  ○ Groq / Mistral / Cohere / OpenRouter

  → Local providers need no API key and never send code off your machine.
```

Si Ollama choisi et non détecté :
```
⚠ Ollama not reachable at http://localhost:11434
  → Install: https://ollama.com/download
  → Or press [b] to go back and pick a cloud provider
```

Si provider cloud choisi :
```
? Paste your Anthropic API key: ****************************
⋯ Testing connection...
✓ Connected — using claude-sonnet-4-6
✓ Saved to ~/.config/aicommit/config.toml (permissions 600)
```

**Étape 3 — Premier commit réel** : enchaîne directement sur le flux normal (dry-run → sélection → commit), pas d'écran supplémentaire. Le setup ne doit jamais être un mur séparé de l'usage réel.

**Règle générale install** : le wizard ne se relance jamais automatiquement une fois configuré. `aicommit config wizard` permet de le relancer à la demande.

**Détection CI** : si `CI` ou `GITHUB_ACTIONS` est présent dans l'environnement, le wizard est automatiquement skipé (pas de TTY).

**Sentinel** : fichier `~/.config/aicommit/.config-done` créé après le premier wizard. Sa présence skip le wizard au prochain lancement.

---

## 8. Structure de l'aide (`--help`)

Format court, groupé par intention (pas alphabétique) :

```
aicommit — AI-powered, atomic Git commits

USAGE:
  aicommit [OPTIONS]
  aicommit <COMMAND>

COMMANDS:
  config     Manage provider and behavior settings
  undo       Revert the last aicommit-generated commit
  wizard     Re-run the interactive setup

OPTIONS:
  --dry-run       Preview commits without applying them
  --single        Force a single commit, skip atomic splitting
  --yes           Skip interactive selection, apply all groups
  --provider <p>  AI provider (ollama, openai, anthropic, ...)
  --model <m>     Model override (e.g. gpt-4o, qwen2.5-coder:7b)
  --no-color      Disable colored output
  --quiet         Only print errors
  -h, --help      Print help
  -V, --version   Print version

Run `aicommit` inside a git repo with staged/unstaged changes to get started.
```

---

## 9. Checklist d'implémentation

| Fix | Fichier | Priorité | Effort |
|---|---|---|---|---|
| Ctrl+C ne doit plus committer "tout" | `interactive.rs` | 🔴 Critique | S — 1h |
| Padding boîtes en largeur d'affichage (`unicode-width`) | `display.rs` | 🔴 Critique | S — 1h |
| Version dynamique (`env!("CARGO_PKG_VERSION")`) | `banner.rs` | 🟡 Rapide | S — 30min |
| Respect `NO_COLOR` + flag `--no-color` | `main.rs` | 🟡 Rapide | S — 1h |
| Largeur de boîte adaptative (`terminal_size`) | `display.rs` | 🟢 Moyen | M — 2h |
| Stats fichiers/lignes dans le sélecteur | `interactive.rs`, `splitter.rs` | 🟢 Moyen | M — 2h |
| Wizard de setup au premier lancement | nouveau `src/wizard.rs` | 🟢 Moyen | L — 5h |
| Format d'erreur standardisé (quoi/pourquoi/fix) | `main.rs` + tous les modules | 🟢 Moyen | M — 3h |
| `--help` restructuré par intention | `main.rs` (clap) | 🔵 Nice-to-have | S — 1h |

---

## 10. Ce qui ne change pas

Le style boîtes-Unicode + cyan est déjà une identité visuelle correcte et reconnaissable — on la garde et on la corrige, on ne la réinvente pas.
