package com.jizhijiandan.mvp.bridge

import org.eclipse.jgit.api.Git
import org.eclipse.jgit.api.MergeResult
import org.eclipse.jgit.transport.UsernamePasswordCredentialsProvider
import java.io.File
import java.time.Instant
import java.util.UUID
import java.util.concurrent.ConcurrentHashMap

/**
 * 确认闸门 + JGit 推送（对应桌面 sync.rs + bridge.rs 的 request/confirm）。
 *
 * 红线体现：
 * - requestSync 只生成 PENDING 任务，绝不出网；
 * - confirmSync 才在后台线程真正 push，凭据此刻才从 Keystore 读取；
 * - 三方合并：fast-forward / merge / 冲突则中止保护数据（不丢任何一方改动）。
 */
class SyncManager(
    private val reposDir: File,
    private val creds: KeystoreCredentialStore,
    private val store: LocalStore,               // 成果/仓库/审计持久化（示意，见下方接口）
    private val audit: (String, String) -> Unit, // 审计回调
) {
    private val pending = ConcurrentHashMap<String, SyncJob>()

    /** 第一步：生成待确认任务（校验仓库与凭据存在） */
    fun requestSync(artifactIds: List<String>, repoId: String): SyncPreview {
        val repo = store.loadRepos().firstOrNull { it.id == repoId }
            ?: throw IllegalStateException("仓库未配置")
        creds.getToken(repo.id) // 提前校验凭据存在（不使用值）
        val arts = store.loadArtifacts().filter { it.id in artifactIds }
        require(arts.isNotEmpty()) { "没有可同步的成果" }

        val job = SyncJob(
            id = UUID.randomUUID().toString(),
            repoId = repo.id,
            status = SyncStatus.PENDING,
            artifactIds = arts.map { it.id },
            createdAt = Instant.now().toString(),
        )
        pending[job.id] = job
        audit("request_sync", "生成待确认任务 -> ${repo.name}")
        return SyncPreview(
            jobId = job.id, repoId = repo.id, repoName = repo.name,
            artifactCount = arts.size, artifactTitles = arts.map { it.title },
            remoteUrl = repo.remoteUrl,
        )
    }

    /**
     * 第二步（闸门）：用户确认后在后台线程真正推送。
     * onDone 回调把最终 SyncJob 回传 UI（对应桌面 sync-completed 事件）。
     */
    fun confirmSync(jobId: String, onDone: (SyncJob) -> Unit) {
        val job = pending[jobId] ?: throw IllegalStateException("未知任务")
        check(job.status == SyncStatus.PENDING) { "任务状态异常" }
        job.status = SyncStatus.RUNNING

        Thread {
            try {
                push(job)
                job.status = SyncStatus.SUCCESS
                audit("confirm_sync", "SUCCESS 任务 ${job.id}")
            } catch (e: Exception) {
                job.status = SyncStatus.FAILED
                job.error = e.message
                audit("confirm_sync", "FAILED 任务 ${job.id}: ${e.message}")
            } finally {
                job.finishedAt = Instant.now().toString()
                onDone(job)
            }
        }.start()
    }

    private fun push(job: SyncJob) {
        val repo = store.loadRepos().first { it.id == job.repoId }
        val token = creds.getToken(repo.id) // 凭据仅此刻读取
        val cp = UsernamePasswordCredentialsProvider(repo.username, token)
        val dir = File(reposDir, repo.id).apply { mkdirs() }

        val git = if (File(dir, ".git").exists()) Git.open(dir)
        else Git.cloneRepository().setURI(repo.remoteUrl).setDirectory(dir)
            .setCredentialsProvider(cp).call()

        // —— 三方合并 —— //
        git.fetch().setCredentialsProvider(cp).call()
        val merge = git.merge()
            .include(git.repository.resolve("origin/${repo.branch}"))
            .call()
        if (merge.mergeStatus == MergeResult.MergeStatus.CONFLICTING) {
            // 冲突：中止合并保护数据，交用户人工处理
            git.repository.writeMergeCommitMsg(null)
            git.repository.writeMergeHeads(null)
            throw IllegalStateException(
                "检测到与远端冲突，已中止自动合并以保护数据。冲突文件: " +
                    merge.conflicts?.keys?.joinToString() + "。请人工合并后重试。"
            )
        }

        // 写入成果（.md + 富文本 .html）
        val arts = store.loadArtifacts().filter { it.id in job.artifactIds }
        val artDir = File(dir, "artifacts").apply { mkdirs() }
        arts.forEach { a ->
            File(artDir, "${a.id}.md").writeText(renderMarkdown(a))
            if (a.html.isNotBlank()) File(artDir, "${a.id}.html").writeText(renderHtml(a))
        }

        git.add().addFilepattern(".").call()
        git.commit().setMessage("sync ${arts.size} artifact(s) via 极智简单").call()
        git.push().setCredentialsProvider(cp).call()
        git.close()
    }

    private fun renderMarkdown(a: Artifact) = buildString {
        appendLine("# ${a.title}\n")
        appendLine("- 来源: ${a.sourceUrl}")
        appendLine("- 溯源哈希: ${a.hash}")
        appendLine("- 采集时间: ${a.createdAt}\n---\n")
        append(a.text)
    }

    private fun renderHtml(a: Artifact) =
        "<!doctype html><meta charset=utf-8><title>${a.title}</title>" +
            "<div style=font-size:12px;color:#888>来源:${a.sourceUrl} · 哈希:${a.hash}</div>" +
            a.html
}

/** 本地持久化契约（对应桌面 workspace.rs），实现可用 Room / 文件 JSON。 */
interface LocalStore {
    fun loadArtifacts(): List<Artifact>
    fun loadRepos(): List<RepoConfig>
}
