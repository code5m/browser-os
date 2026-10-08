# Peripheral Engineering

Peripheral engineering is infrastructure outside normal feature code that can still decide whether BrowserOS is safe to develop, ship and recover.

## Source control safety

- GitHub `origin` is authoritative.
- `.githooks/pre-push` rejects direct non-origin pushes, deletions, non-fast-forward divergence and tag rewrites.
- `scripts/sync-repo.sh` is the safe multi-machine sync entry.
- Server protection is the final Git history boundary.

## Legacy Gitee mirror

- Automatic Gitee branch/tag mirroring is retired.
- `.github/workflows/mirror-to-gitee.yml` is retained as a manual-only historical placeholder so governance can track why mirroring is disabled.
- Gitee must not gate GitHub `master`, roll back GitHub history, or become a second writable truth.

## Diagnostics and recovery

- `npm run doctor`
- `scripts/collect-diagnostics.sh`
- `scripts/git-recover.sh`
- `docs/operations/recovery/`

Diagnostics report facts; they must not silently mutate user data to make gates green.

## Release engineering

Stable releases are explicit immutable references. Capability Platform v4 stays frozen at `capability-platform-v4-stable`; later engineering-only commits do not move it.

## Evidence lifecycle

The repository contains extensive historical evidence. Until a separate retention policy exists, evidence may be indexed/classified but not bulk-deleted.

## Repository topology

Do not split core capabilities into many repositories for cosmetic modularity. Extraction requires independent API, release cadence, compatibility promise and real reuse/ownership boundary.
