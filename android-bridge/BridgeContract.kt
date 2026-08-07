package com.jizhijiandan.mvp.bridge

/**
 * 与前端约定的桥契约（对应桌面 domain.rs + bridge.ts）。
 * 命令名、字段名与桌面端保持一致，前端代码可跨端复用。
 */

// —— 数据结构 —— //

/** 本地成果：带溯源（来源 URL + 内容哈希） */
data class Artifact(
    val id: String,
    val title: String,
    val sourceUrl: String,
    val text: String,
    val html: String,      // 富文本保真副本
    val hash: String,      // sha256(text + sourceUrl)
    val createdAt: String, // ISO-8601
    val tags: List<String> = emptyList(),
)

enum class RepoProvider { GIT, GITEE }

/** 仓库配置（注意：不含 token，凭据隔离） */
data class RepoConfig(
    val id: String,
    val provider: RepoProvider,
    val name: String,
    val remoteUrl: String,
    val branch: String,
    val username: String,
)

enum class SyncStatus { PENDING, RUNNING, SUCCESS, FAILED }

/** 同步任务：推送前必须处于 PENDING 并经确认 */
data class SyncJob(
    val id: String,
    val repoId: String,
    var status: SyncStatus,
    val artifactIds: List<String>,
    val createdAt: String,
    var finishedAt: String? = null,
    var error: String? = null,
)

/** 确认闸门预览 */
data class SyncPreview(
    val jobId: String,
    val repoId: String,
    val repoName: String,
    val artifactCount: Int,
    val artifactTitles: List<String>,
    val remoteUrl: String,
)

data class AuditEntry(val at: String, val action: String, val detail: String)

/**
 * 命令白名单（对应 bridge.rs 的 8 个 #[tauri::command]）。
 * 原生实现见 NativeBridge.kt；每个方法都应写审计。
 */
interface BridgeCommands {
    fun openBrowser(url: String)
    fun collectSelection(url: String, title: String, text: String, html: String): Artifact
    fun listArtifacts(): List<Artifact>
    fun configureRepo(config: RepoConfig, token: String) // token 仅落 Keystore
    fun listRepos(): List<RepoConfig>
    fun requestSync(artifactIds: List<String>, repoId: String): SyncPreview
    fun confirmSync(jobId: String): SyncJob // 闸门：确认后才真正 push
    fun auditLog(): List<AuditEntry>
}
