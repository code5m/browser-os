#!/usr/bin/env python3
# A2-R2B synthetic Obsidian vault fixture generator (NO real vault, NO /tmp dependency).
# Writes a 10-file synthetic vault illustrating every resolution rule into --out (default: tempdir).
# Reproducible: run `python3 A2-R2B-resolve.py $(python3 A2-R2B-make_vault.py)`.
import os, sys, tempfile

def main():
    out = sys.argv[1] if len(sys.argv) > 1 else tempfile.mkdtemp(prefix="a2fix-")
    v = os.path.join(out, "vault")
    os.makedirs(os.path.join(v, "sub"), exist_ok=True)
    os.makedirs(os.path.join(v, "ignored"), exist_ok=True)
    def w(rel, text):
        p = os.path.join(v, rel)
        os.makedirs(os.path.dirname(p), exist_ok=True)
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
    open(os.path.join(v, "img.png"), "wb").write(b"\x89PNG\r\n")
    w("ignored/skip.py", "print('should be ignored')\n")
    w("secret.md", "---\naliases: [S]\ntoken: sk-abc123\n---\n# S\n[[A]]\n")
    print(os.path.join(out, "vault"))
    return 0

if __name__ == "__main__":
    sys.exit(main())
