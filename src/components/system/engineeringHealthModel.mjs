// Read-only and testable model. It must never infer PASS from a previous commit.
export const WORKFLOWS = Object.freeze([
  { id: "engineering-governance", name: "BrowserOS Engineering Governance", label: "工程治理", explanation: "目录与门禁注册表一致性" },
  { id: "ui-safety", name: "BrowserOS UI Safety", label: "界面安全", explanation: "UI 边界与安全策略" },
  { id: "hot-plug", name: "BrowserOS Hot-Plug Acceptance", label: "模块热插拔", explanation: "Capability 生命周期与隔离" },
  { id: "supply-chain", name: "BrowserOS Supply Chain Assurance", label: "依赖供应链", explanation: "依赖审计和 SBOM；历史豁免不等于无漏洞" },
  { id: "full-validation", name: "BrowserOS Full Validation", label: "完整工程验收", explanation: "构建、Rust、安装包 GUI、pre-merge 等" },
]);
export const MAX_EVIDENCE_AGE_MS = 48 * 60 * 60 * 1000;
const order = ["FAIL", "RUNNING", "STALE", "UNKNOWN", "PASS"];
export function evaluateWorkflow(name, sha, runs, now = Date.now()) {
  if (!/^[a-f0-9]{40}$/i.test(sha || "")) return { state: "UNKNOWN", reason: "无法确认当前 master SHA", run: null };
  const candidates = (Array.isArray(runs) ? runs : []).filter(
    r => r && r.name === name && r.head_sha === sha && r.event === "push",
  );
  candidates.sort((a, b) => Date.parse(b.created_at || 0) - Date.parse(a.created_at || 0));
  const run = candidates[0] || null;
  if (!run) return { state: "UNKNOWN", reason: "当前提交没有这条工作流的运行记录", run: null };
  if (run.status !== "completed") return { state: "RUNNING", reason: "正在执行，不等于已通过", run };
  if (run.conclusion !== "success") return { state: "FAIL", reason: "最近运行：" + (run.conclusion || "结果未知"), run };
  const finishedAt = Date.parse(run.updated_at || run.created_at || "");
  if (!Number.isFinite(finishedAt) || now - finishedAt > MAX_EVIDENCE_AGE_MS) {
    return { state: "STALE", reason: "上次通过已超过 48 小时，需重新验证", run };
  }
  return { state: "PASS", reason: "当前 master 提交已验证通过", run };
}
export function evaluateAll(sha, runs, now = Date.now()) {
  const checks = WORKFLOWS.map(w => ({ ...w, ...evaluateWorkflow(w.name, sha, runs, now) }));
  const overall = order.find(state => checks.some(check => check.state === state)) || "UNKNOWN";
  return { checks, overall };
}
