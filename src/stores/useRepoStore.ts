import { defineStore } from "pinia";
import { ref, reactive } from "vue";
import { bridge } from "../bridge";
import { useLayoutStore } from "./useLayoutStore";
import { useArtifactStore } from "./useArtifactStore";

// ============================================================
// Phase 8C-0C — Repo（仓库配置 + 同步）域 owner
//
// 从 useWorkspaceStore 抽出：repos / form / preview / busy / job + 同步动作。
// 后端持久化（listRepos / configureRepo / requestSync / confirmSync）；token 走 keyring（credential owner）。
// preview 由组件经 clearPreview()（canonical writer）关闭，禁止直写。
// Git 版本操作不在此域（GitPanel 走 bridge / useGitStore）——REPO context ≠ GIT operation。
// ============================================================

export const useRepoStore = defineStore("repo", () => {
  const layout = useLayoutStore();

  const repos = ref<RepoConfig[]>([]);
  const form = reactive({
    name: "",
    provider: "git" as RepoProvider,
    remote_url: "",
    branch: "main",
    username: "",
    token: "",
  });
  const preview = ref<SyncPreview | null>(null);
  const busy = ref(false);
  const job = ref<SyncJob | null>(null);

  async function loadRepos() {
    repos.value = await bridge.listRepos();
  }

  async function saveRepo() {
    if (!form.name || !form.remote_url || !form.token) {
      layout.showToast("请填写 名称 / 远程地址 / Token");
      return;
    }
    const config: RepoConfig = {
      id: Math.random().toString(36).slice(2),
      provider: form.provider,
      name: form.name,
      remote_url: form.remote_url,
      branch: form.branch || "main",
      username: form.username || "oauth2",
    };
    await bridge.configureRepo({ config, token: form.token });
    form.name = "";
    form.remote_url = "";
    form.branch = "main";
    form.username = "";
    form.token = "";
    await loadRepos();
    layout.showToast("仓库已保存（token 仅存密钥库）");
  }
  async function requestSync(repoId: string) {
    // 跨域读：同步消费 Artifact 多选（selection 真源归 useArtifactStore）
    const ids = [...useArtifactStore().selected];
    if (!ids.length) {
      layout.showToast("请先勾选要同步的成果");
      return;
    }
    preview.value = await bridge.requestSync({ artifactIds: ids, repoId });
    layout.showToast("已生成待确认同步任务，请确认后推送");
  }
  async function confirmSync() {
    if (!preview.value || busy.value) return;
    busy.value = true;
    job.value = await bridge.confirmSync({ jobId: preview.value.job_id });
    layout.showToast("推送中…");
  }
  // canonical writer：组件关闭同步预览弹窗（禁止直写 preview；WS-OWNER 守护）
  function clearPreview() {
    preview.value = null;
  }
  function onSyncCompleted(j: SyncJob) {
    busy.value = false;
    job.value = j;
    preview.value = null;
    loadRepos();
    layout.showToast(j.status === "success" ? "✅ 推送成功" : `❌ 推送失败: ${j.error ?? ""}`);
  }
  function loadGiteeExample() {
    form.provider = "gitee";
    form.name = "my-gitee-repo";
    form.remote_url = "https://gitee.com/你的用户名/你的仓库.git";
    form.branch = "master";
    form.username = "你的用户名";
    form.token = "";
    layout.showToast("已填入码云示例，补全用户名与 Token 后点保存");
  }

  return {
    repos,
    form,
    preview,
    busy,
    job,
    loadRepos,
    saveRepo,
    requestSync,
    confirmSync,
    clearPreview,
    onSyncCompleted,
    loadGiteeExample,
  };
});
