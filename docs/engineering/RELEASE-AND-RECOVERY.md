# Release and Recovery Policy

## Current stable baseline

- Release: BrowserOS Capability Platform v4 Stable
- Tag: `capability-platform-v4-stable`
- Commit: `a6ff92674db98ffad964b784167fdb8c9a98f3cc`
- Audit: `docs/architecture/capability-platform/FINAL-WORTHWHILE-HOTPLUG-AUDIT-V4-20261008.md`

The tag identifies the accepted product/platform state. Later docs or engineering-infrastructure commits on `master` do not redefine v4.

## Freeze rules

1. Never force-move an accepted stable tag.
2. Never rewrite frozen v1-v4 history.
3. A new stable baseline requires full required gates, audit, new tag and GitHub Release.
4. Release notes must name the exact commit/evidence.
5. Gitee may mirror the tag but is not release authority.

## Recovery order

1. Diagnose and preserve evidence.
2. Classify product, Git, packaging or environment failure.
3. Prefer forward repair or `git revert` for published history.
4. Use stable tags for known-good reproduction/comparison.
5. Re-run the relevant gate stack before declaring recovery complete.

Detailed procedures remain under `docs/operations/recovery/`.
