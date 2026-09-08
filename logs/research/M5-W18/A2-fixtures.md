# A2 · M5-W18-R Behavior-Compatible Test Fixtures (synthetic vault)

> These fixtures are **definitions only**. They are NEVER written into the real vault
> (`/home/ainfinit/Documents/Knowledge-Base/secondBrain/`). W19 importers/derivers must reproduce
> the expected derived graph below. All names are ASCII to keep resolution rules unambiguous; the
> real vault uses CJK names and the same rules apply after NFC + case-fold normalization.

## F1 — Synthetic vault layout

```
vault/
  A.md            frontmatter: aliases:[Alpha], tags:[proj,active]
                  body:
                    # Title A
                    See [[B]] and [[C|See C]].
                    Self link [[Alpha]].
                    Broken [[Missing Note]].
                    Inline #urgent tag.
                    Embed ![[img.png]].
                    Jump to [[A#Section]].
                  ## Section
                    text

  B.md            frontmatter: tags:[proj]
                  body: back to [[A]]

  C.md            frontmatter: aliases:[See C]
                  body: [[A#Section]] referenced heading

  img.png         (attachment; not markdown)
```

## F2 — Link extraction (per file)

| File | Outgoing links (raw) | Resolves to | Unresolved? |
|---|---|---|---|
| A.md | `[[B]]`, `[[C|See C]]`, `[[Alpha]]`, `[[Missing Note]]`, `![[img.png]]`, `[[A#Section]]` | B, C, A(alias), —, img.png(attach), A(heading) | `[[Missing Note]]` = YES |
| B.md | `[[A]]` | A | no |
| C.md | `[[A#Section]]` | A (heading) | no |

## F3 — Expected derived graph (nodes)

| Node id | kind | props (post privacy/capacity scan) | file exists? |
|---|---|---|---|
| `A` | Note | aliases=[Alpha], tags=[proj,active] | yes |
| `B` | Note | tags=[proj] | yes |
| `C` | Note | aliases=[See C] | yes |
| `Missing Note` | Note (placeholder) | none | **no** (unresolved) |
| `img.png` | Attachment | none | yes |

Orphans (no edges): none in F1 (every node participates). A fixture F4 below adds an orphan.

## F4 — Expected edges

| source | target | kind | unresolved | note |
|---|---|---|---|---|
| A | B | Link | no | `[[B]]` |
| A | C | Link | no | `[[C|See C]]` display alias |
| A | A | Link | no | `[[Alpha]]` alias self-link |
| A | Missing Note | Link | **YES** | `[[Missing Note]]` |
| A | img.png | Embed | no | `![[img.png]]` |
| A | A | Link | no | `[[A#Section]]` heading (target=A) |
| B | A | Link | no | `[[A]]` |
| C | A | Link | no | `[[A#Section]]` heading |

## F5 — Backlink derivation (inverse)

| target | backlinks (from) |
|---|---|
| A | B, C, A(self via alias), A(heading×2) |
| B | A |
| C | A |
| Missing Note | A |
| img.png | A |

## F6 — Alias resolution rules (must pass)

- `[[Alpha]]` → resolves to `A` (via `aliases:[Alpha]`).
- `[[See C]]` → resolves to `C` (via `aliases:[See C]`), display text "See C".
- `[[A#Section]]` → target `A`, anchor `Section` (heading exists in A).
- `[[Missing Note]]` → no basename/path/alias match → unresolved placeholder.

## F7 — Filter outcomes (graph view)

Using `GraphQueryRequest` semantics mapped in the main report:

| Filter | Result nodes (F1) | Rationale |
|---|---|---|
| none | A,B,C,Missing Note,img.png | all |
| tag includes `proj` | A,B,C | all three carry `proj` |
| tag excludes `active` | B,C,Missing Note,img.png | A dropped (has `active`) |
| path excludes `*` (only root) | A,B,C (if all root) | path filter |
| `hideUnresolved=true` | A,B,C,img.png | `Missing Note` hidden |
| `showOrphans=true` + add orphan `D.md` (no links) | + D | D is orphan |
| search-query `-(ext:png)` | A,B,C,Missing Note | img.png excluded (glob/query) |

## F8 — Capacity / privacy assertions

- Materialize `A.props` through `graph_props_contain_secret`: if a fixture note had
  `api_key: sk-xxxx`, the privacy scan must flag `SecretInProps` and the value must never be echoed.
- If synthetic vault exceeds `GRAPH_MAX_NODES`, derivation must truncate and return a capacity error,
  not OOM. (Drive with a generated 10k-note fixture in W19 perf test.)

## F9 — Negative / edge cases

- `[[A#NoSuchHeading]]` → target `A` exists, heading missing → treat as valid link to A but flag
  `heading_missing` (do NOT mark unresolved; do not create placeholder).
- `[[ ]]` (empty) → ignored (no node, no edge).
- Duplicate `[[B]]` in same file → one outbound edge (dedupe per (source,target,kind)).
- Circular `A→B→A` → both edges present; no cycle error (graphs allow cycles).
