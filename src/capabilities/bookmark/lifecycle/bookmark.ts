// Bookmark 生命周期钩子（Phase 8B）
// 设计：activate 时注册 UI 贡献；suspend 时不释放——Bookmark 无原生资源，
// 持久化状态保留（disable 不销毁用户数据）。
import { registerBookmarkContributions } from "../index"

export function onActivate(): void {
  registerBookmarkContributions()
}

export function onSuspend(): void {
  // 无原生资源可释放；用户数据保留（resources.destroyable=false）
}
