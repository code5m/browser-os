package com.jizhijiandan.mvp.bridge

import android.webkit.JavascriptInterface
import android.webkit.WebView
import org.json.JSONObject
import java.security.MessageDigest
import java.time.Instant
import java.util.UUID

/**
 * 原生桥（对应桌面 bridge.rs）。
 *
 * 最小 IPC 面：只有标注 @JavascriptInterface 的方法对 WebView 可见，
 * 且必须只对「受信任的浏览子 WebView」调用：
 *
 *   browserWebView.addJavascriptInterface(NativeBridge(...), "AndroidBridge")
 *
 * 主控制台 WebView 不注入本对象（或注入受限子集），避免远程页面越权。
 */
class NativeBridge(
    private val creds: KeystoreCredentialStore,
    private val sync: SyncManager,
    private val store: MutableLocalStore,
    private val onUi: (String, Any?) -> Unit, // 回调到 UI 线程（如同步完成事件）
) : BridgeCommands {

    // —— 采集：网页选区 → 本地成果（带溯源） —— //
    @JavascriptInterface
    fun collectSelection(payloadJson: String): String {
        val p = JSONObject(payloadJson)
        val art = collectSelection(
            url = p.getString("url"),
            title = p.optString("title"),
            text = p.optString("text"),
            html = p.optString("html"),
        )
        return JSONObject()
            .put("id", art.id).put("title", art.title).put("hash", art.hash)
            .toString()
    }

    override fun collectSelection(
        url: String, title: String, text: String, html: String,
    ): Artifact {
        val hash = sha256(text + url)
        val art = Artifact(
            id = UUID.randomUUID().toString(),
            title = title, sourceUrl = url, text = text, html = html,
            hash = hash, createdAt = Instant.now().toString(),
        )
        store.saveArtifact(art)
        audit("collect", "$title <- $url")
        return art
    }

    override fun openBrowser(url: String) {
        onUi("open_browser", url) // 由 Activity 打开浏览子 WebView 并注入 collect_android.js
        audit("open_browser", url)
    }

    override fun listArtifacts(): List<Artifact> = store.loadArtifacts()

    // token 仅落 Keystore，绝不回传前端
    override fun configureRepo(config: RepoConfig, token: String) {
        require(token.isNotBlank()) { "token 不能为空" }
        creds.saveToken(config.id, token)
        store.saveRepo(config)
        audit("configure_repo", "${config.name} (${config.provider})")
    }

    override fun listRepos(): List<RepoConfig> = store.loadRepos()

    override fun requestSync(artifactIds: List<String>, repoId: String): SyncPreview =
        sync.requestSync(artifactIds, repoId)

    // 闸门：确认后后台推送，完成通过 onUi("sync-completed", job) 通知前端
    override fun confirmSync(jobId: String): SyncJob {
        sync.confirmSync(jobId) { job -> onUi("sync-completed", job) }
        // 立即返回 RUNNING（真正结果走事件），与桌面语义一致
        return SyncJob(
            id = jobId, repoId = "", status = SyncStatus.RUNNING,
            artifactIds = emptyList(), createdAt = Instant.now().toString(),
        )
    }

    override fun auditLog(): List<AuditEntry> = store.loadAudit()

    private fun audit(action: String, detail: String) = store.appendAudit(
        AuditEntry(Instant.now().toString(), action, detail)
    )

    private fun sha256(s: String): String =
        MessageDigest.getInstance("SHA-256").digest(s.toByteArray())
            .joinToString("") { "%02x".format(it) }

    companion object {
        /** 仅对受信任的浏览子 WebView 注入桥，收紧 IPC 面。 */
        fun attachToBrowserWebView(web: WebView, bridge: NativeBridge) {
            web.settings.javaScriptEnabled = true
            web.addJavascriptInterface(bridge, "AndroidBridge")
        }
    }
}

/** 可写持久化契约（NativeBridge 用），实现见项目内 Room/文件 JSON。 */
interface MutableLocalStore : LocalStore {
    fun saveArtifact(art: Artifact)
    fun saveRepo(config: RepoConfig)
    fun appendAudit(entry: AuditEntry)
    fun loadAudit(): List<AuditEntry>
}
