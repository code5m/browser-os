// Vault 能力公共契约（Phase 8B — 唯一对外出口）
//
// Shell 与其它能力**仅**经由此文件访问 Vault 语义：
//   - 不暴露内部实现细节
//   - 不提供 service locator / getVaultStore()
//   - 复用 store 本身作为受控公共状态出口（与 bookmark/public.ts 同构）
export { useVaultStore } from "./state/useVaultStore"
