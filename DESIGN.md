# git-doctor — Design System (CLI UI/UX)

Ce document définit le système visuel et les flux d'interaction du CLI `doctor` : couleurs, typographie, composants, écrans d'installation, et gestion d'erreurs. Objectif : cohérence entre toutes les commandes, et une première impression (`init` → premier diagnostic) irréprochable.

Basé sur les principes des [Command Line Interface Guidelines](https://clig.dev) : humain d'abord, scriptable ensuite, silencieux par défaut, jamais surprenant.

---

## 1. Principes directeurs

1. **Jamais destructif par surprise** — toute action qui modifie l'historique (apply, undo) doit être confirmée explicitement. Backup automatique avant apply.
2. **Dégradation gracieuse** — fonctionne sans couleur (`NO_COLOR`), sans TTY (CI/scripts), sur terminal étroit.
3. **Un seul système de couleur sémantique** — la couleur porte toujours le même sens, jamais juste décorative.
4. **La densité d'info suit l'intention** — `--dry-run` montre tout, le mode normal montre l'essentiel, `--quiet` ne montre rien sauf erreur.

---

## 2. Palette de couleurs sémantiques

| Rôle | Couleur | Usage `colored` | Exemple |
|---|---|---|---|
| Marque / structure | Cyan | `.cyan()` | Bordures de boîtes, banner, titres de section |
| Succès | Vert | `.green()` | Commit réussi, plan appliqué, check OK |
| Erreur | Rouge | `.red().bold()` | Échec, refus, config invalide, commit bloqué |
| Avertissement / note | Jaune | `.yellow()` | WIP détecté, commit vague, oversized |
| Info / secondaire | Gris (bright black) | `.dimmed()` | Métadonnées, hints, stats de fichiers |
| Sélection active | Magenta | `.magenta().bold()` | Item survolé/coché dans un `MultiSelect` |
| Texte principal | Blanc/défaut | — | Messages de commit, scores, contenu |

**Règle stricte** : jamais deux couleurs différentes pour le même type d'info entre deux commandes. Si `git.rs` affiche une erreur, elle doit avoir le même style qu'une erreur de `analyze.rs`.

**Accessibilité** : respecter `NO_COLOR` (variable d'env standard) et détecter le TTY. Ajouter au démarrage dans `main.rs` :

```rust
if std::env::var("NO_COLOR").is_ok() || !std::io::stdout().is_terminal() {
    colored::control::set_override(false);
}
```

Et exposer un flag explicite `--no-color` dans `clap` qui fait pareil (override manuel prioritaire sur l'auto-détection).

---

## 3. Iconographie

| Symbole | Sens | Fallback ASCII |
|---|---|---|
| `✓` | Succès | `[OK]` |
| `✗` | Erreur / bloqué | `[X]` |
| `⚠` | Avertissement (WIP, vague) | `[!]` |
| `ℹ` | Info | `[i]` |
| `●` / `○` | Sélectionné / non-sélectionné | `[x]` / `[ ]` |
| `→` | Action suivante / hint | `->` |
| `⋯` (via `indicatif`) | Chargement | `...` |

---

## 4. Composant Boîte

Largeur de boîte = `min(terminal_width - 2, 76)`, minimum `40`. Troncature avec `…` si contenu trop long.

```
╭─ doctor analyze ────────────────────────────────────────────╮
│  History Health: C (62/100)                                  │
│    Message quality:     71%  ← 3 commits with vague subjects │
│    Atomicity:           45%  ← 1 commit touched 12 files     │
│    Size discipline:     58%  ← 2 commits > 500 lines         │
│    Convention:          80%  ← 80% follow Conventional       │
╰──────────────────────────────────────────────────────────────╯
```

---

## 5. États de sortie (output states)

Chaque état a un format fixe : **symbole + couleur + message court + détail optionnel en dimmed en dessous.**

**Succès**
```
✓ Plan applied — 3 operations (backup: doctor-backup-20260728)
```

**Erreur** — toujours 3 lignes : quoi / pourquoi / comment corriger.
```
✗ Failed to reach Anthropic API
  → Invalid API key (401 Unauthorized)
  → Run `doctor config --provider anthropic --api-key <key>` to fix
```

**Avertissement**
```
⚠ WIP detected in abc1234: "wip"
```

**Chargement**
```
⋯ Analyzing 20 commits...
⋯ Generating plan...
```

---

## 6. Flux interactifs

### 6.1 Sélection multi-commit — Ctrl+C

Règle : **annuler (Esc/Ctrl+C) doit annuler**, pas "tout sélectionner".

```rust
pub fn select_commits(groups: &[Group], msgs: &[String]) -> SelectResult {
    match dialoguer::MultiSelect::new()
        .with_prompt("Select commits to apply  [space: toggle, a: all, enter: confirm, esc: cancel]")
        .items(&items)
        .interact_opt()
    {
        Ok(Some(sel)) if sel.is_empty() => SelectResult::None,
        Ok(Some(sel)) => SelectResult::Some(sel),
        Ok(None) | Err(_) => SelectResult::Cancelled,
    }
}
```

### 6.2 Confirmation avant action destructive
Toute commande qui touche l'historique (`apply`, `undo`, `--force`) passe par `dialoguer::Confirm` avec `default(false)`.

---

## 7. Expérience premier lancement

**Premier `doctor analyze` dans un repo git :**

```
╭──────────────────────────────────────────╮
│  Git History Doctor — v0.1.0              │
╰──────────────────────────────────────────╯

ℹ Analysing your history for the first time...

✓ Health score: B (78/100) — 10 commits analyzed
  → Run `doctor plan` to generate a cleanup plan
```

---

## 8. Structure de l'aide (`--help`)

```
doctor — Git History Doctor

USAGE:
  doctor [COMMAND]

WORKFLOW:
  analyze    Diagnose commit history quality
  plan       Generate a cleanup plan
  apply      Apply a cleanup plan safely
  check      Pre-push hook or CI gate

PROVIDER:
  commit     Generate atomic commits from staged changes
  config     Manage provider and behavior settings

DISPLAY:
  --no-color    Disable colored output
  -h, --help    Print help
  -V, --version Print version

Run `doctor analyze` in a git repo to get started.
```

---

## 9. Checklist d'implémentation

| Fix | Fichier | Statut |
|---|---|---|
| Ctrl+C ne doit plus committer "tout" | `interactive.rs` | ✅ Fait |
| Respect `NO_COLOR` + flag `--no-color` | `main.rs` | ✅ Fait |
| Padding boîtes + largeur adaptative | `display.rs` | ✅ Fait |
| Version dynamique (`CARGO_PKG_VERSION`) | `banner.rs` | ✅ Fait |
| Champs stats sur `Group` | `splitter.rs` | ✅ Fait |
| Format d'erreur standardisé (quoi/pourquoi/fix) | tous les modules | ✅ Fait |
| `--help` groupé par intention | `main.rs` (clap) | ✅ Fait |

---

## 10. Ce qui ne change pas

Le style boîtes-Unicode + cyan est une identité visuelle correcte et reconnaissable — on la garde et on la corrige, on ne la réinvente pas.
