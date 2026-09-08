#!/usr/bin/env python3
# A2-R2B Obsidian-derivation harness (self-contained; NO real vault, NO /tmp dependency).
# Verifies the W19 derivation RULES' determinism. NOTE (per A0 audit #4): a 20/20 PASS here
# proves this harness is internally consistent, NOT that Obsidian behaves identically. Obsidian
# behavior must be confirmed against official docs / real-vault observation before W19 adopts it.
import os, re, sys, unicodedata, json

VAULT = sys.argv[1] if len(sys.argv) > 1 else None
if not VAULT:
    sys.stderr.write("usage: A2-R2B-resolve.py <vault_dir>\n")
    sys.exit(2)
# Plain wikilink; embeds are removed first so ![[x]] is never double-counted as a broken wikilink.
WIKILINK = re.compile(r"\[\[([^\[\]]+?)\]\]")
EMBED = re.compile(r"!\[\[([^\[\]]+?)\]\]")
SECRET_PAT = ("sk-", "AKIA", "Bearer ", "eyJ", "-----BEGIN")
SECRET_KEYS = ("token", "password", "secret", "api_key")

def norm_text(s): return unicodedata.normalize("NFC", s)
def match_key(s): return norm_text(s).casefold()
def strip_md(p): return p[:-3] if p.endswith(".md") else p
def shortest(c): return sorted(c, key=lambda p: (len(p.split("/")), p))[0]

def parse_frontmatter(text):
    m = re.match(r"^---\n(.*?)\n---\n", text, re.S)
    aliases, tags = [], []
    if m:
        body = m.group(1)
        am = re.search(r"aliases:\s*\[(.*?)\]", body)
        if am: aliases = [a.strip().strip("'\"") for a in am.group(1).split(",") if a.strip()]
        tm = re.search(r"tags:\s*\[(.*?)\]", body)
        if tm: tags = [t.strip().strip("'\"") for t in tm.group(1).split(",") if t.strip()]
    return aliases, tags

def ignored(rel): return rel.startswith("ignored/") or rel.endswith(".py")

def build_index(files):
    by_path, by_base, by_alias = {}, {}, {}
    for rel, text in files:
        if ignored(rel): continue
        key = strip_md(rel)
        by_path[key] = text
        by_base.setdefault(match_key(key), []).append(key)
        for a in parse_frontmatter(text)[0]:
            by_alias.setdefault(match_key(a), []).append(key)
    return by_path, by_base, by_alias

def resolve(target, by_path, by_base, by_alias):
    t = strip_md(target.strip())
    if t in by_path: return t, "path"
    if match_key(t) in by_base:
        c = by_base[match_key(t)]; return (shortest(c), "basename") if len(c)==1 else (shortest(c), "basename-ambiguous")
    if match_key(t) in by_alias:
        c = by_alias[match_key(t)]; return (shortest(c), "alias") if len(c)==1 else (shortest(c), "alias-ambiguous")
    return None, "unresolved"

def attachment_exists(target): return os.path.exists(os.path.join(VAULT, target.strip()))

def derive(files):
    by_path, by_base, by_alias = build_index(files)
    edges, backlinks = [], {}
    for rel, text in by_path.items():
        for raw in EMBED.findall(text):                       # embeds first
            target = raw.split("|", 1)[0] if "|" in raw else raw
            if "#" in target: target = target.split("#", 1)[0]
            if attachment_exists(target.strip()):
                edges.append((rel, target.strip(), "embed", False)); backlinks.setdefault(target.strip(), []).append(rel)
            else:
                edges.append((rel, raw, "embed", True)); backlinks.setdefault(raw, []).append(rel)
        cleaned = EMBED.sub("", text)
        for raw in WIKILINK.findall(cleaned):                 # then plain wikilinks
            target = raw.split("|", 1)[0] if "|" in raw else raw
            anchor = None
            if "#" in target: target, anchor = target.split("#", 1)
            tgt, how = resolve(target, by_path, by_base, by_alias)
            if tgt is None:
                edges.append((rel, raw, "link", True)); backlinks.setdefault(raw, []).append(rel)
            else:
                edges.append((rel, tgt, "link", False)); backlinks.setdefault(tgt, []).append(rel)
    touched = set()
    for s, d, k, un in edges: touched.add(s); touched.add(d)
    orphans = sorted(set(by_path) - touched)
    return by_path, edges, backlinks, orphans

def privacy_ok(props):
    for k in props:
        if any(n in k.lower() for n in SECRET_KEYS): return False
    for v in props.values():
        if any(p in v for p in SECRET_PAT): return False
    return True

def main():
    files = []
    for root, _, fs in os.walk(VAULT):
        for f in fs:
            if f.endswith(".md"):
                rel = os.path.relpath(os.path.join(root, f), VAULT).replace(os.sep, "/")
                files.append((rel, open(os.path.join(root, f), encoding="utf-8").read()))
    by_path, edges, backlinks, orphans = derive(files)
    _, by_base, by_alias = build_index(files)
    R = []
    def chk(name, cond, detail=""):
        R.append((name, "PASS" if cond else "FAIL", str(detail)))
    chk("precedence-filename>alias:[[Alpha]]->Alpha", resolve("Alpha", by_path, by_base, by_alias) == ("Alpha", "path"), resolve("Alpha", by_path, by_base, by_alias))
    chk("alias:[[See C]]->C", resolve("See C", by_path, by_base, by_alias) == ("C", "alias"), resolve("See C", by_path, by_base, by_alias))
    chk("plain:[[B]]->B", resolve("B", by_path, by_base, by_alias)[0] == "B", resolve("B", by_path, by_base, by_alias))
    chk("path-qualified:[[sub/B]]->sub/B", resolve("sub/B", by_path, by_base, by_alias) == ("sub/B", "path"), resolve("sub/B", by_path, by_base, by_alias))
    chk("unresolved:[[Missing Note]]", resolve("Missing Note", by_path, by_base, by_alias)[1] == "unresolved", resolve("Missing Note", by_path, by_base, by_alias))
    chk("case-insensitive:[[ALPHA]]->Alpha", resolve("ALPHA", by_path, by_base, by_alias)[0] == "Alpha", resolve("ALPHA", by_path, by_base, by_alias))
    chk("dup-basename:[[Note]]->Note(root)", resolve("Note", by_path, by_base, by_alias)[0] == "Note", resolve("Note", by_path, by_base, by_alias))
    a_edges = [(d, k, un) for (s, d, k, un) in edges if s == "A"]
    chk("A->B link", ("B", "link", False) in a_edges, a_edges)
    chk("A->C link(display alias)", ("C", "link", False) in a_edges, a_edges)
    chk("A->self via alias-A (filename precedence -> Alpha)", ("Alpha", "link", False) in a_edges, a_edges)
    chk("A->Missing unresolved", ("Missing Note", "link", True) in a_edges, a_edges)
    chk("A embed img.png (attachment, resolved, NOT broken link)", ("img.png", "embed", False) in a_edges, a_edges)
    chk("A->self via #Section heading", ("A", "link", False) in a_edges, a_edges)
    a_back = sorted(set(backlinks.get("A", [])))
    chk("backlinks-A", a_back == sorted(["A", "B", "C", "Note", "Alpha", "alpha", "secret", "sub/B"]), a_back)
    chk("orphan-D", "D" in orphans, orphans)
    chk("tag proj nodes", sorted(k for k, t in by_path.items() if "proj" in parse_frontmatter(t)[1]) == sorted(["A", "B", "C"]), None)
    chk("privacy-secret-flagged", privacy_ok({"token": "sk-abc123"}) is False, None)
    chk("privacy-clean-ok", privacy_ok({"aliases": "X", "tags": "proj"}) is True, None)
    chk("ignored .py excluded", not any(r.endswith(".py") for r in by_path), list(by_path.keys()))
    npass = sum(1 for _, s, _ in R if s == "PASS")
    print(json.dumps({"vault_files": len(files), "nodes": len(by_path), "edges": len(edges),
                      "orphans": orphans,
                      "assertions": [{"name": n, "status": s, "detail": d} for n, s, d in R],
                      "summary": {"pass": npass, "fail": len(R) - npass}}, ensure_ascii=False, indent=2))
    return 1 if (len(R) - npass) else 0

if __name__ == "__main__":
    sys.exit(main())
