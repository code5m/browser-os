# PR35 build-only dependency decision

The task authorizes fixing the known dependency-policy drift without changing the product footprint baseline or limits. The existing CSS optimizer is accepted only as `devDependencies.lightningcss = 1.33.0`, exactly matching the current manifest and lockfile. It is already used by the Vite production CSS minifier, not by runtime capability code.

The three image-preview/script/command checkers now include this exact declaration in their frozen maps. They still compare all four dependency collections and the lock root. Unknown dependencies, removal, version changes and moves to runtime dependencies remain rejected. No dependency, application feature, asset, audit, threshold or read-only/production policy is removed.

`scripts/check-pr35-dependency-freeze.py` additionally checks the locked version and integrity entry, build-only ownership, absence from runtime imports, and rejection of range broadening, removal, runtime moves and unrelated additions across all three checker families. Their existing positive/negative self-tests remain enabled.

Local integration evidence: all three live policy checks and self-tests passed; complete `npm run check` passed. Supply Chain Assurance, actual final CI and final footprint validation remain mandatory and must be associated with the merged head. The frozen footprint is still 795517 bytes plus at most 25.2 percent growth.
