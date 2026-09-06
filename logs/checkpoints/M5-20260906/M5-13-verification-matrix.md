# M5 验证矩阵（A1 横切 · M5-W0 末位 + W1 reconciliation）

> 子卡 ID：**M5-13** · 跨 M5-1~M5-12 · `[S3|LEVERAGE:2|COMPLEX|AI:DEEP|R:xhigh]`
> 责任 Lane 候选：**A11**（沿用 A7/A11 角色，A0 签发时定）
> 父卡：`详细设计与实施计划.md` 整体（验证门禁横切）
> 配套：每张 M5-x 子卡 §6 COMMANDS / §7 PASS_CRITERIA / §8 FAIL_ACTION

---

## [W1 patched · 2026-09-06 08:50 CST] 策略脚本后缀一致性（.sh → .py）

> **修订来源**：A0 M5-W1 dispatch（`logs/checkpoints/A0-M5-W1-dispatch-20260906-0835.md`）A1 行 reconcile + A2 v3 prework（`logs/assist/A2-M5-core-20260906-0749.md` §13.3 C-8 "用 `.py` 不用 `.sh`"）。
> **修订原则**：A1 W1 接受 A2 C-8 建议（仓库 `scripts/` 30+ 门禁脚本惯例 + `pre-merge.sh` 的 `python3 "$SCRIPT_DIR/x.py" --self-test` 挂法 + A2 §3.3 fixtures 复用）。修订**仅**策略脚本后缀行内修正，**不重写**矩阵结构。

| 修订点 | W0 现状 | W1 修订 |
|---|---|---|
| `check-core-boundary` 后缀 | `.sh` | **`.py`** |

---

## 0. 编号与锚定

- 批次任务号 `M5-13`；与各 M5-x 子卡**横切**（不是顺次子卡，而是"验证矩阵入口"）
- 范围：跨 12 张子卡的反向用例 + 性能基线 + 升级回滚 + 集成冒烟
- 验证期：A2/A3/A4/A5/A6/A7/A8/A9 在 A11 横切检查下做单测/集成/e2e

---

## 1. GOAL

建立 M5 全包的验证矩阵，确保 12 张子卡的反向用例、性能基线、升级回滚、集成冒烟、退出收口五类验证齐备；任何一项红线失守即阻断合入。

---

## 2. READ

每张 M5-x 子卡的 §6 / §7 / §8（含反向用例 N1~Nxx）

---

## 3. WRITE（验证资产清单，**不写实现**）

| 文件 | 性质 | 说明 |
|---|---|---|
| `scripts/check-mcp-policy.py` | **新增**（M5-2） | MCP 政策自检；挂 `pre-merge.sh` |
| `scripts/check-a2a-policy.py` | **新增**（M5-3） | A2A 政策自检；挂 `pre-merge.sh` |
| `scripts/check-agent-kv-policy.py` | **新增**（M5-3） | agent_kv 政策自检；挂 `pre-merge.sh` |
| `scripts/check-core-boundary.py` | **新增**（M5-1） | core 边界；自检 PASS（`--self-test` 2好+2坏+1阴/默认扫描/`--expect-pending` 三模式） |
| `scripts/check-graph-policy.py` | **新增**（M5-7/8） | 图谱政策自检；挂 `pre-merge.sh` |
| `tests/m5_integration_smoke.rs` | **新增** | M5 集成冒烟（与 A4 现有 `tests/` 同款） |
| `tests/m5_reverse_cases.rs` | **新增** | M5 12 子卡反向用例集中（每张子卡 N1~Nxx） |
| `tests/m5_perf_baseline.rs` | **新增** | 性能基线（cargo bench / 集成单测） |
| `tests/m5_upgrade_rollback.rs` | **新增** | schema_version 迁移 + 插件回滚 |
| `src/__tests__/m5-ui.spec.ts` | **新增** | M5 UI 三件套（Agent/Graph/Plugin）e2e |
| `tests/m5_shutdown_order.rs` | **新增** | ShutdownCoordinator 索引依赖断言（沿用 `O-A1-5` 提醒） |
| `tests/m5_acl_terminal.rs` | **新增** | ACL 末条恒为 `list_artifact_images`（静态断言 + 增量插入测试） |

---

## 4. 验证矩阵（按维度）

### 4.1 协议契约测试

| 测试 | 对应子卡 | 反向用例 | 期望 |
|---|---|---|---|
| `MCP JSON-RPC 解析` | M5-2 | malformed JSON | -32700 |
| `MCP 未知 method` | M5-2 | `foo.bar` | -32601 |
| `MCP 未知 capability` | M5-2 | 调白名单外能力 | -32002（不泄露存在性） |
| `MCP 错误码不泄露能力存在性` | M5-2 | 未授权方 | -32001（不是 -32601） |
| `A2A 幂等` | M5-3 | 同 idempotency_key 二次 | 返回首次结果 |
| `A2A 状态机` | M5-3 | Submitted→Working→Completed 合规 | OK |
| `A2A 取消 in-flight` | M5-3 | cancel in Working | 终止，无半成品 |
| `agent_kv 容量` | M5-3 | 超过 5 MB / 5000 条 | LRU 淘汰 |
| `agent_kv 脱敏` | M5-3 | 写 `api_token: "xxx"` | 拒绝（K3） |
| `Skill K6` | M5-4 | SkillDef.exec=InlineScript | 解析拒 |
| `Skill ACL 三态` | M5-4/5 | Safe/Confirm/Dangerous | 闸门行为正确 |
| `Plugin manifest 校验` | M5-10 | 缺 hash | 拒绝 |
| `Plugin 签名` | M5-10 | signature 失败 | 拒绝 + 审计 |
| `Plugin 形态` | M5-10 | 形态② webview 用 `window.__TAURI__` | 阻断 |

### 4.2 隔离与权限测试

| 测试 | 对应子卡 | 反向用例 | 期望 |
|---|---|---|---|
| `MCP read_only` | M5-2 | 调写类能力 | -32003 READ_ONLY_MODE |
| `MCP 危险 SQL 拒` | M5-2 | 写 SQL + `allow_dangerous_sql=false` | 拒绝 |
| `MCP 连接白名单` | M5-2 | 调 db.query 未在 allowed_connection_ids | 拒绝 |
| `Skill 越权` | M5-4 | manifest 未声明的 capability | 阻断 |
| `Plugin 越权` | M5-11 | plugin_invoke 未声明的 capability | CAPABILITY_NOT_ALLOWED |
| `Plugin 跨 workspace` | M5-11 | 跨插件读 workspace | 阻断 |
| `Plugin 用户主 workspace` | M5-11 | 读用户主 workspace 无 `workspace.read` capability | 阻断 |
| `Plugin 资源路径 ..` | M5-11 | 资源路径含 `..` | 阻断 |
| `Graph props 不外泄` | M5-8/9 | graph_query 返回 props 正文 | 阻断（K7） |
| `Graph source=Manual 不被覆盖` | M5-7/8 | upsert source=Manual 节点 | 阻断 |

### 4.3 集成测试

| 测试 | 对应子卡 | 场景 | 期望 |
|---|---|---|---|
| `GraphEvent 上游写 → 抽取 → 查询` | M5-7/8 | 触发 GraphEvent | 自动抽取 + 节点/边入库 + graph_query 可查 |
| `MCP 工具 → db.query` | M5-2 | 通过 MCP 调 db.query | 走 A4 DB API + 校验 |
| `A2A 委派 → Agent.chat` | M5-3/4 | A2aTask 委派给 Agent | Agent 接收 + 走 chat 流式 |
| `Plugin 加载 → 注册 capability` | M5-10/11 | 装一个 Safe 插件 | capabilities 注册到 `capability.rs` |
| `Plugin 卸载 → 撤销 capability` | M5-10/11 | 卸载插件 | capabilities 撤销 |
| `Graph RAG 注入` | M5-4/9 | ChatPanel `/graph <q>` | graph_query 注入 system_prompt |

### 4.4 恢复测试

| 测试 | 对应子卡 | 场景 | 期望 |
|---|---|---|---|
| `graph.db 损坏` | M5-8 | graph.db 损坏 | 自动从 `graph.db.bak` 恢复 + 提示 |
| `agent_kv 容量满` | M5-3 | 超过 5 MB | LRU 淘汰 + 提示 |
| `mcp-calls.json 滚动` | M5-2 | 超过 500 | FIFO 裁剪 |
| `plugin-invokes.json 滚动` | M5-11 | 超过 500 | FIFO 裁剪 |
| `schema_version 迁移` | M5-8 | graph_schema 表新增版本 | 旧数据迁移 + 版本升级断言 |
| `插件回滚` | M5-10/11 | 回滚到上一版本 | storage.json 保留 + manifest 恢复 |

### 4.5 性能基线

| 测试 | 对应子卡 | 场景 | 期望 |
|---|---|---|---|
| `MCP 工具响应` | M5-2 | 100 次工具调用 p99 | < 100ms（除 db.query 大查询） |
| `图查询 depth=2` | M5-8 | 1000 节点 / depth=2 | < 500ms |
| `图查询 depth=2` | M5-8 | 10000 节点 / depth=2 | < 2s |
| `抽取性能` | M5-7 | 1000 文件 / 100 MB workspace | < 60s |
| `插件安装` | M5-10 | 10 插件安装 | < 5s |
| `插件列表` | M5-10 | 100 插件列表 | < 100ms |
| `流式回传节流` | M5-4 | 1s 10000 chunk | 事件数 ≤ 20 |

### 4.6 升级/回滚

| 测试 | 对应子卡 | 场景 | 期望 |
|---|---|---|---|
| `graph schema_version 升级` | M5-8 | v1 → v2 | 数据迁移 + 版本断言 |
| `plugin 升级` | M5-10/11 | v1 → v2 | storage 保留 + manifest_hash 变更触发确认 |
| `plugin 回滚` | M5-10/11 | v2 → v1 | 恢复上一版本 + capabilities 重新校验 |
| `agent_kv schema 升级` | M5-3 | v1 → v2 | 兼容性迁移 |

### 4.7 退出收口

| 测试 | 对应子卡 | 场景 | 期望 |
|---|---|---|---|
| `ShutdownCoordinator 索引序` | M5-1/2/3/4/7/8/10 | 注册序断言 | 严格按 A7 冻结序：worker-shutdown → stop-background-workers → stop-scheduler → graph-store-shutdown → mcp-server-shutdown → a2a-shutdown |
| `plugin 在飞 invoke` | M5-11 | 退出时 invoke 在飞 | 自动 cancel + 写 audit |
| `agent chat 在飞` | M5-4/5 | 退出时 chat 在飞 | 取消 + 清理 |

### 4.8 ACL 静态

| 测试 | 范围 | 场景 | 期望 |
|---|---|---|---|
| `ACL 末条恒为 list_artifact_images` | 全部 M5-x | 增量插入测试 | 末条恒定 |
| `capability.rs 单点` | M5-2/4/10 | 多文件定义测试 | 仅一份 |

---

## 5. PASS_CRITERIA

| # | 判据 | 验证 |
|---|---|---|
| 1 | 7 个 check-*.py / sh 脚本全部就位 + 挂 `pre-merge.sh` | `git ls-files scripts/check-*.{py,sh}` 命中 |
| 2 | `cargo test` 全绿（含 m5_* 集成） | `cargo test` |
| 3 | `tests/m5_reverse_cases.rs` 全反向用例 PASS | `cargo test m5_reverse` |
| 4 | `tests/m5_perf_baseline.rs` 全基线达标 | `cargo bench` / `cargo test m5_perf --release` |
| 5 | `tests/m5_upgrade_rollback.rs` 全迁移 PASS | `cargo test m5_upgrade` |
| 6 | `tests/m5_shutdown_order.rs` 索引序断言 PASS | `cargo test m5_shutdown` |
| 7 | `tests/m5_acl_terminal.rs` 末条恒定 PASS | `cargo test m5_acl` |
| 8 | `src/__tests__/m5-ui.spec.ts` 全 e2e PASS | `pnpm test:e2e` |
| 9 | `pre-merge.sh` ALL_PASS | `bash scripts/pre-merge.sh` |

---

## 6. FAIL_ACTION

| 失败 | 动作 |
|---|---|
| 任何 check-*.py/sh 不全检出合成坏样本 | 修脚本（不修实现避过）；脚本必须真起作用 |
| 反向用例漏检 | 阻断：补用例到 `tests/m5_reverse_cases.rs` |
| 性能基线不达标 | 阻断：调整实现或基线（需 A0 批准） |
| ShutdownCoordinator 序错 | 阻断（A7 冻结） |
| ACL 末条漂移 | 阻断（K1） |
| `capability.rs` 多文件定义 | 阻断：必须收口 |

---

## 7. DOC_BACKWRITE

1. `详细设计与实施计划.md` §7 验证矩阵段落更新
2. `后续需求TODO.md` §X 验证矩阵段落（待 A0 编号）
3. `AI-模型切换与接手清单.md` §X 验证矩阵段落（待 A0 编号）
4. `logs/checkpoints/M5-13.a-2026MMDD-HHMM.md`（实施卡 checkpoint）

---

## 8. COMMIT / NEXT

- **COMMIT**：A0 拣入后由 A11 实施填
- **NEXT**：M5-14（债务账），A1 本批 M5-W0 末位

---

## 9. FORBID 遵守记录

- 本卡为 A1 M5-W0 文档展开，**未写任何产品代码**
- 未触 `src/`、`src-tauri/`、`package.json`、三份主文档、ACL/Capability
- 未移动 `NEXT`（仍 M5-W0）
- 未提交、未 push
