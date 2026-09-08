# A1 · M5-W18-R3 Checkpoint

```text
LANE=A1
DISPATCH=M5-W18-R3-UX
STATUS=READY_FOR_REVIEW
WORKDIR=/home/ainfinit/.codex/worktrees/m5-w18-a1/mvp-browser-os-v3
BRANCH=codex/m5-w18-a1
BASE=200f0f1
HEAD=<pending commit>
CONSUMED_PEERS=A2-A9 pending; A10 review pending — A1 draft first, final after review
FILES=logs/research/M5-W18/A1-R3-browser-workbench.md, logs/research/M5-W18/A1-R3-prototype.html, logs/research/M5-W18/A1-R3-checkpoint.md
CORRECTIONS=R2B prototype rejected (too dense); R3 returns to Chrome-like content-first + IDEA progressive disclosure
VERIFY=HTML prototype browser-openable; viewport measurements inline; no build or product test run
PROPOSED_SLICES=none (R3 is prototype research, not coding authorization)
OPEN_DECISIONS=A10 review may require revision; A2-A9 reports needed for final
NEXT=A10 independent review; then A1 consumes A2-A9 + A10 for final
NO_PRODUCT_CODE=true
NO_PUSH=true
```

## What A1 R3 delivered

### 1. Browser-first replacement shell prototype (`A1-R3-prototype.html`)
- Chrome-like: compact tabs + address bar + large content area
- IDEA-style progressive disclosure: collapsible tool windows, not permanent chrome
- 7 modes: normal / collapsed / focus / database / knowledge / Git(central) / Git(bottom) / context-menu / command-palette
- 4 sizes: 1920×1080 / 1440×900 / 1366×768 / 1024×720
- 4 states: empty / loading / error / disabled
- Git workflow: central-log ↔ bottom-tool-window (reversible, no Rebased branding)

### 2. Design report (`A1-R3-browser-workbench.md`)
- §2 shell design: top chrome ≤80px, edge strip 28px, content-first, tool windows, status bar
- §3 mode specs: 8 modes with exact behavior
- §4 viewport measurements: all sizes × modes with constraint verification
- §7 Git workflow coverage: 18 operations mapped
- §8 diff from rejected R2B prototype

### 3. Constraint compliance
- Top chrome: 68px ✅ ≤80px
- Collapsed content: 93% H / 96% W ✅ ≥85% H / ≥92% W
- Tool windows: open/close/collapse/restore/expand/pin/unpin/auto-hide/resize ✅
- One primary per edge by default ✅
- Focus mode: hide all chrome + one return ✅
- Context menus: scope-aware + grouped + disabled + shortcuts ✅
- Command palette: unified entry + source/scope/disabled ✅
- No large dashboard cards or duplicated navigation ✅

## Corrections from R2B

| # | R2B (rejected) | R3 (replacement) |
|---|---|---|
| 1 | Permanent multi-panel squeeze | Content-first, on-demand tool windows |
| 2 | 48px activity bar with 9 icons | 28px edge strip, icon-only |
| 3 | Always-visible feature buttons | Context menus + command palette |
| 4 | Git as small status panel | Full Git workflow, central/bottom reversible |
| 5 | No context menus | Scope-aware + grouped + disabled + shortcuts |
| 6 | No command palette | Ctrl+Shift+P unified entry |

## Boundary compliance

- Zero product-code changes; no dependency install; no build/test run; no push.
- All output under `logs/research/M5-W18/A1-R3-*` only.
- Prototype HTML is self-contained with synthetic data only.
- No Rebased branding, assets, or JetBrains platform code copied.
- R1/R2/R2B reports preserved (integrated into master by A0).

## Unresolved questions dispatched

- U1 (A2 density audit) → A2
- U2 (A3 progressive disclosure) → A3
- U3 (A7 Git reference) → A7
- U4 (A10 independent review) → A10
- U5 (native WebView verification) → A9/A11
