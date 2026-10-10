# SCR-20261010：BrowserOS 界面低频操作状态与收藏夹入口收口

> Scope: BrowserOS PR #20. This change refines UI only and preserves the existing semantic, native lifecycle, and package-size gates.

## Existing canonical semantics

- File operations and their data remain owned by `useFileStore`. The new `fileActionsOpen` is a **component-local, presentation-only Boolean** for revealing low-frequency file buttons. It is not file data, not persisted, and cannot mutate or mirror the File Store.
- `useBookmarkStore.togglePanel` remains the **only canonical bookmark-panel visibility intent** and `panelOpen` its sole owner state. `layout.activateBrowser` remains the canonical surface navigation intent.
- The address-bar button's `onBookmarkClick` is a UI event adapter that delegates those existing intents; it does not create a second bookmark-panel intent, owner, or writer.

## Registry decision

- Register `fileActionsOpen` under `states.yaml > observed_not_governed > FilePanel.vue`, alongside the existing presentation-only `ftreeBody`. This is explicit inventory registration, **not** promotion to governed business state.
- Reuse `togglePanel` directly; remove the duplicate `openBookmarkPanel` intent wrapper. No changes to `intents.yaml` or its existing `duplicate_names` protection.
- No new IPC, native resource lifecycle, credential store, persistent storage, or second source of truth is created. No ADR is required for a local UI visibility flag.

## Packaging and verification contract

- Remove dead address-bar/bookmark logic and unused component-scoped styles whose corresponding controls were removed by UI v1; retain current navigation, keyboard behavior, contribution registration, and native-safe inline overlays.
- Do **not** change `TOTAL_BYTES_GROWTH_LIMIT_PCT`, the 052b18a baseline metrics, or the pre-merge script. The built `dist` must meet the original 25.2% growth budget and cargo warning non-regression.
- Required gates: `node scripts/check-semantic-registry.mjs --self-test`, `node scripts/check-semantic-registry.mjs`, `npm run check`, `npm run build`, `python3 scripts/measure-build-metrics.py --compare logs/m0-build-metrics/build-metrics-052b18a.json --skip-build`, and full `pre-merge` on CI.
- Results are facts from CI, not implied by this change request. PR merge is prohibited until all required gates pass.
