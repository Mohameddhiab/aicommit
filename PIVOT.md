# git-doctor — Git History Doctor

Diagnostiquer, soigner et maintenir la santé de l'historique Git.

---

## 1. Pourquoi un nouveau produit

`aicommit` était un générateur de messages de commit — un marché saturé
(suenot/aicommit, aicommits, etc.). `git-doctor` change d'horizon :

| Produit | Problème |
|---|---|
| `aicommit` (suenot, 48K+ dl) | Génère des messages de commit |
| `aicommits` (Nutlope, 8.9K stars) | Génère des messages de commit |
| **`aicommit` (toi)** | ~~Générateur de messages~~ → **Historien clinicien** |

**Nouveau positionnement :**

> **`git-doctor`** ausculte ton historique Git, diagnostique les problèmes
> (messages vagues, commits géants, WIP, mixed-concerns), prescrit un plan
> de correction, et l'applique en sécurité.

Aucun outil existant ne couvre le cycle complet :
`diagnostiquer → planifier → appliquer → vérifier`

---

## 2. Architecture produit

```
git-doctor
├── doctor analyze     ← Diagnostique l'historique
├── doctor plan        ← Génère un plan de correction
├── doctor apply       ← Applique le plan (sécurisé)
├── doctor check       ← Pre-push hook / CI gate
├── doctor report      ← Rapport HTML/JSON/Markdown
├── doctor commit      ← Génère un commit atomique de qualité
└── doctor init        ← Installe les hooks + config
```

### 2.1 `doctor analyze` — Diagnostic

Scanne l'historique (HEAD~N ou une plage) et produit un score multidimensionnel.

```
$ doctor analyze

History Health: C (62/100)
  Message quality:     71%  ← 3 commits with vague subjects
  Atomicity:           45%  ← 1 commit touched 12 unrelated files
  Size discipline:     58%  ← 2 commits > 500 lines
  Convention:          80%  ← 80% follow Conventional Commits

Top issues (by severity):
  🔴 Mixed concern in abc1234 (12 files across 4 domains)
  🟡 Vague message in def5678 ("fix stuff")
  🟡 Oversized commit in ghi9012 (+650 lines)
  🟢 3 commits with no body

Detailed: doctor analyze --verbose
Output as JSON: doctor analyze --format json
Output as HTML: doctor analyze --format html --open
```

### 2.2 `doctor plan` — Prescription

Analyse les problèmes et génère un plan de correction actionnable.

```
$ doctor plan

Proposed cleanup plan (3 operations):
  1. SQUASH  def5678 ← fff0000  "Fix review comments"
     → Merge 3 fix-typo commits into parent
  2. REWORD  abc1234  "feat(auth): implement JWT refresh flow"
     → Current: "update auth stuff"
  3. SPLIT   ghi9012  into 3 logical commits
     → Group 1: db/migration/  (7 files)
     → Group 2: api/routes/   (4 files)
     → Group 3: frontend/     (3 files)

Preview with --verbose, save with --output plan.json
```

### 2.3 `doctor apply` — Traitement

Applique le plan avec filets de sécurité.

```
$ doctor apply plan.json

Safety checks:
  ✓ No commits pushed to origin (local branch only)
  ✓ Dry-run: simulating operations...

  Would create backup branch: doctor-backup-20260728
  Would rewrite 12 commits across 3 operations

Run without --dry-run to apply.
Use --force to bypass safety checks.
```

### 2.4 `doctor check` — Prévention

Hook pre-commit ou pre-push qui valide la qualité.

```
$ doctor check --pre-push

Checking 3 outgoing commits...
  ✓ feat(auth): add JWT refresh          (6 files, +120 -15)
  ✗ fix: wip                             ← vague, no scope
  ✗ db migration + api changes           ← mixed concern

❌ 2 commits blocked. Use --force to override.
```

### 2.5 Score multidimensionnel

| Dimension | Métrique | Seuil par défaut |
|---|---|---|
| **Message quality** | Sujet vague (< 3 mots), absence de body, lowercase | Alerte si < 3 mots |
| **Atomicity** | Fichiers dans plusieurs domaines dans 1 commit | Alerte si > 1 domain |
| **Size discipline** | Lignes modifiées, fichiers touchés | Alerte si > 500 lines ou > 10 files |
| **Convention** | Conventional Commits, type valide, scope optionnel | Alerte si non CC |
| **WIP detection** | Patterns (wip, fix stuff, temp, asdf, etc.) | Alerte sur match |

Score global : moyenne pondérée (`message × 0.3 + atomicity × 0.3 + size × 0.2 + convention × 0.2`).

### 2.6 `doctor commit` — L'héritage d'aicommit

La fonctionnalité originale de génération de messages est conservée
comme sous-commande, avec le splitting atomique — mais recentrée
comme un outil de *prévention* plutôt que de *génération*.

---

## 3. Réutilisation du code aicommit

| Module aicommit | Réutilisation | Modification |
|---|---|---|
| `splitter.rs` | Mixed-concern detection pour `analyze` | Ajouter scoring, export des stats par fichier |
| `parser.rs` | Analyser les messages existants dans `analyze` | Ajouter scoring de qualité |
| `git.rs` | Lire l'historique, créer branches backup, rebase | Ajouter `walk_history()`, `create_backup()`, `rebase_plan()` |
| `display.rs` | Boîtes de rapport, couleurs sémantiques | Ajouter format HTML/JSON |
| `config.rs` | Configuration des seuils, hooks | Ajouter section `[quality]` |
| `interactive.rs` | Confirmation avant apply, sélection | Ajouter plan review |
| `prompt.rs` + `llm/` | Optionnel : rewrite AI des messages faibles | Garder tel quel |
| `banner.rs` | Banner first-run pour `doctor init` | Adapter le texte |

### Nouveaux modules à créer

| Module | Taille | Description |
|---|---|---|
| `src/analyze.rs` | M | Scoring, parsing d'historique, détection de patterns |
| `src/plan.rs` | M | Génération de plan de correction |
| `src/apply.rs` | L | Application safe du plan, backup, rollback |
| `src/hook.rs` | S | Installation de hooks pre-push |
| `src/report.rs` | M | Génération de rapports (HTML, JSON, Markdown) |

---

## 4. Plan d'implémentation

### Phase 1 — Renommage + `analyze` (1-2 jours)

1. Renommer le projet `aicommit` → `git-doctor` dans `Cargo.toml`
2. Renommer les dossiers et imports internes
3. Supprimer `aicommit config` au profit de `doctor config`
4. Implémenter `doctor analyze` :
   - `git::walk_history()` pour lire N commits
   - `analyze::score_message()` → scoring qualité message
   - `analyze::detect_mixed_concern()` → reprise de `splitter.rs`
   - `analyze::detect_oversized()` → taille lignes/fichiers
   - `analyze::detect_wip_patterns()` → regex matching
5. Afficher le rapport dans le terminal (boîtes)

### Phase 2 — `plan` (1-2 jours)

1. Implémenter `doctor plan` :
   - Grouper les correctifs par type
   - Suggérer squash groups (reprise de l'algo `splitter.rs`)
   - Suggérer reword messages
   - Suggérer split commits
2. Format de sortie JSON + terminal

### Phase 3 — `apply` (2-3 jours)

1. `git::create_backup_branch()`
2. `git::rebase_plan()` avec script rebase
3. Safety : détection pushed commits, dry-run, rollback
4. `doctor apply <plan.json>`

### Phase 4 — `check` + `report` (1 jour)

1. `doctor check --pre-push` → hook git
2. `doctor init` → installe le hook
3. `doctor report --format html`

### Phase 5 — `commit` préservé (1 jour)

1. Migrer `aicommit` → `doctor commit`
2. Tester la compatibilité

---

## 5. Risques et migrations

### Risque : perte du nom aicommit sur GitHub
Ton repo s'appelle déjà `Mohameddhiab/aicommit`. Deux options :
- Renommer le repo (`git-doctor`)
- Garder le repo et changer le binaire seulement

### Risque : confusion avec git-doctor existant
Le nom `git-doctor` est utilisé par des outils de réparation de dépôt
corrompu, pas de qualité d'historique. La différence est claire :
- `git-doctor` existant → répare un `.git` corrompu
- `doctor` → soigne la qualité de l'historique

### Migration utilisateurs
Les utilisateurs existants d'`aicommit` (0 pour l'instant) ne seront
pas impactés — le projet n'a pas été publié sur crates.io.

---

## 6. Questions ouvertes

- Faut-il garder la compatibilité avec `.aicommit.toml` ou migrer vers
  `doctor.toml` / `.git-doctor.toml` ?
- L'AI rewrite des messages faibles doit-il être optionnel ou intégré ?
- Support des remotes (analyse PRs, stats équipe) en v2 ?
