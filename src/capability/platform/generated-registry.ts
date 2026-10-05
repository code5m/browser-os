// GENERATED FILE. Run npm run generate:capability-registry.
import type { CapabilityManifestV1 } from './contract'
import { demoManifest as manifest0 } from './../../../packages/capability-demo/src/manifest'
import { agentManifest as manifest1 } from './../../capabilities/agent/manifest'
import { appsManifest as manifest2 } from './../../capabilities/apps/manifest'
import { bookmarkManifest as manifest3 } from './../../capabilities/bookmark/manifest'
import { browserManifest as manifest4 } from './../../capabilities/browser/manifest'
import { databaseManifest as manifest5 } from './../../capabilities/database/manifest'
import { gitManifest as manifest6 } from './../../capabilities/git/manifest'
import { graphManifest as manifest7 } from './../../capabilities/graph/manifest'
import { homeManifest as manifest8 } from './../../capabilities/home/manifest'
import { pluginManifest as manifest9 } from './../../capabilities/plugin/manifest'
import { skillManifest as manifest10 } from './../../capabilities/skill/manifest'
import { taskManifest as manifest11 } from './../../capabilities/task/manifest'
import { terminalManifest as manifest12 } from './../../capabilities/terminal/manifest'
import { toolsManifest as manifest13 } from './../../capabilities/tools/manifest'
import { workspaceManifest as manifest14 } from './../../capabilities/workspace/manifest'
import { settingsManifest as manifest15 } from './../../settings/manifest'
import { clipboardManifest as manifest16 } from '@browser-os/capability-clipboard/manifest'
import { vaultManifest as manifest17 } from '@browser-os/capability-vault/manifest'
import { demoCapability as definition0 } from './../../../packages/capability-demo/src/index'
import { agentCapability as definition1 } from './../../capabilities/agent/index'
import { appsCapability as definition2 } from './../../capabilities/apps/index'
import { bookmarkCapability as definition3 } from './../../capabilities/bookmark/index'
import { browserCapability as definition4 } from './../../capabilities/browser/index'
import { databaseCapability as definition5 } from './../../capabilities/database/index'
import { gitCapability as definition6 } from './../../capabilities/git/index'
import { graphCapability as definition7 } from './../../capabilities/graph/index'
import { homeCapability as definition8 } from './../../capabilities/home/index'
import { pluginCapability as definition9 } from './../../capabilities/plugin/index'
import { skillCapability as definition10 } from './../../capabilities/skill/index'
import { taskCapability as definition11 } from './../../capabilities/task/index'
import { terminalCapability as definition12 } from './../../capabilities/terminal/index'
import { toolsCapability as definition13 } from './../../capabilities/tools/index'
import { workspaceCapability as definition14 } from './../../capabilities/workspace/index'
import { settingsCapability as definition15 } from './../../settings/index'

export const GENERATED_MANIFESTS: CapabilityManifestV1[] = [manifest0.v1 ?? manifest0, manifest1.v1 ?? manifest1, manifest2.v1 ?? manifest2, manifest3.v1 ?? manifest3, manifest4.v1 ?? manifest4, manifest5.v1 ?? manifest5, manifest6.v1 ?? manifest6, manifest7.v1 ?? manifest7, manifest8.v1 ?? manifest8, manifest9.v1 ?? manifest9, manifest10.v1 ?? manifest10, manifest11.v1 ?? manifest11, manifest12.v1 ?? manifest12, manifest13.v1 ?? manifest13, manifest14.v1 ?? manifest14, manifest15.v1 ?? manifest15, manifest16.v1 ?? manifest16, manifest17.v1 ?? manifest17]
export const GENERATED_MANIFEST_SOURCES = ["./../../../packages/capability-demo/src/manifest","./../../capabilities/agent/manifest","./../../capabilities/apps/manifest","./../../capabilities/bookmark/manifest","./../../capabilities/browser/manifest","./../../capabilities/database/manifest","./../../capabilities/git/manifest","./../../capabilities/graph/manifest","./../../capabilities/home/manifest","./../../capabilities/plugin/manifest","./../../capabilities/skill/manifest","./../../capabilities/task/manifest","./../../capabilities/terminal/manifest","./../../capabilities/tools/manifest","./../../capabilities/workspace/manifest","./../../settings/manifest","@browser-os/capability-clipboard/manifest","@browser-os/capability-vault/manifest"] as const
export const GENERATED_DEFINITIONS = { 'demo': definition0, 'agent': definition1, 'apps': definition2, 'bookmark': definition3, 'browser': definition4, 'database': definition5, 'git': definition6, 'graph': definition7, 'home': definition8, 'plugin': definition9, 'skill': definition10, 'task': definition11, 'terminal': definition12, 'tools': definition13, 'workspace': definition14, 'settings': definition15 } as const

export const GENERATED_CANONICAL_METADATA = Object.fromEntries(
  GENERATED_MANIFESTS.map((manifest) => [manifest.id, {
    id: manifest.id,
    displayName: manifest.displayName,
    maturity: manifest.maturity,
    dependencies: [...manifest.dependencies],
    optionalDependencies: [...manifest.optionalDependencies],
    semanticOwner: manifest.semanticOwner,
    entrypoint: manifest.entrypoint,
  }]),
) as Record<string, Pick<CapabilityManifestV1, 'id' | 'displayName' | 'maturity' | 'dependencies' | 'optionalDependencies' | 'semanticOwner' | 'entrypoint'>>
