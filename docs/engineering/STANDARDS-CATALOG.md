# BrowserOS Standards Catalog

| Standard | Role |
| --- | --- |
| `AGENTS.md` | Agent operating and validation rules |
| `PROJECT-RULES.md` | locked/current product and native behavior |
| `docs/architecture/README-STANDARD.md` | module front-door contract |
| `scripts/GATE-CONTRACT.md` | M0 gate invocation/evidence contract |
| `CAPABILITY_GRANULARITY_RULES.md` | Capability sizing/boundaries |
| `docs/operations/github-primary-sync.md` | GitHub source-of-truth and mirror rules |
| `docs/engineering/README.md` | engineering-platform front door |

## Precedence

1. Explicit current owner decision.
2. `PROJECT-RULES.md` locked/current rules.
3. Architecture machine registries.
4. `AGENTS.md` operating rules.
5. Engineering governance registry.
6. Historical phase documents/logs.

Historical documents explain history but do not silently override current machine truth or locked decisions.

## Anti-duplication

Human docs should link to machine truth instead of copying volatile counts or ownership lists. Audit counts must be labelled with date/commit context.
