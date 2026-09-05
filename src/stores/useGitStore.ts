import { defineStore } from "pinia";
import { computed, reactive, ref } from "vue";
import { bridge } from "../bridge";
import { useLayoutStore } from "./useLayoutStore";
import { redactSecrets } from "../utils/redact";
import type {
  GitBranch,
  GitDiffResult,
  GitFileStatus,
  GitWriteJob,
  GitWriteOp,
  GitWritePreview,
  GitWriteRisk,
} from "../types";

// ---------------------------------------------------------------------------
// M1-7 Git UI 状态层：消费 M1-5 只读能力（git_status / git_diff / git_branch_list）
// 与 M1-6 写能力（request_git_write / confirm_git_write / git-write-completed）。
//
// 硬约束（fail-closed，任何一环不满足就不发请求）：
// - 写操作**只能**经 requestGitWrite → confirmGitWrite 两步，本文件没有任何直写路径；
// - dangerous（discard / push）必须前端勾选二次确认后才发 confirm，
//   未勾选时连 confirm 命令都不会发出（后端闸门之外的又一道本地锁）；
// - commit message / 分支名 / 路径列表在前端先做非空校验，校验失败**不发 request**
//   （避免无意义的后端往返与审计噪声）；
// - 前端不持有、不持久化任何 token / 凭据；所有错误展示前经 redactSecrets 脱敏；
// - 不存在任何自动触发的写操作：request/confirm 只能由用户点击触发，无 watch 自动调用。
// ---------------------------------------------------------------------------

/** dangerous 操作集合（与后端 `GitWriteOp::is_dangerous` 冻结口径一致）。 */
export const GIT_DANGEROUS_OPS: readonly GitWriteOp[] = ["discard", "push"];

export const GIT_OP_LABEL: Record<GitWriteOp, string> = {
  stage: "暂存",
  unstage: "取消暂存",
  discard: "丢弃改动",
  commit: "提交",
  create_branch: "新建分支",
  checkout_branch: "切换分支",
  push: "推送",
};

/** push 的 UI 约束提示（只讲约束，绝不暴露凭据）。 */
export const PUSH_NOTICE =
  "仅推送当前检出的本地分支到 origin 同名分支，非 force；" +
  "凭据只在后端推送瞬间从系统密钥库读取，前端不接触、不存储任何凭据。";

/** discard 的 UI 约束提示。 */
export const DISCARD_NOTICE = "丢弃会把工作区文件还原到 HEAD 状态，改动不可恢复。";

/** 预览里最多展示的路径条数（后端 GIT_WRITE_PREVIEW_MAX_PATHS = 20，前端对齐）。 */
export const GIT_PREVIEW_SHOWN_PATHS = 20;

export function gitWriteRisk(op: GitWriteOp, dangerous: boolean): GitWriteRisk {
  if (dangerous) return "high";
  if (op === "checkout_branch" || op === "create_branch") return "medium";
  return "low";
}

export const GIT_RISK_LABEL: Record<GitWriteRisk, string> = {
  low: "低风险（仅本地索引/工作区）",
  medium: "中风险（会改变当前检出分支）",
  high: "高风险（不可逆或影响远端）",
};

export const useGitStore = defineStore("git", () => {
  const layout = useLayoutStore();

  // ===== 只读状态（M1-5） =====
  const repoId = ref<string | null>(null);
  const status = ref<GitFileStatus[]>([]);
  const branches = ref<GitBranch[]>([]);
  const diff = ref<GitDiffResult | null>(null);
  const activePath = ref<string | null>(null);

  const statusLoading = ref(false);
  const branchLoading = ref(false);
  const diffLoading = ref(false);
  const statusError = ref<string | null>(null);
  const branchError = ref<string | null>(null);
  const diffError = ref<string | null>(null);

  // ===== 交互态（纯 UI，不含任何凭据） =====
  const selected = reactive<Set<string>>(new Set());
  const commitMessage = ref("");
  const branchName = ref("");
  const createCheckout = ref(true);

  // ===== 写闸门（M1-6） =====
  const preview = ref<GitWritePreview | null>(null);
  const dangerousAck = ref(false);
  const busy = ref(false);
  const lastJob = ref<GitWriteJob | null>(null);
  const writeError = ref<string | null>(null);

  const currentBranch = computed(
    () => branches.value.find((b) => b.is_head)?.name ?? null
  );
  const localBranches = computed(() =>
    branches.value.filter((b) => !b.is_remote).map((b) => b.name)
  );
  const remoteBranches = computed(() => branches.value.filter((b) => b.is_remote));
  const selectedPaths = computed(() => [...selected]);
  const hasStatus = computed(() => status.value.length > 0);
  const isDangerous = computed(
    () => preview.value?.dangerous ?? false
  );
  const previewRisk = computed<GitWriteRisk | null>(() =>
    preview.value ? gitWriteRisk(preview.value.op, preview.value.dangerous) : null
  );
  const canConfirm = computed(() => !preview.value?.dangerous || dangerousAck.value);

  function toast(text: string) {
    layout.showToast(text);
  }

  // ===== 只读加载 =====
  async function loadStatus() {
    if (!repoId.value) return;
    statusLoading.value = true;
    statusError.value = null;
    try {
      const list = await bridge.gitStatus({ repoId: repoId.value });
      status.value = list;
      // 已消失的路径不应继续留在勾选集里（写操作会拿它去后端校验）
      const alive = new Set(list.map((f) => f.path));
      for (const p of [...selected]) if (!alive.has(p)) selected.delete(p);
    } catch (e) {
      status.value = [];
      statusError.value = redactSecrets(e);
    } finally {
      statusLoading.value = false;
    }
  }

  async function loadBranches() {
    if (!repoId.value) return;
    branchLoading.value = true;
    branchError.value = null;
    try {
      branches.value = await bridge.gitBranchList({ repoId: repoId.value });
    } catch (e) {
      branches.value = [];
      branchError.value = redactSecrets(e);
    } finally {
      branchLoading.value = false;
    }
  }

  /** path 为 null/undefined 表示拉取整仓 diff（受后端硬上限截断保护）。 */
  async function loadDiff(path?: string | null) {
    if (!repoId.value) return;
    if (path !== undefined) activePath.value = path;
    diffLoading.value = true;
    diffError.value = null;
    try {
      diff.value = await bridge.gitDiff({
        repoId: repoId.value,
        path: activePath.value ?? undefined,
      });
    } catch (e) {
      diff.value = null;
      diffError.value = redactSecrets(e);
    } finally {
      diffLoading.value = false;
    }
  }

  async function refreshAll() {
    if (!repoId.value) return;
    await Promise.all([loadStatus(), loadBranches()]);
    await loadDiff();
  }

  async function selectRepo(id: string | null) {
    // 切仓库等于放弃上一次待确认任务（任务本身在后端 5 分钟后自然过期）
    cancelWrite();
    repoId.value = id;
    selected.clear();
    activePath.value = null;
    diff.value = null;
    commitMessage.value = "";
    branchName.value = "";
    lastJob.value = null;
    writeError.value = null;
    if (!id) {
      status.value = [];
      branches.value = [];
      return;
    }
    await refreshAll();
  }

  function toggleSelect(path: string) {
    if (selected.has(path)) selected.delete(path);
    else selected.add(path);
  }
  function selectAll() {
    for (const f of status.value) selected.add(f.path);
  }
  function clearSelection() {
    selected.clear();
  }

  // ===== 写操作：阶段一 request =====
  /**
   * 前端预校验：任何失败都直接拦下，**连 request 都不发**。
   * 只做「必然失败」的判定（空值/空串），格式与业务校验仍以后端为准（fail-closed）。
   */
  function precheck(
    op: GitWriteOp,
    opts: { paths?: string[]; message?: string; branch?: string }
  ): string | null {
    switch (op) {
      case "stage":
      case "unstage":
      case "discard":
        if (!opts.paths || opts.paths.length === 0) return "请先勾选要操作的文件";
        return null;
      case "commit":
        if (!opts.message || opts.message.trim() === "") return "提交信息不能为空";
        return null;
      case "create_branch":
      case "checkout_branch":
        if (!opts.branch || opts.branch.trim() === "") return "分支名不能为空";
        return null;
      case "push":
        return null;
      default:
        return "不支持的操作";
    }
  }

  async function requestWrite(
    op: GitWriteOp,
    opts: { paths?: string[]; message?: string; branch?: string; checkout?: boolean } = {}
  ) {
    if (!repoId.value) {
      toast("请先选择一个仓库");
      return;
    }
    if (busy.value) {
      toast("上一个 Git 写操作仍在执行中");
      return;
    }
    if (preview.value) {
      toast("已有一个待确认的 Git 写任务，请先确认或取消");
      return;
    }
    const invalid = precheck(op, opts);
    if (invalid) {
      // 不发请求：避免无效后端往返与审计噪声
      writeError.value = invalid;
      toast(invalid);
      return;
    }
    writeError.value = null;
    dangerousAck.value = false;
    try {
      preview.value = await bridge.requestGitWrite({
        repoId: repoId.value,
        op,
        paths: opts.paths,
        message: opts.message,
        branch: opts.branch,
        checkout: opts.checkout,
      });
    } catch (e) {
      preview.value = null;
      writeError.value = redactSecrets(e);
      toast(`${GIT_OP_LABEL[op]}被拒绝: ${writeError.value}`);
    }
  }

  // ===== 写操作：阶段二 confirm =====
  async function confirmWrite() {
    const pv = preview.value;
    if (!pv) return;
    if (busy.value) return;
    // 本地又一道锁：dangerous 未二次确认时，confirm 命令根本不会发出
    if (pv.dangerous && !dangerousAck.value) {
      writeError.value = "请先勾选二次确认";
      toast(`${GIT_OP_LABEL[pv.op]}需二次确认后才能执行`);
      return;
    }
    writeError.value = null;
    busy.value = true;
    try {
      lastJob.value = await bridge.confirmGitWrite({
        jobId: pv.job_id,
        confirmedDangerous: pv.dangerous ? true : false,
      });
      preview.value = null;
      dangerousAck.value = false;
      toast(`${GIT_OP_LABEL[pv.op]}已提交执行…`);
    } catch (e) {
      busy.value = false;
      writeError.value = redactSecrets(e);
      toast(`${GIT_OP_LABEL[pv.op]}执行失败: ${writeError.value}`);
    }
  }

  function cancelWrite() {
    preview.value = null;
    dangerousAck.value = false;
  }

  /** `git-write-completed` 事件回调：刷新 status/diff/branch。 */
  function onWriteCompleted(job: GitWriteJob) {
    if (!job) return;
    // 只处理当前选中仓库的任务（其它仓库的任务与本面板无关）
    if (repoId.value && job.repo_id !== repoId.value) return;
    busy.value = false;
    lastJob.value = job;
    if (job.status === "failed") {
      writeError.value = redactSecrets(job.error ?? "写操作失败");
      toast(`❌ ${GIT_OP_LABEL[job.op]}失败: ${writeError.value}`);
    } else if (job.status === "success") {
      writeError.value = null;
      toast(`✅ ${GIT_OP_LABEL[job.op]}完成`);
    }
    if (job.op === "commit") commitMessage.value = "";
    if (job.op === "create_branch") branchName.value = "";
    void refreshAll();
  }

  // ===== 便捷入口（只负责组装参数，闸门语义完全一致） =====
  const stage = (paths = selectedPaths.value) => requestWrite("stage", { paths });
  const unstage = (paths = selectedPaths.value) => requestWrite("unstage", { paths });
  const discard = (paths = selectedPaths.value) => requestWrite("discard", { paths });
  const commit = (message = commitMessage.value, paths = selectedPaths.value) =>
    requestWrite("commit", { message, paths });
  const createBranch = (branch = branchName.value, checkout = createCheckout.value) =>
    requestWrite("create_branch", { branch, checkout });
  const checkoutBranch = (branch: string) => requestWrite("checkout_branch", { branch });
  const push = () => requestWrite("push");

  return {
    repoId,
    status,
    branches,
    diff,
    activePath,
    statusLoading,
    branchLoading,
    diffLoading,
    statusError,
    branchError,
    diffError,
    selected,
    commitMessage,
    branchName,
    createCheckout,
    preview,
    dangerousAck,
    busy,
    lastJob,
    writeError,
    currentBranch,
    localBranches,
    remoteBranches,
    selectedPaths,
    hasStatus,
    isDangerous,
    previewRisk,
    canConfirm,
    loadStatus,
    loadBranches,
    loadDiff,
    refreshAll,
    selectRepo,
    toggleSelect,
    selectAll,
    clearSelection,
    requestWrite,
    confirmWrite,
    cancelWrite,
    onWriteCompleted,
    stage,
    unstage,
    discard,
    commit,
    createBranch,
    checkoutBranch,
    push,
  };
});
