# A2 · M5-W18-R Research: Obsidian Vault Semantics Reverse-Engineering

> Lane A2 is **RESEARCH ONLY**. This report reverse-engineers Obsidian vault semantics from the
> local vault at `/home/ainfinit/Documents/Knowledge-Base/secondBrain/` and public Obsidian
> behavior. It does **not** modify product code, the vault, or any dependency. Each proposed
> feature is classified per the W18-R copy policy. Behavior-compatible test fixtures live in
> `A2-fixtures.md` (synthetic, never written into the real vault).

```text
LANE=A2
STATUS=PASS
BASE=78d2cfb8e90d323d35df920e9807d32189f867cd   (origin/master)
HEAD=78d2cfb8e90d323d35df920e9807d32189f867cd   (no commit; canonical copy, not pushed — see checkpoint)
REFERENCE_EVIDENCE=see SOURCE_MAP below (exact local paths + pinned none — Obsidian is local config, not a repo)
FILES=logs/research/M5-W18/A2-obsidian-vault-semantics.md, logs/research/M5-W18/A2-fixtures.md, logs/checkpoints/A2-M5-W18-R-20260908.md
CLASSIFICATION=COPY=0, ADAPT=0, REIMPLEMENT_FROM_BEHAVIOR=11, DEFER=1, REJECT=Obsidian IP (policy)
VERIFY=read-only inspection of vault + `.obsidian/*` + product `graph.rs`/`domain.rs`; no product tests executed (research boundary). Fixtures defined for W19 reuse.
CHECKPOINT=logs/checkpoints/A2-M5-W18-R-20260908.md
MERGE_NOTES=feeds A3 (graph UX blueprint), A10 (parser dependency ledger: markdown/YAML/glob), A11 (test matrix). Depends on A1 baseline for capacity budget.
NEXT=W19 implementation cards: vault-scan → link-resolve → graph-derive pipeline reusing `GraphStore` capacity/privacy guards.
```

---

## 1. Current-product gap

The product (`mvp-browser-os-v3`) already has a **generic** knowledge graph, not a notes vault:

- `src-tauri/src/graph.rs` — pure-logic `GraphStore`/`GraphState` with validation, capacity bounds
  (`GRAPH_MAX_NODES`, `GRAPH_MAX_EDGES`), and a privacy double-scan
  (`graph_props_contain_secret`, `SENSITIVE_KEY_NAMES`, `SENSITIVE_VALUE_PATTERNS`).
- `src-tauri/src/domain.rs` DTOs: `GraphNode` (2082), `GraphEdge` (2092), `GraphNodeView`,
  `GraphEdgeView`, `GraphQueryRequest` (2143), `GraphQueryLimits` (2158), `GraphQueryResult` (2165),
  `GraphStats` (2177).

**Observed gap (grep, read-only):**
- `grep -rni "wikilink|backlink|outgoing.?link" src/` → **0 matches**. The frontend has no note/vault
  link model.
- `grep` over `src-tauri/src` matches only the generic `GraphNode`/`GraphEdge`/`GraphStore` — there is
  **no** concept of a markdown note, alias, frontmatter, tag, or backlink *derived from markdown*.
- The graph is populated by Skill/Agent/knowledge nodes today; it has no importer that turns a markdown
  vault into nodes+edges.

**Conclusion:** W19, if it adopts Obsidian-compatible vault semantics, must add a **read-only
derivation layer** (markdown → links → resolved nodes/edges) that feeds the *existing* bounded
`GraphStore`. It must not reimplement capacity/privacy — those guards are reused as-is.

---

## 2. Obsidian vault semantics (reverse-engineered)

### 2.1 Markdown & wikilinks
- Syntax: `[[Note]]`, `[[Note|Display]]` (display alias), `[[Folder/Note]]` / `[[Folder/Note|Display]]`
  (path-qualified), `[[Note#Heading]]`, `[[Note#^blockid]]`, `[[Note#Heading|Display]]`.
- Resolution target = note **basename without `.md`**; path-qualified links match the vault-relative
  path (folders joined by `/`). Obsidian is case-insensitive on some filesystems; the product must
  pick a deterministic normalization (recommend: NFC + case-fold for matching, preserve original for display).
- **Observed in vault (real evidence):**
  - plain: `[[使用说明]]`, `[[Shell脚本手册]]`, `[[Code-to-Knowledge-Graph]]`
  - display alias: `[[phantom-db/README|V1 注释优先版]]`, `[[功能域索引|按业务功能域检索表]]`
  - path-qualified: `[[phantom-db/README]]`, `[[by-module/phantom_admin]]`
  - 3420+ wikilink occurrences across the vault.

### 2.2 Aliases
- Frontmatter `aliases:` (YAML array) lets a note be linked by an alternate name.
- **Observed:** `TuGraph深度文档.md` line 3 → `aliases: [TuGraph深度文档, TuGraph架构与集成]`, and the
  body links `[[TuGraph深度文档]]` resolve to that note via the alias.
- Resolution order: exact basename → exact path → any `aliases` entry.

### 2.3 Headings & block references
- `[[Note#Heading]]` links to a heading anchor; `[[Note#^blockid]]` links to a block (Obsidian assigns
  a stable `^id` or auto-block-id). Heading text is normalized (punctuation/whitespace stripped) for anchor matching.
- **Not observed** in this vault (`#\^` / ` ^id` = 0 matches) but is a first-class Obsidian feature and
  must be specified for W19 so heading/block links don't become false "unresolved".

### 2.4 Tags
- Two forms:
  - **Frontmatter (YAML):** `tags: [AI, 自动化, cron, 教程]` — observed widely; also nested-style
    `tags: [过期待复核, 据backV8核验, 已被v6取代]`.
  - **Inline:** `#tag` anywhere in body (not observed in sampled grep but standard behavior).
- Tags are first-class graph entities in Obsidian (tag nodes / tag filters). `graph.json` here sets
  `showTags:false`, so tags are *parsed* but not drawn as separate nodes in this config.

### 2.5 Frontmatter / properties
- YAML block delimited by `---` at top of file. Parsed by the `properties` core plugin.
- Keys may carry types (text/number/date/checkbox/list). For vault semantics, only `aliases` and
  `tags` are required for link/tag derivation; other properties are metadata → map to `GraphNode.props`
  (subject to capacity + privacy scan).

### 2.6 Attachments & embeds
- Embed: `![[file.png]]` / `![[note]]` (transclusion) / `![alt](path)` (standard markdown image).
- Attachments live in the vault (e.g. `技术图表库` has 73 `*.png`). `graph.json` here sets
  `showAttachments:false` → attachment nodes hidden in graph, but embeds still create a reference edge.
- **Not observed** as `![[` in this vault (0 matches) — but the reference edge type must exist so
  embeds aren't mistaken for unresolved note links.

### 2.7 Unresolved links
- A wikilink whose target note/alias/path does not exist is **unresolved**. Obsidian still records it
  (renders as a "create" link; in graph it appears as a dashed/unresolved node when `hideUnresolved:false`).
- **Observed config:** `graph.json` → `"hideUnresolved": false` → unresolved nodes are shown.
- The product must model this: a link may point to a node that has no file (placeholder), and the edge
  is flagged `unresolved` so it doesn't pollute "broken" metrics incorrectly.

### 2.8 Backlinks / outgoing links
- **Computed**, not stored: outgoing = links from note N; backlinks = inverse, aggregated per target.
- Core plugins `backlink` and `outgoing-link` are **enabled** (`core-plugins.json`) → these are active
  semantics the product would need to reproduce (a note panel listing inbound links).

### 2.9 Ignore filters (two distinct mechanisms)
1. **Graph view search query** (`graph.json` → `search`): an *Obsidian search-language* expression.
   Observed: `-(path:phantom-wiki AND ext:py) -(path:phantom-wiki AND ext:sh) -(path:phantom-wiki AND ext:pkl) -(path:phantom-wiki AND ext:json) -(path:phantom-wiki AND (name:__pycache__ OR name:.venv))`.
   This is boolean query syntax (`AND`/`OR`/`path:`/`ext:`/`name:`/`-` negation), **not** glob.
2. **Global user ignore filters** (`app.json` → `userIgnoreFilters`): glob patterns. Observed:
   `phantom-wiki/**/*.py`, `phantom-wiki/**/*.pyc`, `phantom-wiki/**/*.sh`, `phantom-wiki/**/*.pkl`,
   `phantom-wiki/**/*.json`, `phantom-wiki/**/.gitignore`, `phantom-wiki/**/requirements.txt`,
   `phantom-wiki/**/__pycache__`, `phantom-wiki/**/.venv`. These exclude files from graph, search, and
   other indexes globally.
- **Implication:** the product needs *both* a glob ignore (cheap, pre-scan) and a query-language filter
  (graph-view-time). The query language is a REIMPLEMENT; glob matching can COPY a glob crate (A10 ledger).

### 2.10 Rename / delete behavior
- **Rename:** Obsidian updates all `[[links]]` pointing to the renamed note (with a confirm modal
  listing N affected files); link text may use the old name, the new name, or an alias.
- **Delete:** removes the file; all its inbound links become unresolved (cascade), not auto-deleted.
- **Classification:** this is a **write/mutation path** (outside read-only research). It must be
  DEFERRED to W19 with a transaction + explicit user confirmation; the read-only *derivation* of
  backlinks/outgoing/graph is REIMPLEMENT. Flagged DEFER below.

### 2.11 Graph data derivation
- **Nodes:** every markdown note (plus, optionally, tag/attachment nodes per config).
- **Edges:** one directed edge per wikilink occurrence (note → target); unresolved targets become
  placeholder nodes with an `unresolved` edge flag.
- **Filters applied at view time:** tag whitelist/blacklist, folder/path include-exclude, link-type,
  the graph `search` query, `colorGroups` (node coloring), `depth` (local graph hop limit),
  `showOrphans` (nodes with no edges), `hideUnresolved`.
- **Observed config:** `showOrphans:true`, `hideUnresolved:false`, `showTags:false`,
  `showAttachments:false`, `showArrow:false`, `colorGroups:[]`, plus force-layout knobs
  (`centerStrength`, `repelStrength`, `linkStrength`, `linkDistance`, `scale`).
- The layout/physics knobs are UI concerns (A3); A2 only specifies the **derived graph model + filters**.

---

## 3. Data / control flow (proposed derivation pipeline)

```text
vault files
  │  (apply global userIgnoreFilters globs first — cheap pre-scan)
  ▼
scan each .md
  │  strip frontmatter (YAML) → props (aliases, tags, others)
  │  inline parse → wikilinks [[..]], embeds ![[..]]/![](), #tags, #heading/#^block
  ▼
resolve each link:
  basename → path → alias  (deterministic normalize: NFC + case-fold match, keep original display)
  │  miss → placeholder node + unresolved edge
  ▼
build GraphNode (id=vault-rel-path minus .md, kind=Note, props=aliases/tags/meta)
build GraphEdge (kind=Link|Embed, unresolved flag, source/target)
  │
  ▼
derive backlinks = inverse index; orphans = nodes with no edges
  │  (reuse GraphStore capacity: GRAPH_MAX_NODES/EDGES, GRAPH_PROPS_MAX_ENTRIES, privacy scan)
  ▼
graph view = GraphQueryRequest(filters: tags/path/link-type/search-query, depth, colorGroups)
```

All of this is **read-only** w.r.t. the vault. Write paths (rename/delete link-rewrite) are DEFERRED.

---

## 4. Persistence format

- **Obsidian:** notes are plain `.md` files on disk; structure is the folder tree. No central database.
  Config is JSON in `.obsidian/` (`graph.json`, `app.json`, `core-plugins.json`, `workspace.json`).
- **Product contrast:** the product persists graph state in **SQLite** via `database.rs` (per
  `graph.rs` header). So the derived vault graph should be *materialized into* the existing
  `GraphStore`/SQLite, not replicate Obsidian's file-only model. The markdown vault becomes an
  *import source*, not the system of record (unless W19 decides otherwise — out of A2 scope).

---

## 5. Concurrency / lifecycle

- Obsidian is single-process with a file watcher that reconciles external edits (debounced).
- Product: derivation should be **idempotent and re-runnable** (re-scan replaces the derived slice);
  respect the existing single SQLite connection in `database.rs` (no second execution path — consistent
  with `GRAPH_NO_SECOND_PATH` boundary in `graph.rs`).

---

## 6. Security / privacy

- A vault is **local plaintext**; notes routinely contain secrets (the product's own
  `SENSITIVE_KEY_NAMES`/`SENSITIVE_VALUE_PATTERNS` exist precisely for this). When materializing note
  *props* (frontmatter) into `GraphNode.props`, the **existing `graph_props_contain_secret` double-scan
  must run** — never store token/api_key/secret/password values or `sk-`/`AKIA`/`Bearer `/`eyJ`/`-----BEGIN`
  patterns.
- No remote egress: derivation is local; do **not** send note bodies to any embedding service unless
  A9's remote-embedding authorization gate is implemented (out of A2 scope).
- Release-origin gate (WORKSPACE_IDENTITY): this is product-code-adjacent; any W19 command touching
  vaults must land with source-check + ACL + typed bridge + policy tests together (per non-repeat gate).

---

## 7. Performance / capacity

- This vault is **large**: `00-知识索引` ~2497 files, `SQL翻译三层` ~3182 files, plus phantom-db* trees,
  totals **~7000+ markdown files**. Naive O(N²) link resolution is unacceptable.
- **Reuse product capacity guards:** `GRAPH_MAX_NODES`, `GRAPH_MAX_EDGES`, `GRAPH_PROPS_MAX_ENTRIES=64`,
  `GRAPH_LABEL_MAX_BYTES`, `GRAPH_PROPS_MAX_BYTES` — the derived graph must stay within these, with a
  bounded scan (skip/truncate beyond limits, surface a capacity error, never OOM).
- Build an **in-memory resolve index** (basename→path, alias→path, path→path) once per scan; links
  resolve via hash map, not per-link filesystem walk.

---

## 8. Dependencies / licenses

- **Obsidian is NOT a source-code donor** (W18-R copy policy). REJECT copying Obsidian code, icons,
  branding, or assets. Reproduce only observable behavior + documented/local vault formats.
- **Third-party parser libs** (markdown parser, YAML parser, glob matcher) are *dependency decisions*
  owned by **A10** (source-transplant ledger + BOM). A2 flags them as needed but does **not** classify
  them COPY/ADAPT — that is A10's call. Apache-2.0 dbx / zvec-grep are irrelevant to A2.
- No new Cargo/npm dependency may be added in W18-R (research frozen). W19 may add after A10 verdict.

---

## 9. Target mapping (Obsidian concept → product construct)

| Obsidian concept | Product target | Notes |
|---|---|---|
| markdown note | `GraphNode` (kind=`Note`, id = vault-rel-path sans `.md`) | reuse `domain.rs:2082` |
| wikilink / embed | `GraphEdge` (kind=`Link`/`Embed`, `unresolved` flag) | reuse `domain.rs:2092` |
| alias / tag / prop | `GraphNode.props` | run `graph_props_contain_secret` + `GRAPH_PROPS_MAX_ENTRIES` |
| backlink index | derived inverse of `GraphEdge` set | not stored; computed per query |
| graph view (filters/depth/colorGroups) | `GraphQueryRequest` + `GraphQueryLimits` | reuse `domain.rs:2143/2158` |
| resolved graph | `GraphStore` / `GraphState` | reuse `graph.rs:185/311` + capacity |
| glob ignore | pre-scan filter (glob crate via A10) | cheap, before parse |
| search-query filter | REIMPLEMENT small boolean query evaluator | graph-view-time |

No new module is *required* for the model — it maps onto the existing bounded graph. A thin
read-only **vault-derivation** module (importer) is the only new code W19 would add.

---

## 10. Test reuse

- Define behavior-compatible fixtures in `A2-fixtures.md` (synthetic vault, never written to real
  vault). W19 tests assert: resolved nodes/edges, alias resolution, path-qualified resolution,
  unresolved placeholder + flag, backlink inverse, tag/folder/search filtering, orphan detection,
  capacity truncation, and privacy redaction of secret props.
- Reuse product's existing graph bounded-query tests (`graph.rs`) by feeding derived nodes/edges
  through the same `GraphStore` validation path.

---

## 11. Classification

| # | Feature | Class | Reason |
|---|---|---|---|
| 1 | Markdown + wikilink extraction | REIMPLEMENT_FROM_BEHAVIOR | No Obsidian code; custom scanner over a markdown parse. |
| 2 | Alias resolution (frontmatter) | REIMPLEMENT_FROM_BEHAVIOR | Behavior-only; basename→path→alias order. |
| 3 | Heading / block (`#h`, `#^id`) links | REIMPLEMENT_FROM_BEHAVIOR | Behavior-only; anchor normalization. |
| 4 | Tags (frontmatter YAML + inline `#tag`) | REIMPLEMENT_FROM_BEHAVIOR | Behavior-only; parser lib is A10's call. |
| 5 | Frontmatter / properties parse | REIMPLEMENT_FROM_BEHAVIOR | YAML parse; lib dependency → A10. |
| 6 | Attachments / embeds (`![[`, `![]()`) | REIMPLEMENT_FROM_BEHAVIOR | Reference-edge type; behavior-only. |
| 7 | Unresolved links / placeholder nodes | REIMPLEMENT_FROM_BEHAVIOR | Behavior-only; edge `unresolved` flag. |
| 8 | Backlinks / outgoing derivation | REIMPLEMENT_FROM_BEHAVIOR | Computed inverse; behavior-only. |
| 9 | Ignore filters: glob (`userIgnoreFilters`) | REIMPLEMENT_FROM_BEHAVIOR (glob match may COPY a crate via A10) | Behavior-only; matcher is A10 ledger. |
| 10 | Ignore filters: search-query language | REIMPLEMENT_FROM_BEHAVIOR | Small boolean evaluator; behavior-only. |
| 11 | Graph data derivation (nodes/edges/filters/depth/orphans/colorGroups) | REIMPLEMENT_FROM_BEHAVIOR | Maps onto `GraphStore` + `GraphQueryRequest`. |
| 12 | Rename / delete link-rewrite (write path) | **DEFER** | Mutation requiring transaction + user confirm; out of read-only research. Derivation half is REIMPLEMENT. |
| — | Copying Obsidian source / icons / branding | **REJECT** (policy) | W18-R copy policy: reproduce behavior only. |

Counts: **COPY=0, ADAPT=0, REIMPLEMENT_FROM_BEHAVIOR=11, DEFER=1, REJECT=Obsidian IP (policy)**.

---

## 12. Unresolved questions (for A0 / W19)

1. Does W19 want a **full vault import** (notes as graph nodes) or only a **link-graph** view?
   A2 assumes link-graph + note metadata; full note-body storage is a larger decision.
2. Folder-path links (`[[Folder/Note]]`): match by vault-relative path exactly, or also by basename
   fallback? Recommend exact-path-first, then basename, to mirror Obsidian.
3. Transclusion (`![[note]]`) rendering is a UI concern (A3); A2 only needs the reference edge type.
4. Block-ref granularity: store block id only, or also block text for preview? Recommend id-only in
   the graph; text preview is A3.
5. Rename/delete link-rewrite: which links to update (basename-only, path, alias)? Needs W19 write
   transaction + confirm UI; DEFER.
6. Capacity: if a vault exceeds `GRAPH_MAX_NODES`, truncate by recency/size and surface a capacity
   error — confirm the product's preferred degradation policy with A1's budget.

---

## 13. Behavior-compatible test fixtures

See `A2-fixtures.md` (synthetic vault + expected derived graph + filter outcomes). These are
**definitions only**; they are never written into `/home/ainfinit/Documents/Knowledge-Base/secondBrain/`.
