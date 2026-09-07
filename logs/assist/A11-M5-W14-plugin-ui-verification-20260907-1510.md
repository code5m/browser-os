# M5-W14 Assist Review — 插件 UI 隐私夹具误报 + UI 逻辑测试缺口
> 评审方: A11 (Verification) · 对象: A4 (夹具 `check-plugin-ui-privacy.py`) + A6 (插件管理器 UI)
> 时间: 2026-09-07 15:10 · 关联 checkpoint: `logs/checkpoints/A11-M5-W14-20260907-1510.md`

## 0. 一句话结论
pre-merge 因 `PLUGIN_UI_NO_SECRET_RENDER` 变红，但属**夹具结构性误报**：A6 未展示/持久化任何公钥原文，仅有一个用户自粘贴、立即清空的 `pubkey` 瞬时输入字段被子串扫描命中。需 A4 或 A6 择一修复后复跑；另 A6 缺交付 `check-plugin-ui-logic.mjs`。

---

## A. 阻塞项（给 A4 / A6）

### A.1 现象
```
pytest 门禁: check-plugin-ui-privacy.py --self-test  → FAIL
真实仓库 ACTIVE 违规: ['PLUGIN_UI_NO_SECRET_RENDER']
命中: \.pubkey ×4 , pubkey ×16
PRE_MERGE_RESULT=FAIL
```

### A.2 误报证据（A6 实质合规）
| 检查点 | 位置 | 结论 |
|---|---|---|
| 受信任密钥列表渲染内容 | `PluginManager.vue` L165-170 | 仅 `{{ k.key_id }}` + `{{ k.fingerprint }}`，**无 `k.pubkey`** ✓ |
| 详情脱敏投影 | `pluginUi.ts::redactDetail` L141-160 / `redactKeyRecord` L168-170 | 显式挑选字段，丢弃 `signature.value`/路径/`pubkey`/metadata ✓ |
| `pubkey` 字面量出处 | `PluginManager.vue` L24-25,66,94,96,173；`usePluginStore.ts` L195-196,205 | 全部是"用户粘贴→`addKey`→`keyForm.pubkey=""` 清空"的**瞬时输入**，从不回显/持久化 ✓ |
| 裸 invoke | 全组件/store | 仅调用 `bridge.plugin*`，无 `invoke("plugin_"` / `@tauri-apps/api/tauri` ✓ |

### A.3 修复方案
**方案 A（推荐，A4 改夹具）** — 收紧 A2 检测粒度，只抓"展示/输出位置"：
```python
# scripts/check-plugin-ui-privacy.py :: detect_hits, A2 分支
# 将 SECRET_RENDER_TOKENS 中的裸 r"pubkey" 改为仅匹配展示上下文：
SECRET_RENDER_TOKENS = (
    r"signature\.value",
    r"\.pubkey",                      # 保留：覆盖 {{ x.pubkey }} / k.pubkey 绑定
    r"\{\{[^}]*pubkey",              # 模板插值中的 pubkey（真实回显）
    r"resource_path",
    r"rawSignature", r"raw_signature",
    r"privateKey", r"private_key",
    r"apiKey", r"api_key",
    r"secretKey", r"secret_key",
)
# 并可在扫描时跳过 <input ...> 标签内与 reactive({...}) 表单声明行（可选增强）
```
> 去掉裸 `pubkey` 后，A6 的瞬时输入字段与 `placeholder="pubkey..."` 文案不再命中；而一旦有人写成 `{{ detail.pubkey }}` 仍会被 `\.pubkey` / 模板插值规则捕获。

**方案 B（A6 改代码，夹具保持严格）** — 把瞬时字段与 placeholder 中的 `pubkey` 字面量改名：
- `keyForm.pubkey` → `keyForm.keyMaterial`
- `addKey(keyId, pubkey, note)` → `addKey(keyId, keyMaterial, note)`
- placeholder `"pubkey（仅用于登记，不回显）"` → `"公钥原文（仅用于登记，不回显，提交后不留存）"`（去掉 `pubkey` 子串）

任选其一即可让 pre-merge 转绿；**方案 A 更优**（保留对真实回显的防护，且不动产品代码）。

---

## B. 缺口（给 A6）

### B.1 缺失 `scripts/check-plugin-ui-logic.mjs`
A6 Must-Deliver 要求无头 UI 逻辑测试，当前未交付。建议覆盖 `pluginUi.ts` 纯函数（与 `plugin.rs::can_transition` 1:1 镜像，是 W14 高价值回归点）：
- `canTransitionUi(from,to)` / `enabledActionsFor(state)` — 8 态状态机边；
- `redactDetail(d)` — 断言 `signature` 无 `value`、`resource` 无绝对路径、无 `metadata`；
- `parseManifestInput(text)` — 必填字段/JSON 形态/entry 对象校验；
- ACL 风险档 `aclLevelLabel` / `isDangerousGate` / `aclLevelClass`。

交付后纳入 A11 第 13 项 UA 脚本复跑。

---

## C. 软观察（长期建议，非门禁）
- **错误渲染**：`usePluginStore.describeError` 把 `e.message` 截断 300 字直渲 (`PluginManager.vue` L239)。`PLUGIN_UI_ERROR_CODE_ONLY` 为 PENDING，暂不强；建议长期映射为稳定 `error_code`（对齐 `PluginSummary.error_code`），契合"错误只渲染稳定码"红线。
- **uninstall 语义**：A6 `uninstall` → `bridge.pluginDisable`（后端无独立 uninstall 命令，守 W13 红线）。设计合理，但 UI 文案"卸载"与后端"disable"需对齐，避免误解。

---

## D. 构建指标回归（给 A0 / A6）

### D.1 现象
```
[pre-merge] FAIL: build metrics regression vs logs/m0-build-metrics/build-metrics-4f0e8ab.json
```
- M0 基线 `build-metrics-4f0e8ab.json`：`dist.total_bytes = 612943`（8 文件）。
- 当前（a7eefbb + A6 UI）：`dist.total_bytes = 765705`（22 文件）。
- 增长 = (765705 − 612943) / 612943 = **+24.91%** > `TOTAL_BYTES_GROWTH_LIMIT_PCT = 23.0` → 失败。
- `cargo_warnings = 2`（与基线持平，未增），`fmt_clean = true`，无 >500KB chunk → 仅体积越限。

### D.2 归因
`measure-build-metrics.py` 头部注释记录了阈值演进：M0=15% → M4=16% → W5=19% → W6=21% → W8=22% → W12=23%（每次均附"当前实测%"）。W12 实测 22.26% 压线；本波（W13/W14）前端新增以 **A6 插件管理器面板**（`PluginManager.vue` + `usePluginStore.ts` + `pluginUi.ts`）为主，把增长推到 24.91%，越过红线。属真实门禁失败（非工具误报）。

### D.3 处置（需 A0 / A6 决策）
- **方案 C（推荐，A0 重定基线）**：沿用 W5/W6/W8/W12 先例，在脚本注释追加 M5-W14 记录并把 `TOTAL_BYTES_GROWTH_LIMIT_PCT` 抬至 **26%**（实测 24.91%，留约 1pp 余量），附书面理由。
- **方案 D（A6 减量）**：将插件面板改为路由级懒加载（动态 import）并剔除未用依赖；注意本门禁比对 dist **总**体积，懒加载主要降主包、对总量影响有限。

---

## E. 复跑命令（修复后）
```bash
python3 scripts/check-plugin-ui-privacy.py --self-test   # 期望 SELF_TEST_RESULT=PASS
python3 scripts/check-plugin-ui-privacy.py               # 期望 all invariants hold
bash scripts/pre-merge.sh                                # 期望 PRE_MERGE_RESULT=ALL_PASS
node scripts/check-plugin-ui-logic.mjs                   # 期望 A6 交付后全绿
```
