import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  Artifact,
  RepoConfig,
  SyncPreview,
  SyncJob,
  AuditEntry,
} from "./types";

// 类型化 IPC 封装：前端永远只传“意图”，不直接碰 OS / 凭据
export const bridge = {
  openBrowser: (url: string) => invoke("open_browser", { url }),

  collectSelection: (p: {
    url: string;
    title: string;
    html: string;
    text: string;
  }) => invoke<Artifact>("collect_selection", p),

  listArtifacts: () => invoke<Artifact[]>("list_artifacts"),

  // token 仅在此调用中传给后端，存入系统密钥库；不会被前端持久化/回显
  configureRepo: (p: { config: RepoConfig; token: string }) =>
    invoke("configure_repo", p),

  listRepos: () => invoke<RepoConfig[]>("list_repos"),

  // 第一步：生成“待确认”SyncJob（不真正推送）
  requestSync: (p: { artifactIds: string[]; repoId: string }) =>
    invoke<SyncPreview>("request_sync", p),

  // 第二步：用户确认后才真正推送（闸门）。后台线程执行，立即返回 Running 任务
  confirmSync: (p: { jobId: string }) => invoke<SyncJob>("confirm_sync", p),

  // 订阅后台推送完成事件（成功/失败都会触发，payload 为最终 SyncJob）
  onSyncCompleted: (cb: (job: SyncJob) => void) =>
    listen<SyncJob>("sync-completed", (e) => cb(e.payload)),

  auditLog: () => invoke<AuditEntry[]>("audit_log"),
};
