# git-doctor — Git History Doctor

Diagnostiquer, soigner et maintenir la santé de l'historique Git.

---

## 1. Pourquoi un nouveau produit

L'ancien projet (`aicommit`) était un générateur de messages de commit — un marché saturé
(suenot/aicommit, aicommits, etc.). `git-doctor` change d'horizon :

| Produit | Problème |
|---|---|
| `aicommit` (suenot, 48K+ dl) | Génère des messages de commit |
| `aicommits` (Nutlope, 8.9K stars) | Génère des messages de commit |
| **`aicommit` → `git-doctor`** | ~~Générateur de messages~~ → **Historien clinicien** |

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
├── doctor commit      ← Génère un commit atomique de qualité (hérité)
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

Hook pre-push qui valide la qualité des commits sortants.

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

### 2.6 `doctor commit` — L'héritage

La fonctionnalité originale de génération de messages est conservée
comme sous-commande, avec le splitting atomique — mais recentrée
comme un outil de *prévention* (créer des commits propres) plutôt que de curation.

---

## 3. Réutilisation du code existant

| Module source | Réutilisation | Modification |
|---|---|---|
| `splitter.rs` | Mixed-concern detection pour `analyze` | Ajout scoring, export stats |
| `parser.rs` | Analyser les messages existants dans `analyze` | Ajout scoring de qualité |
| `git.rs` | Lire l'historique, créer branches backup | Ajout `walk_history()`, `create_backup()`, `remote_exists()` |
| `display.rs` | Boîtes de rapport, couleurs sémantiques | Largeur adaptative + unicode-width |
| `config.rs` | Configuration des seuils | Ajout section `[doctor]` |
| `interactive.rs` | Confirmation avant apply | Retour Ctrl+C propre (SelectResult) |
| `prompt.rs` + `llm/` | Génération de messages | Conservé tel quel |
| `banner.rs` | Banner first-run | Version dynamique + chemin `doctor` |

### Nouveaux modules créés

| Module | Description |
|---|---|
| `src/analyze.rs` | Scoring, parsing d'historique, détection de patterns |
| `src/plan.rs` | Génération de plan de correction |
| `src/apply.rs` | Application safe du plan, backup, rollback |
| `src/hook.rs` | Installation de hooks pre-push |
| `src/report.rs` | Génération de rapports (HTML, JSON, Markdown) |

---

## 4. Plan d'implémentation (réalisé)

### Phase 1 — Renommage + `analyze` ✅
1. Renommer le projet `aicommit` → `git-doctor` dans `Cargo.toml`
2. Renommer les imports internes (`aicommit::` → `git_doctor::`)
3. Migration `aicommit config` → `doctor config`
4. Implémenter `doctor analyze` (walk_history, scoring, WIP/vague detection)
5. Rapport texte + JSON + HTML

### Phase 2 — `plan` ✅
1. Génération de plan avec opérations Squash, Reword, Split
2. Format JSON + texte

### Phase 3 — `apply` ✅
1. `git::create_backup_branch()`
2. `git::remote_exists()` — détection pushed commits
3. Dry-run, force flag, rollback backup

### Phase 4 — `check` + hooks ✅
1. `doctor check --pre-push`
2. `doctor init` / `doctor uninstall`

### Phase 5 — `commit` préservé ✅
1. Migration `aicommit` → `doctor commit`
2. Compatibilité totale avec l'ancien flux

---

## 5. Risques et migrations

### Risque : nom du repo GitHub
Le repo s'appelle encore `Mohameddhiab/aicommit`. Décision : renommer en `git-doctor`.

### Risque : confusion avec git-doctor existant
Le nom `git-doctor` est utilisé par des outils de réparation de dépôt corrompu.
La différence est claire : réparation `.git` vs qualité d'historique.

### Migration utilisateurs
Aucun utilisateur existant — le projet n'a pas été publié sur crates.io sous l'ancien nom.

---

## 6. Prochaines étapes

- Renommer le repo GitHub `Mohameddhiab/aicommit` → `Mohameddhiab/git-doctor`
- Mettre à jour les URLs (Cargo.toml, README, CONTRIBUTING)
- Publier v0.1.0 sur crates.io
- Ajouter la détection mixed-concern réelle via `splitter::group_key`
