#!/usr/bin/env python3
# A10 / M5-W18-R3 research asset (self-contained, synthetic, zero product impact).
#
# Purpose: make the R3 viewport acceptance constraints arithmetically checkable so
# that every lane's "viewport numbers" can be verified instead of asserted.
#
# R3 global acceptance constraints (M5-W18-R3-UX-TASKS-20260908.md):
#   - collapsed mode: >=85% of the post-titlebar HEIGHT and >=92% of the WIDTH for the active content
#   - normal top chrome: at most two compact rows and at most 80px total height
#
# The script only does arithmetic. It does NOT measure a running GUI (no native run was
# performed for this report), so every pixel input is an explicit assumption printed in the output.

TARGETS = [(1920, 1080), (1440, 900), (1366, 768), (1024, 720)]
HEIGHT_SHARE = 0.85
WIDTH_SHARE = 0.92
TOP_CHROME_MAX_PX = 80
# Linux window decorations are window-manager dependent; R3 speaks of the post-titlebar
# height, so T is swept instead of guessed once.
TITLEBAR_SWEEP = [0, 32, 37]


def fmt(x):
    return f"{x:8.1f}"


def main():
    print("A10-R3 viewport budget — derived limits (no GUI measurement)\n")

    print("== A. width budget: how much SIDE chrome may remain in collapsed mode ==")
    print("   rule: (W - side) / W >= 0.92  ->  side <= 0.08 * W")
    print(f"   {'frame':>10} {'max side px':>12} {'48px rail':>10} {'2x48 rails':>11} {'260px panel':>12}")
    for (w, h) in TARGETS:
        mx = (1 - WIDTH_SHARE) * w
        ok1 = "PASS" if 48 <= mx else "FAIL"
        ok2 = "PASS" if 96 <= mx else "FAIL"
        ok3 = "PASS" if 260 <= mx else "FAIL"
        print(f"   {w}x{h:<5} {fmt(mx)} {ok1:>10} {ok2:>11} {ok3:>12}")
    print()

    print("== B. height budget: how much VERTICAL chrome may remain in collapsed mode ==")
    print("   rule: (Hpost - chrome) / Hpost >= 0.85  ->  chrome <= 0.15 * Hpost, Hpost = H - titlebar")
    print(f"   {'frame':>10} {'titlebar':>9} {'max chrome px':>14} {'cur 34+26=60':>14} {'240px bottom':>13}")
    for (w, h) in TARGETS:
        for t in TITLEBAR_SWEEP:
            hp = h - t
            mx = (1 - HEIGHT_SHARE) * hp
            cur = "PASS" if 60 <= mx else "FAIL"
            bot = "PASS" if 240 <= mx else "FAIL"
            print(f"   {w}x{h:<5} {t:>9} {fmt(mx)} {cur:>14} {bot:>13}")
    print()

    print("== C. top chrome rows ==")
    print(f"   rule: <=2 rows and <= {TOP_CHROME_MAX_PX}px total")
    rows = [("current ActivityBar primary row (24px control + row padding)", 34),
            ("ActivityBar omni expand row (max-height: 74px, source)", 74),
            ("StatusBar (height: 26px, source)", 26)]
    for name, px in rows:
        print(f"   {name:<60} {px:>4}px  {'OK' if px <= TOP_CHROME_MAX_PX else 'OVER'}")
    print("   -> primary row + one 74px omni row = 108px: OVER the 80px cap if both are 'normal'.")
    print()

    print("== D. product frame facts (source, not measured) ==")
    print("   main window inner_size = 1200x800, min_inner_size = 900x600 (src-tauri/src/main.rs:1225-1226)")
    print("   1024x720 is above min_inner_size -> reachable; it is NOT the shipping default size.")
    print("   Linux decoration height is WM-dependent -> post-titlebar height must be measured, not assumed.")


if __name__ == "__main__":
    main()
