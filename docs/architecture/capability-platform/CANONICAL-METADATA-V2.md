# Capability Metadata Single Source — V2 Stage 1

## Canonical source

The canonical source for capability metadata is each capability's `manifest.ts` Building Block Contract.
The generated registry discovers those manifests and exposes `GENERATED_MANIFESTS` plus
`GENERATED_CANONICAL_METADATA`. Runtime catalog, dependency assembly, Capability Manager and gates
must consume that generated/canonical path rather than create a second handwritten runtime table.

Canonical low-risk fields in Stage 1:

- id
- displayName
- maturity
- dependencies
- optionalDependencies
- semanticOwner
- entrypoint

## Compatibility projections

`docs/architecture/capability-registry/capabilities.yaml`,
`dependencies.yaml`, semantic owner documentation and maturity audit documents remain compatibility
projections during V2. They are not deleted in Stage 1. Existing drift gates remain blocking while fields
are migrated away from manual duplication.

## Data flow

`manifest.ts -> generate-capability-registry.mjs -> generated-registry.ts -> CAPABILITY_CATALOG /
CANONICAL_CAPABILITY_METADATA -> Assembly / Runtime / Manager / Gates`

Documentation and maturity reports may project from the canonical view, but must not become runtime truth.

## Drift policy

- generated registry stale -> FAIL
- manifest/registry dependency or entrypoint drift -> FAIL
- agent/graph runtime cycle -> FAIL
- future migrated fields must be added to the drift checker before their handwritten projection is retired

This stage intentionally does not rewrite the whole registry system and does not delete existing YAML.
