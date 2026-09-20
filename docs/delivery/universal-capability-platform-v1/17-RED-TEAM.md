# 17 — Independent Red Team（逐条诚实作答，不做辩解）

| # | 攻击问题 | 裁决 | 依据 |
|---|---|---|---|
| 1 | 只是把目录改名成 capabilities？ | **PASS** | 不是改名：运行时 register/unregister 真实生效（贡献 3→0→3），缺席能力零注册零贡献 |
| 2 | 只是 manifest 文档？ | **PASS** | manifest 被 `validateManifestV1` 机器校验，并驱动 resolve/assembly/hot-plug 的真实决策 |
| 3 | 只是隐藏 UI？ | **PASS** | `DISABLE/UNREGISTER` 真实从 registry 摘除 contribution 并移除 runtime 记录 |
| 4 | Browser absent 时 WebView 还在？ | **PASS（结构级）/ UNKNOWN（进程级）** | 结构：无 host contribution → 无 BrowserHost 挂载 → 无 WebView 创建入口；进程级今晚无实例，不估算 |
| 5 | Terminal absent 时 PTY/进程还在？ | **PASS（结构级）/ UNKNOWN（进程级）** | 同上，PLT2-12b |
| 6 | disable 后后台行为还在？ | **部分** | Bookmark 无后台行为，因此不受影响；其它能力尚未支持 disable（HP0），业务动作级拦截未完成（D-8，已登记） |
| 7 | hot plug 是否其实要求重启？ | **否（对 bookmark）** | 演示在同一进程内完成 register→disable→enable→unregister |
| 8 | register/unregister 只是布尔值？ | **否** | runtime 记录真实增删 + contribution registry 真实增删，且可重复插拔 |
| 9 | Shell 是否还知道具体能力？ | **部分 ⚠️** | 4 个积木已零 import；但 Shell 仍硬编码 Dock 页签名（文件/终端/资源/会话）→ D-10；且仍直连 10 个未积木化面板 → D-1 |
| 10 | 能力是否直接 import 别人 internal store？ | **PASS** | 既有边界门禁 fail=0；散落的 `utils/*Ui.ts`、`useBrowserHost` 记为物理边界债（D-3/D-4，非 state truth 复制） |
| 11 | Runtime 是否成为 God Object？ | **PASS** | runtime 仅持 id/state/enabled 元数据；platform 模块为纯函数，不含业务状态 |
| 12 | Registry 是否成为 Service Locator？ | **PASS** | 只存 contribution 元数据；无能力通过它取别人实现或状态 |
| 13 | Assembly Engine 是否只是 profile 切换？ | **否** | 依赖展开 + 拓扑排序 + 冲突/环/缺失拒绝，且真实驱动 bootstrap |
| 14 | 依赖缺失是否 deterministic reject？ | **PASS** | PLT2-04/05/06/07 负例全通过 |
| 15 | remove 强依赖是否正确拒绝？ | **PASS** | PLT2-15（DEPENDENT_PRESENT，force 才绕过） |
| 16 | 有没有第二 state truth？ | **PASS（0）** | PLT2-17：catalog 与 definition 同一对象；语义门禁 DEV-02c fail=0 |
| 17 | 物理模块是否真有 build/dependency boundary？ | **PARTIAL ⚠️** | 单 Vite 应用，边界由约定+checker 保证，**不是真正的构建边界** → 07 号文档诚实裁决 DEFERRED |
| 18 | 是否把 component 错当 Capability？ | **PASS** | workspace 的 7 个 main view 归属同一 capability（同 owner 体系），未拆成独立 capability |
| 19 | 是否高报 C3/C4/C5？ | **未发现** | 无人声称 C4/C5；browser/terminal 明确保持 HP0 |
| 20 | 是否高报 HP1/HP2/HP3？ | **未发现** | 仅 bookmark HP1+HP2（有证据）；HP3 明确未做 |
| 21 | 是否把 DECLARED 写 MEASURED？ | **未发现** | 资源判定一律标注 DECLARED/UNKNOWN，CLI 明确输出该口径 |
| 22 | 是否为了 Demo 修改真实用户数据？ | **否** | 仅改仓库代码与配置；未碰用户数据/凭据/系统安装 |
| 23 | 是否为了 PASS 放宽 checker？ | **否** | 未删除任何 assertion，未加 blanket allow-list；历史 RED（pre-merge）保持原样未掩盖 |
| 24 | 普通 AI 是否真可局部修改？ | **部分** | 4 个积木可（改动面见 AI_CHANGE_SURFACE.md）；其余约 10 个面板因未积木化仍需跨模块理解 |

## 红队结论

- **无 HARD STOP**（未触发 §49 中 H1–H10 任何一条）。
- 主要真实缺口：**Shell 仍知道具体能力（D-1/D-10）** 与 **物理无构建边界（D-9）**。
  二者均已登记为债务并在 `16-NEXT-STEPS.md` 给出计划，**未被描述成已完成**。

## 需-human 裁决的点

无。若要求「Shell 必须零能力知识」才算通过 GUI 验收，则当前应判 **PARTIAL** 而非 PASS。
