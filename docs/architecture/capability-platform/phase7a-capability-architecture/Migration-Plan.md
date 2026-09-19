# Migration Plan（迁移计划）

> Phase 7A　基线：`semantic-governance-v1`
> 原则：**兼容优先、小步、每步可回滚**。不追求一晚拆完。

---

## 1. 波次规划

| Wave | Phase | 目标 | 产物 | 风险 | 状态 |
|---|---|---|---|---|---|
| W0 | **7A** | 能力审计与架构设计 | 本文档集（Inventory/Dependency/Lifecycle/Resource/Shell/Migration） | 无（纯文档） | ✅ 本阶段 |
| W1 | **7B** | Capability Contract + Registry | `docs/architecture/capability-registry/*.yaml` + 最小 SDK 接口定义 | 低（新增，不改既有） | 待做 |
| W2 | **7C** | Minimal Capability Runtime | `src/capability/runtime.ts`（register/resolve/activate/suspend/inspect） | 中（新增运行时） | 待做 |
| W3 | **7D** | Pilot = Bookmark | Bookmark manifest + adapter，证明模型可用 | 低（试点） | 待做 |
| W4 | **7E** | Resource Governance v1 | resources.yaml + Capability Resource Report | 低 | 待做 |
| W5 | 7F | Preview 验收 + 领导材料 | `docs/delivery/capability-preview-v1/` | 低 | 待做 |

---

## 2. 明确推迟（不在今晚，需单独 ADR）

| 能力 | 推迟理由 |
|---|---|
| Browser / Grid | 用户已认可的**稳定高价值基线**（VERY_HEAVY + MULTI_WEBVIEW）；改动风险最高，必须先有 Runtime 稳定运行证据 |
| Terminal | PTY / 进程 / replay 缓冲；且与 Clipboard 共享 `useSystemStore`，需先拆分 Owner（属语义变更，需 SCR） |
| Plugin | runtime 仍 LOCKED（既有冻结决定），今晚只登记不装配 |
| Agent / Skill | Skill 无登记 Owner；Agent 涉及 NETWORK，需安全边界 ADR |

> 以上均登记为 `TARGET_COMPOSABLE`，**不得写成已实现**。

---

## 3. 逐步装配路径（目标态，非今晚）

```text
Step 1 (今晚 W3):  Bookmark 经 Adapter 接入 Runtime
                   → 证明 manifest/register/resolve/activate 真实可用
Step 2 (未来):     Notes / Vault / Skill / Snippet（LIGHT 类）接入
Step 3 (未来):     Git / Database / KnowledgeGraph / ResourceCollection / Workbench（MEDIUM）
Step 4 (未来):     Task / Script / Session（BACKGROUND / PROCESS）
Step 5 (未来):     Terminal（需先拆 useSystemStore 的 Terminal/Clipboard Owner）
Step 6 (未来):     Browser / Grid（需 Runtime 成熟度证据 + 回归门禁）
Step 7 (未来):     Plugin（需 runtime 解冻 ADR）
```

每一步都必须：
```text
Design → Independent Review → Implementation → Checker → Build/Test → Closeout → Commit → Tag
```

---

## 4. 回滚点

| 回滚点 | 用途 |
|---|---|
| `semantic-governance-v1` (`316130d`) | 治理冻结基线，任何能力改造失败都回到此 |
| `capability-phase7a-architecture-pass` | 7A 审计基线（纯文档） |
| 各 Phase tag（7B/7C/7D/7E） | 分阶段回滚 |
| `.snapshots/2026-09-19-overnight-baseline.txt` | 开工前状态快照 |

回滚方式（**禁止 blind reset**）：
```bash
bash scripts/git-recover.sh        # 走既有护栏
bash scripts/snapshot.sh --list    # 对比快照
```

---

## 5. 禁止事项（贯穿全程）

```text
禁止一次移动全部 store 目录
禁止一次拆 monorepo / 十几个 npm package
禁止删除旧系统
禁止降低 Checker / 删除失败测试 / 新增大面积 allow-list
禁止 force push / 移动旧 tag
禁止隐藏 Known Debt / 编造 Runtime PASS / 把未验证写 PASS
禁止 sudo apt install / dpkg -i（不覆盖用户当前安装）
```
