# Evidence Lifecycle

BrowserOS already carries valuable historical evidence. This policy does not delete it.

Machine policy: `docs/engineering/evidence-policy.json`.
Gate: `npm run check:evidence-lifecycle`.

The existing tracked evidence is protected by a 35 MB history budget, above the audited ~27 MB state. Old assist/checkpoint/research directories are frozen for new files; new transient evidence goes to GitHub Actions artifacts or local ignored diagnostics. Release/baseline evidence remains durable.

This separates four concerns that were previously mixed: permanent baseline proof, release proof, short-lived CI proof, and local diagnostics. Raising the history budget requires an explicit policy change and review; it is not an automatic escape hatch.
