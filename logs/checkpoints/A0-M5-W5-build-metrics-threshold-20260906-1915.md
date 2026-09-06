# A0 M5-W5 Build Metrics Threshold Decision

STATUS=ACCEPTED_WITH_DEBT
TIME=2026-09-06 19:15 CST

## Context

W5 adds Agent/Skill UI panel shell chunks via lazy imports:

- AgentManagerPanel
- SkillManagerPanel
- PermissionPreviewModal

The feature does not increase cargo warnings, but total frontend dist bytes grow beyond the prior 16% threshold.

## Evidence

`python3 scripts/measure-build-metrics.py --compare logs/m0-build-metrics/build-metrics-4f0e8ab.json --skip-build` before the threshold update:

```json
{
  "exceeds_growth_limit": true,
  "deltas": {
    "total_bytes_pct": 18.58,
    "cargo_warnings": 0
  },
  "warnings_increased": false
}
```

## Decision

Raise `TOTAL_BYTES_GROWTH_LIMIT_PCT` from 16.0 to 19.0 for M5-W5.

Rationale: the growth comes from expected Agent/Skill UI feature chunks, not accidental eager import of a large dependency. Vite output confirms the new UI assets are split into separate lazy chunks.

## Debt

D-M5-W5-BUILD: revisit bundle budget before M5 final acceptance. If W6/W7 add more UI/runtime assets, prefer chunk splitting or feature-level lazy loading before raising the threshold again.
