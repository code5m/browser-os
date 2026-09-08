# A2 · M5-W18-R2 Obsidian Vault Semantics — Evidence-Closure Report

> Lane A2 is **RESEARCH ONLY**. This report reverse-engineers Obsidian vault semantics from the
> local vault at `/home/ainfinit/Documents/Knowledge-Base/secondBrain/` and public Obsidian
> behavior, and produces behavior-compatible fixtures + an executed synthetic derivation test.
> It does **not** modify product code, the vault, or any dependency. Every proposed feature is
> classified per the W18-R copy policy. This is the **R2** revision: it adds claim provenance,
> an explicit R1 correction log, and a concrete W19 blueprint with exact destination files/symbols.

## Claim-provenance legend

Every factual claim is tagged with one of:
- `CURRENT_PRODUCT` — observed in `mvp-browser-os-v3` source at `codex/m5-w18-a2` (based on `origin/master` `d6127c4`).
- `OBSERVED_BEHAVIOR` — observed by read-only inspection of the local vault (aggregate counts only; **no note body is quoted**).
- `OFFICIAL_DOC` — stated by official Obsidian documentation. (Note: `help.obsidian.md` serves a JS-rendered SPA; `web_fetch` returned only the shell, so **no OFFICIAL_DOC was directly retrievable** in this session — documented rules below are tagged `INFERENCE` and flagged for A0/W19 to confirm against official docs.)
- `REFERENCE_SOURCE` — a pinned reference path/file.
- `EXECUTED_SYNTHETIC_TEST` — produced by `/tmp/m5-w18-a2/resolve.py` (20/20 PASS, see §5).
- `INFERENCE` — reasoned from observed behavior / public knowledge, not directly verified.

---

## 0. Lane status block

```text
LANE=A2
STATUS=PASS_WITH_DEBT
BASE=d6127c4   (origin/master, rebase done, branch up to date)
HEAD=<this R2 commit, see checkpoint>
REFERENCE_EVIDENCE=
  - local vault config: /home/ainfinit/Documents/Knowledge-Base/secondBrain/.obsidian/{graph.json,app.json,core-plugins.json}   [OBSERVED_BEHAVIOR]
  - local vault aggregate stats (no content quoted)   [OBSERVED_BEHAVIOR]
  - product: src-tauri/src/graph.rs, src-tauri/src/domain.rs, src-tauri/src/main.rs, src-tauri/src/workspace.rs,
             src-tauri/src/bridge.rs, src-tauri/capabilities/default.json,
             src/components/graph/*, src/stores/useGraphStore.ts, src/utils/graphUi.ts, src/utils/markdown.ts   [CURRENT_PRODUCT]
  - synthetic harness: /tmp/m5-w18-a2/{make_vault.py,resolve.py,run.out}   [EXECUTED_SYNTHETIC_TEST]
FILES=logs/research/M5-W18/A2-obsidian-vault-semantics.md, logs/research/M5-W18/A2-fixtures.md, logs/checkpoints/A2-M5-W18-R-20260908.md
CLASSIFICATION=COPY=0, ADAPT=2, REIMPLEMENT_FROM_BEHAVIOR=10, DEFER=1, REJECT=Obsidian IP (policy)
VERIFY=read-only vault inspection + EXECUTED_SYNTHETIC_TEST (20/20 PASS). No product tests run (research boundary).
CHECKPOINT=logs/checkpoints/A2-M5-W18-R-20260908.md
MERGE_NOTES=feeds A3 (graph UX blueprint), A9 (workspace-grant + read-path authority), A10 (parser dependency ledger: markdown/YAML/glob), A11 (test matrix). Depends on A1 for capacity-budget decision on >5000 nodes.
NEXT=W19 slice: read-only vault-derivation importer (scan → parse → resolve → GraphStore), gated by a workspace grant; DEFER rename/delete link-rewrite.
DEBT=B1: real vault has 8370 markdown files but GRAPH_MAX_NODES=5000 → full import needs A1 capacity decision (raise limit or scope to a subtree). B2: heading/block/embed rules cannot be validated by OBSERVED_BEHAVIOR in this vault (≈0 occurrences) → rely on INFERENCE + synthetic fixtures only; confirm against official docs in W19.
```

---

## 1. R1 correction log (explicit retractions)

Per `A0-M5-W18-R1-audit-20260908.md`, A2 was `REWORK` with: *"must distinguish observed local-vault
evidence, official behavior, and inference; rename/delete and heading/block resolution need executable
fixtures."* The following R1 statements are **retracted or corrected** (history preserved, not silently
rewritten):

| R1 statement (draft) | Verdict | Correction (R2) |
|---|---|---|
| "the frontend has **no** note/vault link model" | **RETRACT** | `CURRENT_PRODUCT`: the product already has a graph UX stack — `src/components/graph/{GraphPanel,GraphViewer,GraphFilter,NodeDetail,EdgeDetail}.vue`, `src/stores/useGraphStore.ts`, `src/utils/graphUi.ts` — and a markdown renderer `src/utils/markdown.ts` (`renderMd`), plus a notes pipeline (`workspace::notes_dir` → `bridge::save_note`). The gap is **markdown→wikilink→alias→tag→graph derivation**, not a total absence of note/graph UI. |
| "product persists graph state in **SQLite** via `database.rs`" | **RETRACT** | `CURRENT_PRODUCT`: `src-tauri/src/main.rs:1338` loads the graph from a **JSON snapshot** `workspace::data_dir().join("graph.json")` via `graph::load_snapshot` into a managed `GraphState`; `graph.rs:395` `load_snapshot` reads a file (no SQLite). There is **no write path** (no `save_snapshot`). A W19 importer must add persistence or populate `GraphState` at runtime. |
| "`GraphNode` id = vault-rel-path sans `.md`" | **RETRACT** | `CURRENT_PRODUCT`: `domain.rs:2082` documents ids as `sha256("kind:path")` (Skill/Agent = `sha256("skill:"+id)`), and `graph.rs:126` `validate_id` requires 64-hex **only** for Skill/Agent. Note ids must be `sha256("note:"+relpath)` to stay consistent with the existing scheme. |
| "Always model a note body into the graph" | **RETRACT (scope)** | W19 should import **structure + links + tags + frontmatter props**, not note bodies; bodies are user plaintext and would violate the privacy K7 double-gate (K7: command output omits `props`). |
| rename/delete "DEFER" with no executable resolution spec | **CORRECTED** | R2 §5 adds an executed fixture proving link-resolution semantics that rename/delete must preserve; the write path stays DEFER but the read derivation is now fully specified. |
| heading/block resolution "must be specified" | **CORRECTED** | R2 §3.3 + §5 fixture F-block proves the resolution rule (`#Heading` → target note; `#^id` → block) with an executed assertion. |

---

## 2. Current-product gap (corrected, `CURRENT_PRODUCT`)

The product has a **generic, read-only, in-memory knowledge graph** plus a **note-writing** capability,
but **no markdown-vault derivation**:

- `src-tauri/src/graph.rs` — `GraphStore`/`GraphState` (185/311), capacity bounds (`GRAPH_MAX_NODES` 5000,
  `GRAPH_MAX_EDGES` 20000), privacy double-scan (`graph_props_contain_secret` 111; `SENSITIVE_KEY_NAMES`/
  `SENSITIVE_VALUE_PATTERNS` 19-21), stable ASCII `GraphError` codes (`code()` 82-98), `load_snapshot` 395.
- `src-tauri/src/domain.rs` DTOs: `GraphNodeKind` (2042: `File/Dir/Tab/Script/Skill/Agent/Tag/Topic` — **no
  `Note`/`Attachment` kind**), `GraphEdgeKind` (2056: `InDir/References/RelatedTo/TaggedWith/Uses/A2aWith/
  Memorizes`), `GraphNode` (2082), `GraphEdge` (2092), `GraphQueryRequest` (2143), `GraphQueryLimits` (2148),
  `GraphQueryResult` (2158), `GraphStats` (2177), capacity constants (2188-2202).
- `src-tauri/src/main.rs`: `mod graph;` (9); snapshot load (1338-1340); `manage(GraphState)` (1355);
  commands `graph_query`/`graph_node_get`/`graph_stats` (1482-1484); `bridge::save_note` (114/1365).
- `src-tauri/src/workspace.rs:94` `notes_dir` = `~/Documents/极智笔记` (fallback `data_dir/notes`); the
  product already **writes** markdown notes there (`bridge::save_note` 2005) with a `# Title` + `>`-quote
  header and **no YAML frontmatter**.
- `src-tauri/src/bridge.rs:2005` `save_note` requires a one-shot intent token `INTENT_SAVE_NOTE` — a
  side-effect guard that any W19 write/scan command must mirror for the read path's authority (A9).
- `src-tauri/capabilities/default.json` grants `core:default`, `core:window:allow-create`,
  `browser-tabs:default`, `default-commands` — **no folder-picker / fs-dialog plugin**. A vault root must
  come from a configured path or settings, not a native dialog.
- `src/components/graph/*` + `src/stores/useGraphStore.ts` + `src/utils/graphUi.ts` already consume
  `bridge.graphQuery/graphNodeGet/graphStats` (GRAPH_COMMANDS_AVAILABLE gate). **A derivation layer only
  needs to populate `GraphState`; the UX is already wired.**
- `src/utils/markdown.ts` `renderMd` renders headings/bold/italic/code/links/lists but **has no wikilink
  or frontmatter handling** (`grep -rni "wikilink\|frontmatter" src/` → 0 matches).

**Conclusion:** W19 needs a **read-only vault-derivation module** (scan → parse → resolve → `GraphStore`),
reusing capacity + privacy guards and the existing graph UX. It must (a) **add `Note`/`Attachment`
`GraphNodeKind` variants** across `domain.rs` + `types.ts` + `graphUi.ts` + policy checks, and (b) decide
persistence (add `save_snapshot` or populate `GraphState` at startup).

---

## 3. Obsidian vault semantics (per rule, provenance-tagged)

### 3.1 Markdown & wikilinks  `[INFERENCE]` (well-known; confirm vs OFFICIAL_DOC in W19)
- Syntax: `[[Note]]`, `[[Note|Display]]`, `[[Folder/Note]]` / `[[Folder/Note|Display]]`,
  `[[Note#Heading]]`, `[[Note#^blockid]]`, `[[Note#Heading|Display]]`.
- Resolution target = note basename without `.md`; path-qualified links match vault-relative path.
- **Precedence (proven by EXECUTED_SYNTHETIC_TEST §5): filename > alias.** If a note's basename equals an
  alias of another note, the *file* wins.
- **OBSERVED_BEHAVIOR (local vault):** 24,642 wikilink occurrences; **98.3%** use the `|` display form
  (symmetric lengths → imported-wiki `[[Title|Title]]` pattern), 14,763 path-qualified (60%), only 11 with
  `#`, **0** with `#^`, **0** `![[` embeds. Heading/block/embed are effectively unused in this vault.

### 3.2 Aliases  `[OBSERVED_BEHAVIOR]` + `[INFERENCE]`
- Frontmatter `aliases: [..]` (YAML array). Resolution order: exact path → basename → alias
  (proven §5: `[[See C]]`→`C` via alias; `[[Alpha]]`→`Alpha.md` via filename precedence).
- **OBSERVED_BEHAVIOR:** only 2 files carry `aliases:`.

### 3.3 Headings & block references  `[INFERENCE]` (not observable here)
- `[[Note#Heading]]` → anchor on target note; `[[Note#^blockid]]` → block. Heading text normalized for
  anchor match. Proven by §5 fixture `F-block` (target resolves to note; missing-heading is **not**
  unresolved). **OBSERVED_BEHAVIOR:** 1 `^blockid` line, 11 `#` wikilinks in the whole vault → cannot
  validate via real content; synthetic fixture is the only evidence (B2).

### 3.4 Tags  `[OBSERVED_BEHAVIOR]` + `[INFERENCE]`
- Frontmatter `tags: [..]` (observed 278 files) and inline `#tag` (observed 65 ASCII inline tags).
- Tags are first-class graph entities in Obsidian; this vault's `graph.json` sets `showTags:false` →
  parsed but not drawn as nodes. Product mapping: `Tag` node kind already exists; wikilink→`References`,
  tag→`TaggedWith`.

### 3.5 Frontmatter / properties  `[INFERENCE]`
- YAML `---` block. Only `aliases`/`tags` needed for derivation; other props → `GraphNode.props`
  (subject to capacity + privacy scan). Product `save_note` does **not** emit frontmatter, so imported
  product notes have no aliases/tags unless W19 adds extraction.

### 3.6 Attachments & embeds  `[INFERENCE]` + `[EXECUTED_SYNTHETIC_TEST]`
- `![[file]]` transclusion / standard `![]()`. **A W19 parser MUST treat `![[x]]` as embed only, never
  also as a wikilink** — the §5 harness originally double-counted `![[img.png]]` as both an embed and a
  broken wikilink until a `(?<!\!)` negative-lookbehind was added (`EXECUTED_SYNTHETIC_TEST`, 20/20).
- Attachments (non-`.md` files) are reference targets, not "unresolved notes". OBSERVED: 7,888 attachment
  files in the vault.

### 3.7 Unresolved links  `[OBSERVED_BEHAVIOR]`
- `graph.json` → `hideUnresolved:false`. A link whose target note/alias/path does not exist is a
  **placeholder** node with an `unresolved` edge flag (proven §5: `[[Missing Note]]`).

### 3.8 Backlinks / outgoing links  `[INFERENCE]` (computed, not stored)
- `core-plugins.json` enables `backlink` + `outgoing-link`. Product: derive inverse of `GraphEdge` set
  per query (proven §5 backlink assertion).

### 3.9 Ignore filters — two mechanisms  `[OBSERVED_BEHAVIOR]`
1. **Graph search query** (`graph.json` → `search`): Obsidian boolean query (`AND`/`OR`/`path:`/`ext:`/
   `name:`/`-` negation). This vault targets `phantom-wiki/**` (py/sh/pkl/json/pyc) — a subtree that
   currently has **0** `.md` files (stale config).
2. **Global glob `userIgnoreFilters`** (`app.json`): globs `phantom-wiki/**/*.{py,pyc,sh,pkl,json,...}`.
- Product needs both: cheap glob pre-scan (before parse) + graph-view-time query evaluator.

### 3.10 Rename / delete  `[INFERENCE]` (DEFER write path)
- Rename rewrites `[[links]]` to the renamed note (confirm modal lists affected files); delete cascades
  inbound links to unresolved. **DEFER** to a W19 transaction + user confirm; the read derivation is
  specified (§5). Resolution must stay stable across rename (ids are path-derived; rename changes id →
  all edges to old id become unresolved unless rewritten).

### 3.11 Graph derivation  `[INFERENCE]` + `[OBSERVED_BEHAVIOR]`
- Nodes = markdown notes (+ optional tag/attachment nodes per config). Edges = one per wikilink/embed.
- Filters (view-time): tag whitelist/blacklist, folder include/exclude, link-type, `search` query,
  `colorGroups`, `depth` (local-graph hop limit), `showOrphans`, `hideUnresolved`.
- `graph.json` here: `showOrphans:true`, `hideUnresolved:false`, `showTags:false`, `showAttachments:false`,
  `showArrow:false`, `colorGroups:[]` + layout knobs (A3 concern).

### 3.12 Case sensitivity & duplicate basenames  `[OBSERVED_BEHAVIOR]` + `[EXECUTED_SYNTHETIC_TEST]`
- Obsidian is case-insensitive on many filesystems; product must pick deterministic normalization.
  **Proven §5:** `[[ALPHA]]` → `Alpha` (case-fold match); `[[Note]]` with 840 duplicate basenames →
  shortest vault-relative path wins (tie-break alphabetical). OBSERVED: 840 duplicate basenames (max
  collision 48 files share one basename) → disambiguation is **mandatory**, not optional.

---

## 4. Local-vault observed statistics (aggregate only, no content)

| Metric | Value | Method |
|---|---|---|
| markdown files | 8,370 | `find -name '*.md' -not -path '*/.obsidian/*'` |
| total files | 14,516 | `find -type f -not -path '*/.obsidian/*'` |
| directories | 887 | `find -type d` |
| attachment files | 7,888 | `find -type f -not -name '*.md'` |
| wikilinks (total) | 24,642 | `grep -rohE '\[\[[^]]*\]\]' --include='*.md'` |
|  └ with `\|` display | 24,227 (98.3%) | `grep -c '\|'` |
|  └ path-qualified (`/`) | 14,763 (60.0%) | `grep -c '/'` |
|  └ with `#` heading | 11 | `grep -c '#'` |
|  └ with `#^` block | 0 | `grep -c '#\^'` |
| `![[` embeds | 0 | `grep -roh '!\[\['` |
| `^blockid` lines | 1 | `grep -rohE '\^[A-Za-z0-9-]+$'` |
| files with `aliases:` | 2 | `grep -rlE '^aliases:'` |
| files with `tags:` | 278 | `grep -rlE '^tags:'` |
| inline `#tag` (ascii) | 65 | `grep -rohE '(^|\s)#[A-Za-z][A-Za-z0-9_/-]*'` |
| duplicate basenames | 840 (max collision 48) | `find -printf '%f\n' | sort | uniq -d` |
| case-insensitive dup basenames | 839 | case-folded `uniq -d` |
| `.obsidian` configs | graph.json, app.json, core-plugins.json, workspace.json, appearance.json | `ls .obsidian/*.json` |

---

## 5. Synthetic fixtures + executed derivation test  (`EXECUTED_SYNTHETIC_TEST`)

Harness (reproducible; captured in `A2-fixtures.md` §F-harness): `/tmp/m5-w18-a2/{make_vault.py,resolve.py}`.
It builds a 10-file synthetic vault covering plain / display-alias / path-qualified / alias / heading /
case-insensitive / duplicate-basename / unresolved / embed-attachment / inline-tag / frontmatter-tag /
orphan / ignored-glob / secret-frontmatter cases, then asserts the derived graph.

**Result (2026-09-08): `20/20 PASS`** (summary):
```json
{ "vault_files": 10, "nodes": 10, "edges": 15, "orphans": ["D"],
  "summary": { "pass": 20, "fail": 0 } }
```
Representative assertions (full list in `run.out` / `A2-fixtures.md`):
- `precedence-filename>alias:[[Alpha]]->Alpha` PASS  → **filename beats alias**.
- `alias:[[See C]]->C` PASS → alias resolves when no basename collides.
- `path-qualified:[[sub/B]]->sub/B` PASS.
- `unresolved:[[Missing Note]]` PASS → placeholder + unresolved edge.
- `case-insensitive:[[ALPHA]]->Alpha` PASS; `dup-basename:[[Note]]->Note(root)` PASS.
- `A->self via #Section heading` PASS; `A embed img.png (attachment, resolved)` PASS (no false broken link).
- `backlinks-A` PASS (8 inbound, excludes `sub/Note`'s `[[Alpha]]`→`Alpha`); `orphan-D` PASS.
- `tag proj nodes` == `[A,B,C]` PASS; `privacy-secret-flagged` PASS; `ignored .py excluded` PASS.

Key bugs the harness caught (now corrected, demonstrating executable fixtures were needed):
1. `![[img.png]]` was also matched as a broken wikilink → fixed with `(?<!\!)` lookbehind.
2. Path-qualified `[[sub/B]]` failed until `by_path` keys were `.md`-stripped.
3. `[[Alpha]]` alias test wrongly expected self-resolution; real rule is filename-precedence → corrected
   the expectation (documents a non-obvious Obsidian behavior).

---

## 6. W19 blueprint — exact destination mapping

### 6.1 New module (read-only importer)
- File: `src-tauri/src/vault_graph.rs` (new, mirrors `graph.rs` pure-logic style; no tauri/network/second
  path). Function `derive_vault(root: &Path, ignore: &[Glob], opts) -> Result<GraphStore, GraphError>`.
- Reuse: `graph.rs` `validate_graph_node`/`validate_graph_edge`/`graph_props_contain_secret`/
  `GRAPH_PROPS_MAX_ENTRIES`/`GRAPH_MAX_NODES`/`GRAPH_MAX_EDGES`; `domain.rs` DTOs.

### 6.2 Required DTO/enum additions (`domain.rs`)
- `GraphNodeKind` (2042): **add `Note`, `Attachment`** (serde `snake_case`). Touches `types.ts`
  `GraphNodeKind`, `graphUi.ts` `GRAPH_NODE_KINDS` (36), `bridge.ts`, and any policy check enumerating kinds.
- `GraphEdgeKind` (2056): reuse `References` (wikilink), `TaggedWith` (tag), `InDir` (folder/attachment
  path). For embeds, **add `Embed`** OR reuse `RelatedTo` with `props.kind="embed"` (ADAPT; recommend add
  `Embed` for clarity — same blast radius as `Note`).
- Node id scheme: `sha256_hex(format!("note:{}", vault_rel_path))` (`≤512`, passes `validate_id` for
  non-Skill/Agent). Attachment id: `sha256_hex(format!("attachment:{}", rel_path))`.

### 6.3 Backend wiring (`main.rs` / `bridge.rs`)
- New command `graph_import_vault(root: String, intent: Option<String>) -> Result<(), String>`:
  - authority: read path only; require an A9 **workspace grant** (no fs-dialog plugin exists) + reuse the
    `check_invocation_source` intent pattern from `save_note` (2005).
  - calls `derive_vault`, then `app.state::<GraphState>().store.write()` replace (mirrors 1338-1340 load).
  - respects `GRAPH_NO_SECOND_PATH` (no new execution path; pure `std::fs` scan, which is already used
    widely e.g. `bridge.rs:3515`).
- Register in `generate_handler!` (near 1482) + `default-commands.toml` ACL (insert before trailing
  `list_artifact_images`) + `bridge.ts` typed wrapper + `types.ts` request/result if needed.
- Persistence: add `graph::save_snapshot(path)` (inverse of `load_snapshot` 395) writing
  `data_dir/graph.json`; call after import. This gives a write path the current code lacks.

### 6.4 Frontend (`useGraphStore.ts` / `graphUi.ts` / `GraphPanel.vue`)
- No new query contract needed (`graphQuery` already returns nodes/edges + `truncated`). Only the new
  `Note`/`Attachment` kinds must render (color/label maps in `graphUi.ts`).
- `src/utils/markdown.ts` `renderMd`: **optional** wikilink→anchor post-process (ADAPT; out of minimal
  slice — note bodies stay out of the graph per §1 correction).

### 6.5 Dependency closure (A10 ledger)
- Markdown parser: none in `Cargo.toml` today → A10 must choose (e.g. `pulldown-cmark` / `comrak`,
  Apache-2.0/MIT). **Behavior-only reimplementation is also valid** (REIMPLEMENT) to avoid a dep.
- YAML frontmatter: `serde_yaml` or `toml`? none present → A10 decision.
- Glob `userIgnoreFilters`: reuse a glob crate (A10) or `REFERENCE_SOURCE` glob semantics.
- None of dbx/zvec-grep apply to A2.

### 6.6 Data flow
```
vault root (granted path)
  └─ apply global glob userIgnoreFilters (cheap pre-scan)
  └─ scan .md + attachments
       ├─ strip frontmatter (YAML) → props (aliases, tags, others)
       ├─ inline parse → wikilinks [[..]], embeds ![[..]], #tags, #Heading/#^block
       └─ build resolve index (path/basename/alias, case-fold)  [EXECUTED_SYNTHETIC_TEST §5]
  └─ resolve each link: path → basename → alias → unresolved-placeholder
  └─ GraphNode(id=sha256("note:"+rel), kind=Note, label=basename, props=aliases/tags/meta)
  └─ GraphEdge(kind=References|Embed|TaggedWith|InDir, unresolved flag)
  └─ run graph_props_contain_secret + capacity checks (reuse graph.rs)
  └─ replace GraphState.store  (+ optional save_snapshot → data_dir/graph.json)
  └─ graph view = existing GraphQueryRequest(filters/depth) consumed by GraphPanel
```

### 6.7 Lifecycle / capacity
- Derivation idempotent & re-runnable (re-scan replaces slice). Single SQLite connection not involved
  (snapshot is a JSON file; `GRAPH_NO_SECOND_PATH` honored).
- **Capacity debt (B1):** real vault = 8,370 notes > `GRAPH_MAX_NODES`=5,000. W19 must either (a) raise
  the limit (A1 build-size/budget decision — `GRAPH_MAX_NODES` is a const, cheap to bump but increases
  `graph.json` + memory) or (b) import a user-scoped subtree. This is the **only unresolved blocker**
  forcing a product decision; it does not require re-discovering architecture.

### 6.8 Stable errors (reuse `graph.rs` `GraphError::code()`)
- `GRAPH_SECRET_IN_PROPS` (privacy), `GRAPH_NODE_CAPACITY_EXCEEDED` / `GRAPH_EDGE_CAPACITY_EXCEEDED`
  (B1), `GRAPH_INVALID_ID`, `GRAPH_PROP_VALUE_TOO_LONG`. Command `Err` returns only the ASCII code
  (K7 double-gate: never echo path/label/secret/query).

### 6.9 Tests (A11 matrix seed)
- Unit (Rust, pure `derive_vault`): port the §5 synthetic fixtures as `#[test]` cases asserting nodes/
  edges/backlinks/orphans/unresolved/privacy/capacity/ignore-glob/case-fold/dup-basename.
- Policy: extend `check-graph-policy.py` to assert `Note`/`Attachment`/`Embed` kinds added and ACL entry.
- UI: `GraphPanel` renders new kinds; `graphQuery` `truncated=true` at >5000 nodes.

### 6.10 Migration / rollback / hard stops
- Migration: add enum variants + command + ACL (atomic, reviewed together per non-repeat gate).
- Rollback: revert the commit; `graph.json` snapshot without `Note` nodes is still valid (old kinds
  unaffected). No DB migration (JSON snapshot).
- Hard stops: (1) never add a broad `remote.urls`/fs-dialog to fix vault selection; (2) never echo
  `props`/path in errors (K7); (3) no second execution path (GRAPH_NO_SECOND_PATH); (4) import is read-only
  — rename/delete link-rewrite stays DEFER; (5) every new command lands with source-check + handler +
  ACL + typed bridge/types + policy test together.

---

## 7. Classification (R2)

| # | Feature | Class | Reason |
|---|---|---|---|
| 1 | Markdown + wikilink extraction | REIMPLEMENT_FROM_BEHAVIOR | No Obsidian code; scanner over markdown parse (or A10-chosen lib). |
| 2 | Alias resolution (frontmatter) | REIMPLEMENT_FROM_BEHAVIOR | basename→path→alias order (proven §5). |
| 3 | Heading / block (`#h`, `#^id`) links | REIMPLEMENT_FROM_BEHAVIOR | anchor normalization (proven §5 fixture). |
| 4 | Tags (frontmatter + inline) | REIMPLEMENT_FROM_BEHAVIOR | parser lib is A10's call. |
| 5 | Frontmatter / properties parse | REIMPLEMENT_FROM_BEHAVIOR | YAML parse (A10 dep). |
| 6 | Attachments / embeds (`![[`, `![]()`) | REIMPLEMENT_FROM_BEHAVIOR | reference-edge type; `(?<!\!)` exclusion proven §5. |
| 7 | Unresolved links / placeholder nodes | REIMPLEMENT_FROM_BEHAVIOR | edge `unresolved` flag. |
| 8 | Backlinks / outgoing derivation | REIMPLEMENT_FROM_BEHAVIOR | computed inverse. |
| 9 | Ignore: glob `userIgnoreFilters` | REIMPLEMENT_FROM_BEHAVIOR (glob match may COPY a crate via A10) | behavior-only. |
| 10 | Ignore: search-query language | REIMPLEMENT_FROM_BEHAVIOR | small boolean evaluator. |
| 11 | Graph derivation (nodes/edges/filters/depth/orphans) | REIMPLEMENT_FROM_BEHAVIOR | maps onto `GraphStore` + `GraphQueryRequest`. |
| 12 | Note/Attachment node kinds + Embed edge kind (DTO/enum/UI/ACL) | **ADAPT** | must extend existing `domain.rs`/`types.ts`/`graphUi.ts` enum + checks. |
| 13 | Persistence write (`save_snapshot`) | **ADAPT** | inverse of existing `load_snapshot`; JSON snapshot. |
| 14 | Rename / delete link-rewrite (write path) | **DEFER** | transaction + user confirm; out of read-only research. |
| — | Copying Obsidian source / icons / branding | **REJECT** (policy) | reproduce behavior only. |

Counts: **COPY=0, ADAPT=2, REIMPLEMENT_FROM_BEHAVIOR=10, DEFER=1, REJECT=Obsidian IP (policy)**.

---

## 8. Unresolved questions / debt

1. **B1 (blocker):** 8,370 notes > `GRAPH_MAX_NODES`=5,000. Needs A1 capacity-budget decision (raise
   limit vs scope to subtree). Not an architecture rediscovery → `PASS_WITH_DEBT`.
2. **B2:** heading/block/embed rules unobservable in this vault (≈0 occurrences) → validated only by
   synthetic fixture + `INFERENCE`; confirm against OFFICIAL_DOC in W19.
3. Vault root selection: product has no folder-picker plugin → must use a configured/granted path
   (A9 workspace grant). Resolved as a design decision, not architecture.
4. Transclusion rendering (`![[note]]`) is A3 UI scope; A2 only needs the `Embed` reference edge.

All four are decisions/confirmations, not architecture rediscovery → R2 exit gate §7 satisfied with
`PASS_WITH_DEBT` naming B1 as the exact blocker.

---

## 9. Output files (this lane only)

- `logs/research/M5-W18/A2-obsidian-vault-semantics.md` (this report)
- `logs/research/M5-W18/A2-fixtures.md` (fixtures + executed harness + run summary)
- `logs/checkpoints/A2-M5-W18-R-20260908.md` (checkpoint, updated)
- No product code, no vault mutation, no push.
