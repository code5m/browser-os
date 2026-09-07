# A4 · M5-W17 Checkpoint — 主页/客户端启动 隐私与安全静态审查

- **Lane**：A4（START REVIEW）
- **Wave**：M5-W17 Desktop Client Completeness and Home Recovery Dispatch
- **Base**：`052b18a`（origin/master，W15 PUSHED·accepted）
- **形态**：REVIEW + 新建策略夹具（`scripts/check-home-client-policy.py`）；**零产品 UI 编辑**
- **未 commit、未 push**（Only A0 pushes）

---

## 1. 交付物

| 文件 | 说明 |
|---|---|
| `scripts/check-home-client-policy.py` | 新建静态守门夹具：3 ACTIVE + 3 PENDING 码位，三模式 |
| `logs/assist/A4-M5-W17-home-client-privacy-review-20260908-1130.md` | 审查 note（结论 + 整改清单 + 风险） |
| `logs/checkpoints/A4-M5-W17-home-client-privacy-review-20260908-1130.md` | 本 checkpoint |
| `logs/checkpoints/Lane-A4-M5-W17-home-client-privacy-review-20260908-1130.patch` | binary patch |

---

## 2. 码位清单

### ACTIVE（W17 永久红线，默认门禁判，当前 PASS）

| 码位 | 守门 | 实测 |
|---|---|---|
| `HOME_NO_PRIVILEGE_EXPANSION` | 禁新增 Tauri 命令/ACL/`shell:`·`fs:`·`http:`·`network:` 权限/`Command::new`/`TcpListener` | 零命中 ✅ |
| `HOME_NO_SHELL_INJECTION` | 禁 `target`/`cmd` 未校验拼接进命令执行 | 零命中 ✅ |
| `HOME_STARTUP_SAFE` | 启动助手禁 sudo/chmod 777/curl\|bash/eval/无差别 pkill | 零命中 ✅（`run-gui.sh` 仅 build + env + exec） |

### PENDING（既有债务，`--expect-pending` 判，当前 FAIL = 整改清单）

| 码位 | 实测命中 |
|---|---|
| `HOME_NO_SECRET_PERSIST` | `src/stores/useHomeStore.ts:71`（app exec 命令体落 localStorage） |
| `HOME_NO_RAW_ERROR_ECHO` | `src/stores/useHomeStore.ts:180`（`e?.message ?? e` 原文回显） |
| `HOME_NO_SENSITIVE_TARGET_RENDER` | `src/components/home/HomePanel.vue:48`（`:title="s.target"`） |

---

## 3. 精确整改清单（给 A3/A5，工具已就绪，属接线）

| ID | 位置 | 整改 |
|---|---|---|
| F-1 | `useHomeStore.ts:71` | 落盘只存安全投影（用 `homeDisplayTarget()` 或仅存 `id/type/name/icon`），不落 app exec 命令体与 dir 绝对路径 |
| F-2 | `useHomeStore.ts:180` | 改用 `panelStateHome({error}).message` / `HOME_ERROR_FALLBACK`，丢弃 `e` |
| F-3 | `HomePanel.vue:48` | `:title="homeDisplayTarget(s)"`；无障碍名用 `homeAccessibleLabel(s)` |

整改后把 3 个 PENDING 码位转入 ACTIVE。

---

## 4. 前序 lane 审查结论

| Lane | 产物 | 结论 |
|---|---|---|
| A2 | `run-gui.sh` | 安全 PASS（无 sudo/eval/curl\|bash/无差别 kill）；**功能未满足 acceptance #1**——未启动/等待 Vite，而 `main.rs:1210` debug 强制 `localhost:1421`。建议补：探测 1421 → 未起则起 Vite 并记录 PID → `trap cleanup EXIT` 只 kill 自起 PID |
| A3 | `src/utils/homeUi.ts` | **正面**：`looksSensitive` / `homeDisplayTarget` / `homeAccessibleLabel` / `panelStateHome`（错误态不回显原文）+ 有界（24/12）+ 迁移安全。已达 W16 P2/P3/P6 与 W17 #4 |
| A3 | `useHomeStore.ts` | 引入 `homeUi` 真源（好），但本身**未接线**安全函数（F-1/F-2） |
| A5 | `src/components/home/HomeLaunchers.vue` | **0 byte 空文件**（board §5 规则 5 违规）→ 报 A5/A0 |
| A3/A9 | `check-home-store-logic.mjs` / `check-home-ui-logic.mjs` | 未落（本夹具 gated no-op） |

---

## 5. 验证命令与结果

```bash
python3 scripts/check-home-client-policy.py --self-test
# HOME_CLIENT_POLICY_SELF_TEST_RESULT=PASS: ...ACTIVE=3 PENDING=3

python3 scripts/check-home-client-policy.py
# home client policy: all invariants hold（ACTIVE=3）

python3 scripts/check-home-client-policy.py --expect-pending
# HOME_CLIENT_POLICY_PENDING_RESULT=FAIL
#   HOME_NO_SECRET_PERSIST:src/stores/useHomeStore.ts:71（app exec 命令体落浏览器存储）
#   HOME_NO_RAW_ERROR_ECHO:src/stores/useHomeStore.ts:180
#   HOME_NO_SENSITIVE_TARGET_RENDER:src/components/home/HomePanel.vue:48

python3 -m py_compile scripts/check-home-client-policy.py   # OK
```

夹具自检期间修正的自身缺陷（诚实记录）：
1. shell 注入初版"元字符+邻近词"窗口匹配误报好样本（TS 到处是 `;`）→ 改为只匹配拼接表达式。
2. 盘符正则 `[A-Za-z]:[\\/]` 误判 URL `https://` → 增加 `_scrub_schemes()` 先抹 scheme。

---

## 6. 风险残留

1. F-1..F-3 待 A3/A5 接线（默认门禁不阻塞）。
2. A2 acceptance #1 功能缺口需 A2 自补 + A8 实机确认；A4 未伪造验证。
3. 本夹具未接 `scripts/pre-merge.sh`（属 A9/A0 范围，避免跨 lane 冲突）；建议 A0/A11 集成时接入**默认模式**。
4. A5 `HomeLaunchers.vue` 0 byte 待处理。
5. A4 零产品代码改动 → build metrics 不适用；GUI 验收归 A8。

---

## 7. LANE 输出

```text
LANE: A4
STATUS: PASS_WITH_DEBT
SCOPE: scripts/check-home-client-policy.py (new), logs/assist/A4-M5-W17-*.md, logs/checkpoints/A4-M5-W17-*.md
DELIVERED: 主页/启动静态隐私安全守门夹具（3 ACTIVE + 3 PENDING，三模式）+ A2/A3/A5 产物审查 + F-1..F-3 整改清单
VERIFY: --self-test PASS; default PASS(ACTIVE=3); --expect-pending FAIL(3 hits); py_compile OK
METRICS: N/A（零产品代码改动）
PATCH: logs/checkpoints/Lane-A4-M5-W17-home-client-privacy-review-20260908-1130.patch
RISKS: F-1..F-3 待接线；A2 未启动/等待 Vite（debug 依赖 1421）；夹具未接 pre-merge；A5 空文件
NO_PUSH: confirmed
```
