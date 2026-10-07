import { databaseManifest } from "./manifest"
import { createLazyContributionRegistrar } from "../../capability/platform/contributed"
import { activateDatabaseLifecycle, suspendDatabaseLifecycle } from "./lifecycle"

export const DATABASE_CAPABILITY_ID = "database"

const registerDatabase = createLazyContributionRegistrar(databaseManifest, [
  { id: "database.main.panel", type: "surface", slot: "workbench-main", view: "db", label: "数据库", icon: "🗄️", panelState: true, load: () => import("./ui/DatabasePanel.vue") },
])
export function registerDatabaseContributions(): void { registerDatabase() }

export const databaseCapability = {
  ...databaseManifest,
  lifecycle: {
    ...databaseManifest.lifecycle,
    onActivate: () => {
      activateDatabaseLifecycle()
      registerDatabaseContributions()
    },
    onSuspend: suspendDatabaseLifecycle,
    onDeactivate: suspendDatabaseLifecycle,
  },
}
