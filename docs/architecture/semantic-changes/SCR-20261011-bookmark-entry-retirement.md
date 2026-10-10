# 2026-10-11 BrowserOS Bookmark dead contribution retirement

## Evidence and compatibility decision
- Product v1 exclusively renders the `ADDRESS_BAR_ACTIONS` slot in ActivityBar; the legacy `ACTIVITY_BAR_TRAILING` is not rendered.
- `bookmark.entry-button` is an internal contribution, not a stable public API; remove both runtime registration and Manifest declaration together, plus its async component file.
- Preserve the generic `ACTIVITY_BAR_TRAILING` slot in the typed Contribution registry to avoid breaking third-party capability compatibility.
- Preserve `bookmark.sidebar` and `bookmark.address-star` with their current store / lifecycle; do not alter browser, grid or workspace native behavior.
- `scripts/check-bookmark-single-entry.mjs` prevents reintroduction. Existing hot-plug and composition acceptance still must PASS.
- Do not claim a numeric installation-size reduction until matching build artifacts have been measured.
