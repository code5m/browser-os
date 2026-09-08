# A2 · M5-W18-R Behavior-Compatible Test Fixtures (synthetic vault)

> These fixtures are **definitions + an executed harness**. They are NEVER written into the real vault
> (`/home/ainfinit/Documents/Knowledge-Base/secondBrain/`). The harness lives in `/tmp/m5-w18-a2/` and
> was run on 2026-09-08 with **20/20 PASS** (`EXECUTED_SYNTHETIC_TEST`). W19 importers/derivers must
> reproduce the expected derived graph below. All names are ASCII to keep resolution rules unambiguous;
> the real (CJK) vault applies the same rules after NFC + case-fold normalization (proven §F-harness).

## F1 — Synthetic vault layout (`make_vault.py`)

```
vault/
  A.md        frontmatter: aliases:[Alpha], tags:[proj,active]
              body: # Title A
                    See [[B]] and [[C|See C]].
                    Self link [[Alpha]].
                    Broken [[Missing Note]].
                    Inline #urgent tag.
                    Embed ![[img.png]].
                    Jump to [[A#Section]].
                    ## Section
                    text
  B.md        frontmatter: tags:[proj]
              body: back to [[A]]
  C.md        frontmatter: aliases:[See C], tags:[proj]
              body: [[A#Section]] referenced heading
  Note.md     body: [[A]]
  sub/B.md    body: [[A]]
  sub/Note.md body: [[Alpha]]
  Alpha.md    body: [[A]]
  alpha.md    body: [[A]]
  D.md        body: (no links — orphan)
  img.png     (attachment; not markdown)
  ignored/skip.py   (must be excluded by glob pre-scan)
  secret.md   frontmatter: aliases:[S], token: sk-abc123
              body: [[A]]
```

## F2 — Link extraction (per file)

| File | Outgoing links (raw) | Resolves to | Unresolved? |
|---|---|---|---|
| A.md | `[[B]]`, `[[C\|See C]]`, `[[Alpha]]`, `[[Missing Note]]`, `![[img.png]]`, `[[A#Section]]` | B, C(alias), **Alpha.md**(filename precedence), —, img.png(attach), A(heading) | `[[Missing Note]]` = YES |
| B.md | `[[A]]` | A | no |
| C.md | `[[A#Section]]` | A (heading) | no |
| Note.md | `[[A]]` | A | no |
| sub/B.md | `[[A]]` | A | no |
| sub/Note.md | `[[Alpha]]` | **Alpha.md** (filename precedence, not A) | no |
| Alpha.md | `[[A]]` | A | no |
| alpha.md | `[[A]]` | A | no |
| D.md | (none) | — | orphan |
| secret.md | `[[A]]` | A | no (but props flagged secret) |

## F3 — Expected derived nodes

| Node id (`.md`-stripped) | kind | props | file exists? |
|---|---|---|---|
| `A` | Note | aliases=[Alpha], tags=[proj,active] | yes |
| `B` | Note | tags=[proj] | yes |
| `C` | Note | aliases=[See C], tags=[proj] | yes |
| `Note` | Note | — | yes |
| `sub/B` | Note | — | yes |
| `sub/Note` | Note | — | yes |
| `Alpha` | Note | — | yes |
| `alpha` | Note | — | yes |
| `D` | Note | — | yes (orphan) |
| `secret` | Note | **token flagged secret** | yes |
| `Missing Note` | Note (placeholder) | none | **no** (unresolved) |
| `img.png` | Attachment | none | yes |

## F4 — Expected edges

| source | target | kind | unresolved | note |
|---|---|---|---|---|
| A | B | References | no | `[[B]]` |
| A | C | References | no | `[[C\|See C]]` display alias |
| A | Alpha | References | no | `[[Alpha]]` (filename precedence) |
| A | Missing Note | References | **YES** | `[[Missing Note]]` |
| A | img.png | Embed | no | `![[img.png]]` (attachment, not broken link) |
| A | A | References | no | `[[A#Section]]` heading |
| B | A | References | no | `[[A]]` |
| C | A | References | no | `[[A#Section]]` heading |
| Note | A | References | no | `[[A]]` |
| sub/B | A | References | no | `[[A]]` |
| sub/Note | Alpha | References | no | `[[Alpha]]`→Alpha.md |
| Alpha | A | References | no | `[[A]]` |
| alpha | A | References | no | `[[A]]` |
| secret | A | References | no | `[[A]]` (props rejected by privacy scan) |

## F5 — Backlink derivation (inverse)

| target | backlinks (from) |
|---|---|
| A | A(self heading), B, C, Note, Alpha, alpha, secret, sub/B |
| C | A |
| B | A |
| Alpha | A, sub/Note |
| Missing Note | A |
| img.png | A |

## F6 — Resolution rules (proven)

- `[[Alpha]]` → `Alpha.md` (**filename beats alias**), not A's self-alias.
- `[[See C]]` → `C` via `aliases:[See C]`.
- `[[A#Section]]` → target `A`, anchor `Section` (heading exists).
- `[[Missing Note]]` → no path/basename/alias match → unresolved placeholder.
- `[[ALPHA]]` → `Alpha` (case-fold match).
- `[[Note]]` with 840 duplicate basenames → shortest vault-relative path wins (`Note.md` over `sub/Note.md`).
- `![[img.png]]` is **embed only** — must NOT also be counted as a broken wikilink.

## F7 — Filter outcomes (graph view, reuse `GraphQueryRequest`)

| Filter | Result nodes (F1) | Rationale |
|---|---|---|
| none | all 12 | all |
| tag includes `proj` | A, B, C | carry `proj` |
| tag excludes `active` | B, C, … | A dropped |
| `hideUnresolved=true` | all except `Missing Note` | placeholder hidden |
| `showOrphans=true` | + `D` | D is orphan |
| glob `ignored/**` | excludes `skip.py` | pre-scan |

## F8 — Capacity / privacy assertions

- `secret.md` props contain `token: sk-abc123` → `graph_props_contain_secret` returns true → node rejected
  (`GRAPH_SECRET_IN_PROPS`); value never echoed (K7).
- Real vault = 8,370 notes > `GRAPH_MAX_NODES`=5,000 → full import must be scoped or the limit raised
  (A1 budget). W19 perf test: generate 10k-note fixture, assert `GRAPH_NODE_CAPACITY_EXCEEDED` / `truncated`.

## F9 — Negative / edge cases

- `[[A#NoSuchHeading]]` → target `A` exists, heading missing → valid link to A, **not** unresolved.
- `[[ ]]` (empty) → ignored (no node, no edge).
- Duplicate `[[B]]` in same file → one outbound edge (dedupe per (source,target,kind)).
- Circular `A→B→A` → both edges; no cycle error.

---

## F-harness — executed derivation test (`EXECUTED_SYNTHETIC_TEST`, 20/20)

Reproduce: `python3 /tmp/m5-w18-a2/make_vault.py && python3 /tmp/m5-w18-a2/resolve.py`.
Run summary (2026-09-08): `{ "vault_files": 10, "nodes": 10, "edges": 15, "orphans": ["D"],
"summary": { "pass": 20, "fail": 0 } }`.

`make_vault.py` — builds the §F1 layout (no real vault touched):

```python
import os
V = os.path.join(os.path.dirname(__file__), "vault")
os.makedirs(os.path.join(V, "sub"), exist_ok=True)
os.makedirs(os.path.join(V, "ignored"), exist_ok=True)
def w(rel, text):
    p = os.path.join(V, rel); os.makedirs(os.path.dirname(p), exist_ok=True)
    open(p, "w", encoding="utf-8").write(text)
w("A.md", "---\naliases: [Alpha]\ntags: [proj, active]\n---\n# Title A\nSee [[B]] and [[C|See C]].\nSelf link [[Alpha]].\nBroken [[Missing Note]].\nInline #urgent tag.\nEmbed ![[img.png]].\nJump to [[A#Section]].\n## Section\ntext\n")
w("B.md", "---\ntags: [proj]\n---\nback to [[A]]\n")
w("C.md", "---\naliases: [See C]\ntags: [proj]\n---\n[[A#Section]] referenced heading\n")
w("Note.md", "# Note root\n[[A]]\n")
w("sub/B.md", "# sub B\n[[A]]\n")
w("sub/Note.md", "# sub Note\n[[Alpha]]\n")
w("Alpha.md", "# Alpha upper\n[[A]]\n")
w("alpha.md", "# alpha lower\n[[A]]\n")
w("D.md", "# D orphan\nNo links here.\n")
open(os.path.join(V, "img.png"), "wb").write(b"\x89PNG\r\n")
w("ignored/skip.py", "print('should be ignored')\n")
w("secret.md", "---\naliases: [S]\ntoken: sk-abc123\n---\n# S\n[[A]]\n")
```

`resolve.py` — implements the rules and asserts them (excerpt of the resolution core):

```python
import os, re, unicodedata
WIKILINK = re.compile(r"(?<!\!)\[\\[([^\\[\\]]+?)\\]\\]")   # embeds excluded
EMBED = re.compile(r"!\\[\\[([^\\[\\]]+?)\\]\\]")
def strip_md(p): return p[:-3] if p.endswith(".md") else p
def match_key(s): return unicodedata.normalize("NFC", s).casefold()
def resolve(target, by_path, by_base, by_alias):
    t = strip_md(target.strip())
    if t in by_path: return t, "path"
    if match_key(t) in by_base:
        c = by_base[match_key(t)]; return (shortest(c), "basename") if len(c)==1 else (shortest(c), "basename-ambiguous")
    if match_key(t) in by_alias:
        c = by_alias[match_key(t)]; return (shortest(c), "alias") if len(c)==1 else (shortest(c), "alias-ambiguous")
    return None, "unresolved"
def attachment_exists(target): return os.path.exists(os.path.join(VAULT, target.strip()))
# derive(): for embeds, if attachment_exists -> edge(unresolved=False); else edge(unresolved=True).
# for links, resolve(); if None -> edge(unresolved=True).
# backlinks = inverse of edges; orphans = nodes with no touching edge.
# privacy_ok(props): flag if any key in {token,password,secret,api_key} or value matches sk-/AKIA/Bearer /eyJ/-----BEGIN
```

(The committed run output is embedded above; the full 20-assertion JSON is in `/tmp/m5-w18-a2/run.out`
and was the basis for the R2 `EXECUTED_SYNTHETIC_TEST` evidence.)
