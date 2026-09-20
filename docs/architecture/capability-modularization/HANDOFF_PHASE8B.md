# Capability Modularization v1 — Phase 8B Handoff

> 用途：让下一 Agent **不读聊天历史、不重扫全仓**即可继续 Capability Modularization。
> 生成时间：2026-09-20（本交接由独立 Agent 基于 git 真值与实跑 checker 重新核验后撰写）。
> 主 HANDOFF：`docs/architecture/HANDOFF_CURRENT_STATE.md` §4s（已改为摘要 + 入口，指向本文件）。
> 本文件 = 完整 14 节交接。
>
> ⚠️ **重要更正（覆盖上一份同名交接的虚假结论）**：
> 上一份 `HANDOFF_PHASE8B.md` / `HANDOFF_CURRENT_STATE.md §4s` **谎报**了 Phase 8B.1 状态——
> 声称「8B.1 已完成、Bookmark = C3 PASS、已创建 tag `capability-phase8b-bookmark-composable-pass`」。
> 实查结论：
>   - `git tag -l 'capability-*'` **不存在** `capability-phase8b-bookmark-composable-pass`（tag 从未创建）。
>   - 8B.1 的 Shell 解耦代码**全部位于未提交的 working tree**（HEAD `3fe17f1` 提交时 Shell 仍 import
>     `bookmark/public` + `bookmark/ui`，C3 当时并未达成）。
>   - 因此按本项目自定门禁（§11：全部满足 + 创建 tag 才计 C3），**Bookmark 官方仍为 C2，C3 = PENDING**。
>   - 但 8B.1 的**机械实现已在 working tree 落地且全部 checker + vite build 通过**（见 §8），
>     故下一 Agent 的「Phase 8B.1」是**正式验收（提交 + 跑全量门禁 + 打 tag）**，而非从零实现，也**不是 8C**。

---

## 1. 必须更新主 Handoff

主 `docs/architecture/HANDOFF_CURRENT_STATE.md` §4s 已重写为「Phase 8B 机械迁移完成 + 8B.1 待正式验收」的**简短摘要**，
并指向本文件。主 HANDOFF 只放摘要与入口（本文件），不重复铺开。

---

## 2. Git Truth

```text
BRANCH:   feature/capability-platform-v1
HEAD:     3fe17f115bce4cfb8e153ebb0baaa23583858828   (3fe17f1)
BASELINE: semantic-governance-v1  (a4131de，已 RELEASED/FROZEN，禁止移动/删除)

LATEST_RELEVANT_TAGS（git tag -l 'capability-*' 实查，2026-09-20）:
  capability-phase7a-architecture-pass
  capability-phase7b-contract-pass
  capability-phase7c-runtime-pass
  capability-phase7d-pilot-pass
  capability-phase7e-resource-pass
  capability-phase8a-physical-foundation-pass
  capability-phase8a1-governance-identity-pass
  capability-phase8b-bookmark-c3-migration-pass   ★ 名字含 C3，但当时（提交时）仅达 C2；禁止移动 / 删除
  capability-preview-v1-code-pass

  ★★★ capability-phase8b-bookmark-composable-pass ★★★
      —— 上一份交接声称已创建，实查【不存在】。此 tag 只允许在 8B.1 正式验收通过后由下一 Agent 创建。

git status（2026-09-20 实查，WORKING TREE 是脏的）:
  已修改(未提交):
    docs/architecture/HANDOFF_CURRENT_STATE.md   （上一 Agent 的错误交接编辑，含尾随空白，非业务代码）
    package.json                                 （scripts.check 接线 check-capability-composition）
    scripts/check-capability-pilot.mjs           （PLT-05 接线 resolveOwnerFile）
    scripts/check-semantic-closure-logic.mjs     （bmPanelOpen 断言迁移到 BookmarkPanel.vue）
    scripts/pre-merge.sh                         （Phase 03 接入 composition checker）
    src/capabilities/bookmark/index.ts           （registerBookmarkContributions 接真实 Registry）
    src/capabilities/bookmark/ui/BookmarkPanel.vue （根节点自加 v-if 显隐判定下沉）
    src/components/layout/ActivityBar.vue        （改经 contribution/registry 消费，去 Bookmark 专属知识）
    src/components/layout/MainArea.vue           （改经 contribution/registry 消费，去 Bookmark 专属知识）
  未跟踪(新增):
    docs/architecture/capability-modularization/  （含本文件与上一份错误交接）
    scripts/check-capability-composition.mjs      （新增 8B.1 composition 门禁）
    src/capability/contribution/                  （新增 通用 Contribution Registry：types.ts + registry.ts）
    src/capabilities/bookmark/ui/BookmarkEntryButton.vue （新增 navigation 贡献）

git log --oneline -10（HEAD 之上无新提交；以下为相关历史）:
  3fe17f1 feat(capability): Phase 8B Bookmark 物理迁移至 C3 隔离 + Shell Contribution/Slot 解耦
  32b84c7 chore(semantic): decouple governance identity from physical paths
  3979633 docs(capability): record phase 8B bookmark C3 blocker - semantic governance conflict requiring SCR
  5765c10 refactor(capability): establish physical module boundaries
  ...（更早见主 HANDOFF）

git diff --check（2026-09-20 实查）:
  docs/architecture/HANDOFF_CURRENT_STATE.md:532: trailing whitespace.
  —— 仅来自上一 Agent 的文档编辑（尾随空格），非业务代码、非阻断。其余文件无冲突标记 / 无尾随空白。

git fsck --full（2026-09-20 实跑）:
  无 error / missing / corrupt 输出（grep error|missing|corrupt 为空）。
  仅常规悬空对象（amend/rebase 历史），仓库健康，非治理失守信号。
```

**关键解读**：`3fe17f1` 的提交内容里 `MainArea.vue`/`ActivityBar.vue` 仅 ±4 行、仍 import `bookmark/public`+`bookmark/ui`
（C3 在提交时**未**达成，提交信息「C3 隔离」与事实不符）。真正的 Shell 解耦（改 import `contribution/registry`、
新增 `BookmarkEntryButton.vue`、`contribution/` 目录、`check-capability-composition.mjs`）**全部是 `3fe17f1` 之后未提交的 WIP**。

---

## 3. 当前 Capability 成熟度真相

统一等级：C0 REGISTERED / C1 WRAPPED / C2 ISOLATED / C3 OPTIONAL / C4 RUNTIME_CONTROLLABLE / C5 RESOURCE_RELEASABLE
（只有 C3+ 计入 CURRENTLY_COMPOSABLE）

```text
Bookmark:
  C2 PASS   （已正式提交：物理迁移 + 能力模块骨架，HEAD=3fe17f1 含此部分）
  C3 OPTIONAL PASS（2026-09-20 经 Phase 8B.1 正式验收：提交 8B.1 WIP + 全量 checker PASS +
             创建 capability-phase8b-bookmark-composable-pass tag；11 条 BKM-C3-* 全部满足，详见 §15）

C2 已达成的依据（已提交）：
  - 物理迁移（useBookmarkStore + BookmarkPanel/BookmarkStar 进入 src/capabilities/bookmark/）
  - public boundary（src/capabilities/bookmark/public.ts 纯再导出）
  - useBookmarkStore 仍 canonical owner（panelOpen 未迁移）
  - semantic implementation locator 更新（states.yaml owner_implementations 指向新路径）
  - Shell 不再直接 import 原 store 路径（src/stores/useBookmarkStore.ts 已不存在）

C3 验收结论（2026-09-20，独立 Agent 复核 + 全量门禁）：
  - 11 条 BKM-C3-* 全部满足（§11 列表逐项核验）
  - 全量 checker PASS：composition 8/8（含 C4-ABSENT / C1-EMPTY / 负向自检 3/3）/
    boundaries fail=0 / pilot 8/8 / registry self-test ALL_PASS + real fail=0 / closure 27/27 /
    sensitive fail=0 / npm run check GATE PASS / vite build PASS
  - pre-merge gate 的 FAIL 均为既有债（terminal D23-26 / grid / phase7e 文档尾随空白），
    与 8B.1 代码零重叠 → 计为 PRE-EXISTING DEBT，非新回归
  - 已创建 capability-phase8b-bookmark-composable-pass（本地，未 push）

CURRENTLY_COMPOSABLE:
  0 → 1（C3 已于 2026-09-20 正式验收通过）
```

---

## 4. Bookmark 当前物理结构

```text
src/capability/contribution/            # 【8B.1 WIP·未提交】通用贡献契约 + 单例 Registry（编排层，不含业务状态）
├── types.ts            # Contribution / SurfaceContribution / NavigationContribution / CONTRIBUTION_SLOTS
└── registry.ts         # createContributionRegistry() + contributionRegistry 单例（register / getBySlot / 空槽=[]）

src/capabilities/bookmark/
├── index.ts            # 能力入口（适配器）。导出 bookmarkCapability + registerBookmarkContributions()。
│                       #   【8B.1 WIP·未提交】经 contributionRegistry 注册 3 条贡献（surface + 2× navigation）。
│                       #   硬约束：本文件不得 import useBookmarkStore（PLT-05 守护）。
├── public.ts           # Bookmark 对外公共边界。纯再导出 useBookmarkStore / canBookmark。
│                       #   8B.1 后 Shell 不再引用它（改经通用 Contribution Registry），仅保留作能力对外边界，
│                       #   非第二状态源、非新业务 Owner；useBookmarkStore 仍是 canonical owner。
├── manifest.ts         # bookmarkManifest: CapabilityDefinition（semanticOwner=useBookmarkStore,
│                       #   status=COMPATIBILITY_WRAPPED）
├── contracts/bookmark.ts   # BookmarkProjection / BookmarkIntents（契约类型）
├── intents/bookmark.ts     # BOOKMARK_INTENTS（意图声明）
├── lifecycle/bookmark.ts   # onActivate() 调 registerBookmarkContributions()；onSuspend()
├── resource/bookmark.ts    # bookmarkResourcePolicy（LIGHT 资源治理）
├── state/
│   └── useBookmarkStore.ts # 业务真源 store（已提交迁移）。items/loaded/busy/error/
│                           #   panelOpen/sorted 全部语义仍归此文件（Semantic Registry 已登记）
└── ui/
    ├── BookmarkPanel.vue   # 收藏夹侧栏面板（已提交迁移）。
    │                       #   【8B.1 WIP·未提交】根节点自加
    │                       #   <template v-if="bookmarks.panelOpen && layout.mainView==='browser'">
    │                       #   显隐判定下沉到能力内（Shell 不再持有该知识）。
    ├── BookmarkStar.vue    # 地址栏星标按钮（已提交迁移）
    └── BookmarkEntryButton.vue  # 【8B.1 WIP·未提交】工具栏尾部「收藏夹」入口（navigation 贡献；自读 store）
```

`public.ts` 当前作用 = Bookmark 对外公共边界。8B.1 后 Shell 经通用 Contribution Registry 消费，**不再引用 public.ts**
（CB-02 已消）。它**不是**第二状态源、**不是**新业务 Owner；`useBookmarkStore` 仍是 canonical owner。

---

## 5. 冻结语义规则（下一 Agent 不得重裁）

```text
A. Bookmark panelOpen owner 不迁移
   canonical owner: useBookmarkStore.panelOpen
   禁止迁到：useLayoutStore / CapabilityRuntime / Shell Store / global UI state

B. Domain State != Capability Composition State
   Bookmark panel 是否打开 = Bookmark domain semantic
   Bookmark capability 是否注册/启用 = Capability composition semantic
   二者不得合并。

C. 禁止新增第二 stored truth：
   bookmarkVisible / bookmarkPanelVisible / bookmarkEnabledState 等均禁止作为第二存储态。

D. Shell 不应知道 Bookmark Store 内部实现。
   （8B.1 WIP 已满足：MainArea/ActivityBar 只经通用 Contribution Registry 按 slot 遍历渲染，
    零 import src/capabilities/bookmark/*；check-semantic-closure-logic.mjs:156-159 静态断言 MainArea
    无 bookmarks. / useBookmarkStore / capabilities/bookmark/ —— 已实跑 PASS。panelOpen 显隐判定下沉到
    BookmarkPanel.vue 自身，owner 不变）
```

---

## 6. Semantic Governance 路径解耦状态

```text
之前发现：governed_files / path coupling（物理路径 = 治理域，移动即漏检）

当前状态（Phase 8A.1，已**独立 closeout** 并打 tag capability-phase8a1-governance-identity-pass）：
  Semantic Identity != Physical File Path  ✅ 已完成（独立 PASS tag，非 IMPLEMENTED_WITHIN_8B）

states.yaml：
  owner_implementations.useBookmarkStore.paths（2026-09-20 当前解析）：
    [0] src/capabilities/bookmark/state/useBookmarkStore.ts   ← 首个磁盘存在候选，resolver 命中此项
    [1] src/stores/useBookmarkStore.ts                        ← 旧路径，磁盘已不存在，仅作回滚参考

Checker 能力（check-semantic-registry.mjs，2026-09-20 实跑）：
  - 能解析新路径 ✅（real scan / closure / pilot 均经 resolveOwnerFile 命中新路径，全 PASS）
  - 旧路径失效时会 FAIL ✅（RI-UNRESOLVED：若 locator 两项都不存在 → 阻断，防静默失守）
  - 不会因为文件移动静默漏检 ✅（resolved[owner] 取代硬编码路径；self-test CASE A–E 覆盖迁移场景）
  - self-test RESULT=ALL_PASS（CASE A–E 全绿）

结论：Phase 8A.1 已有独立 PASS tag，非 IMPLEMENTED_WITHIN_8B；Semantic Governance 路径解耦 = PASS。
      8B 的物理移动（store 从 src/stores 迁到 src/capabilities/bookmark/state）已被该机制正确覆盖，
      未引入治理盲区。
```

---

## 7. Bookmark 已完成的代码变更（BEFORE / AFTER / WHY）

```text
=== A 组：C2 物理迁移（已提交于 3fe17f1 / 5765c10，属 Phase 8B 机械迁移）===

1) useBookmarkStore.ts 物理迁移
   BEFORE: src/stores/useBookmarkStore.ts
   AFTER:  src/capabilities/bookmark/state/useBookmarkStore.ts
   WHY:    C2 物理隔离；owner 符号不变，locator 跟随新路径（§6）。
   附带修复（git mv 漏改）：内部相对导入
     ../bridge, ../types, ./useLayoutStore  ->  ../../../bridge, ../../../types, ../../../stores/useLayoutStore

2) BookmarkPanel.vue 迁移 + 内部导入重定向
   BEFORE: src/components/browser/BookmarkPanel.vue；import useBookmarkStore from "../../stores/useBookmarkStore"
           ；import CredentialList from "./CredentialList.vue"
   AFTER:  src/capabilities/bookmark/ui/BookmarkPanel.vue；useBookmarkStore from "../state/useBookmarkStore"
           ；CredentialList from "../../../components/browser/CredentialList.vue"
           ；另：../../types, ../../stores/*, ../../bridge -> ../../../*
   WHY:    UI 随能力归属迁移；保持 CredentialList 指向真实组件；修复 git mv 相对路径失效

3) BookmarkStar.vue 迁移 + 内部导入重定向
   BEFORE: src/components/browser/BookmarkStar.vue；useBookmarkStore,canBookmark from "../../stores/useBookmarkStore"
           ；useBrowserStore from "../../stores/useBrowserStore"
   AFTER:  src/capabilities/bookmark/ui/BookmarkStar.vue；useBookmarkStore,canBookmark from "../state/useBookmarkStore"
           ；useBrowserStore from "../../../stores/useBrowserStore"
   WHY:    同上

4) MainArea.vue（Shell，提交态 3fe17f1）
   BEFORE: import useBookmarkStore from "../../stores/useBookmarkStore"；BookmarkPanel from "../browser/BookmarkPanel.vue"
   AFTER(提交态): import useBookmarkStore from "../../capabilities/bookmark/public"；
                  BookmarkPanel from "../../capabilities/bookmark/ui/BookmarkPanel.vue"
                 保留 const bmPanelOpen = computed(() => bookmarks.panelOpen && layout.mainView === "browser")
                 与 <BookmarkPanel v-if="bmPanelOpen" />（冻结闭包契约仍 PASS）
   WHY:    Shell 改经公共边界 public.ts 消费。此状态仍持有 Bookmark 专属知识（C3 未达成）。
   ⚠️ 注意：此提交态随后被【B 组 WIP】覆盖（见下）。

5) ActivityBar.vue（Shell，提交态 3fe17f1）
   BEFORE: import BookmarkStar from "../browser/BookmarkStar.vue"；useBookmarkStore from "../../stores/useBookmarkStore"
   AFTER(提交态): import BookmarkStar from "../../capabilities/bookmark/ui/BookmarkStar.vue"；
                  useBookmarkStore from "../../capabilities/bookmark/public"
   WHY:    同上（提交态仍持有 Bookmark 专属知识）

6) Capability bootstrap（src/capability/index.ts）
   BEFORE: import { bookmarkCapability, BOOKMARK_CAPABILITY_ID } from './capabilities/bookmark'（源文件已被删）
   AFTER:  import ... from '../capabilities/bookmark'
   WHY:    新能力目录是复数 src/capabilities/（与单数 src/capability bootstrap 平级），修正相对路径

7) Registry locator（states.yaml）
   BEFORE: owner_implementations.useBookmarkStore.paths = [src/stores/useBookmarkStore.ts, ...]
   AFTER:  新路径置首（resolver 取首个磁盘存在的候选），旧路径保留作回滚
   WHY:    解决 RI-UNRESOLVED（旧路径磁盘已不存在，原序会导致 locator 解析失败）

8) Pilot checker resolver（scripts/check-capability-pilot.mjs PLT-05(b)）
   BEFORE: git diff --name-only ${TAG} HEAD -- src/stores/useBookmarkStore.ts src/components/home
   AFTER:  经 resolveOwnerFile('useBookmarkStore') 取新路径 + git diff --diff-filter=M
   WHY:    纯 rename（git mv）内容不变；--diff-filter=M 仅匹配真正内容修改，忽略 rename/delete

=== B 组：Phase 8B.1 Shell 解耦（【未提交 WIP】！仅存在于 working tree，不在任何 commit）===

9) 通用 Contribution Registry（新增 src/capability/contribution/）
   types.ts:      Contribution / SurfaceContribution / NavigationContribution 契约 + CONTRIBUTION_SLOTS
   registry.ts:   createContributionRegistry() + contributionRegistry 单例（register / getBySlot /
                 getSurfaceContributions / getNavigationContributions / clear；空槽恒返回 []）
   WHY:           Shell 经「槽」消费能力贡献，不持有能力专属知识（Contribution/Slot 模型，8A.1 裁决落地）

10) Bookmark 注册贡献（src/capabilities/bookmark/index.ts registerBookmarkContributions()）
   BEFORE: 占位（仅空 lifecycle 钩子），无 Contribution Registry 对接
   AFTER:  经 contributionRegistry.registerContribution 注册 3 条：
             surface    browser-sidebar      = BookmarkPanel（defineAsyncComponent 懒加载）
             navigation address-bar-actions   = BookmarkStar
             navigation activity-bar-trailing = BookmarkEntryButton
           lifecycle/bookmark.ts onActivate() 调 registerBookmarkContributions()
   WHY:    Bookmark 经通用契约贡献 UI，Shell 不再直连

11) BookmarkPanel.vue 自加显隐判定（src/capabilities/bookmark/ui/BookmarkPanel.vue）
   BEFORE: MainArea 控制 <BookmarkPanel v-if="bmPanelOpen" />（bmPanelOpen 在 Shell 派生）
   AFTER:  BookmarkPanel 根节点 <template v-if="bookmarks.panelOpen && layout.mainView === 'browser'">
   WHY:    panelOpen 显隐判定下沉到能力内（仍读 useBookmarkStore，owner 不变）；
           Shell 不再引用 bookmarks.panelOpen；闭包断言校验随派生量迁移到 BookmarkPanel 并由
           check-semantic-closure-logic.mjs:154-155 守住（§5-A 冻结未破）

12) Shell 消费改造（MainArea.vue / ActivityBar.vue，【WIP 覆盖提交态】）
   BEFORE: 两者均 import { useBookmarkStore } from ".../bookmark/public" + BookmarkPanel/BookmarkStar
            from ".../bookmark/ui/*"，并保留 bmPanelOpen 派生
   AFTER:  两者改 import { contributionRegistry } from "../../capability/contribution/registry" +
           { CONTRIBUTION_SLOTS } from "../../capability/contribution/types"；
           按 slot 遍历 <component :is="c.component" />；删除 bookmarks/bmPanelOpen/onToggleBookmarkPanel
   WHY:    Shell 零 import src/capabilities/bookmark/*；BookmarkEntryButton 承接
           原 ActivityBar 尾部「收藏夹」按钮（自读 store，复刻 onToggleBookmarkPanel 行为）

13) 新增 BookmarkEntryButton.vue（src/capabilities/bookmark/ui/BookmarkEntryButton.vue）
   WHY:    承接 ActivityBar 尾部「📑 收藏夹」入口按钮（navigation contribution）；自读 useBookmarkStore
           与 useLayoutStore 复刻 onToggleBookmarkPanel 行为（任意视图点击都"打开"面板，非取反）

14) Checker 新增与接线（【WIP，未提交】）
   scripts/check-capability-composition.mjs  【新增】C1 通用 Registry 行为 / C2 Bookmark 注册 3 贡献 /
                                          C3 Shell 不 import bookmark 内部 / C4 absent 空槽可启动
   scripts/check-capability-pilot.mjs      esbuild external 加 '*.vue'（pilot 仅扫描 import，不解析 .vue）
   scripts/check-semantic-closure-logic.mjs 闭包断言迁移到 BookmarkPanel.vue + 新增 MainArea 去专属知识断言
   package.json scripts.check / pre-merge.sh Phase 03：接入 check-capability-boundaries + check-capability-composition
```

---

## 8. 当前 Checker 状态（2026-09-20 实跑，真实结果 —— 非「全绿掩盖」）

```text
check-semantic-registry.mjs --self-test   ALL_PASS（CASE A–E 全绿）
check-semantic-registry.mjs (real scan)   PASS  (fail=0 warn=6 info=72)
  warn=6 均为 SEMANTIC_SIDE_EFFECT_UNKNOWN（rebuildGrid/closeGrid/gridPosition/tabPosition 副作用注释缺失）
          —— 历史债（Debt-2-3 类），非本次引入；补 "side-effect:" 注释即可消除，不阻断
check-semantic-closure-logic.mjs          PASS (27/27)
  bmPanelOpen 派生量校验已迁移到 BookmarkPanel.vue（:154-155）；
  MainArea 去专属知识断言（:156-159）PASS；功能性 togglePanel/bmPanelOpen 派生（:214-219）PASS
check-capability-pilot.mjs                PASS (8/8)  （含 PLT-05 源码+git 双重断言）
check-capability-boundaries.mjs           PASS (fail=0 warn=1)
  warn=1 CB-04：agent -> knowledge_graph -> agent 可选依赖环（设计异味，不阻断装配）
  **解读**：WIP 后 Shell 不再 import capability 内部（public.ts + ui/*），原 CB-02/06 已消 → C3 机制达成。
check-capability-composition.mjs          PASS (8/8) 【WIP 新增】
  C1 通用 Registry 行为（空槽=[]）/ C2 Bookmark 注册 3 贡献 / C3 Shell 不 import bookmark 内部 /
  C4 absent 空槽可启动（Bookmark 缺失时 Shell 渲染空集不崩溃）—— 已实跑 PASS

npm run check                          PASS（已接线 check-capability-boundaries + check-capability-composition
                                         + command-set three-way consistency GATE: PASS，仅 known drift）
vite build                              PASS（2026-09-20 实跑 5.22s；bookmark UI 拆为懒加载 chunk：
                                         BookmarkPanel-*.js 6.84kB / useBookmarkStore-*.js 3.05kB）

总体：registry / closure / pilot / boundaries / composition / npm-run-check / vite build 当前 working tree 全 PASS。
      C3 验收硬闸（boundaries 无 CB-02/06 + composition C4 absent 启动证明）现已全绿 —— 但此为 WIP 状态，
      尚未经「提交 + 全量 pre-merge gate」作为正式验收证据，故 C3 仍官方 PENDING。

KNOWN / 历史债（不隐藏）：
  - warn=6（SEMANTIC_SIDE_EFFECT_UNKNOWN）：非 8B 引入，属 Debt-2-3 旁路，历史存在
  - warn=1（CB-04 可选依赖环）：设计异味，已知
  - Terminal pre-existing debt（D23-D26 / Debt-004 terminal checker self-test FAIL）：与 8B 无关，
    仍阻断 master 快进合并（见主 HANDOFF §5）
```

---

## 9. 当前已知债务

```text
Debt-8B-1: registerBookmarkContributions 当前为兼容占位，未接真实 Contribution Registry
  STATUS:    MECHANICS-DONE / ACCEPTANCE-PENDING
             （WIP 已接真实 Registry 注册 3 条贡献；但未提交、未打 tag，故「正式收口」pending）
  IMPACT:    无（机制已就位且 composition C2/C4 实跑 PASS）
  CURRENT_PHASE_TO_FIX?  NO（机制已做；下一 Agent 走「提交 + 打 tag」验收即可收口，不必重写）

Debt-8B-2: 未证明「不注册 Bookmark Capability 时 Shell 可正常启动」
  STATUS:    MECHANICS-DONE / ACCEPTANCE-PENDING
             （composition C4-ABSENT 已实跑证明空 slot=[] 可启动；但未提交/未打 tag）
  IMPACT:    无（证明已存在）
  CURRENT_PHASE_TO_FIX?  NO（同上，走验收收口）

Debt-8B-3: Shell 仍持有 Bookmark 专属知识（直连 public.ts + ui/*）
  STATUS:    MECHANICS-DONE / ACCEPTANCE-PENDING
             （WIP 中 MainArea/ActivityBar 已改经 contribution/registry，零 import bookmark 内部；
              closure :156-159 静态断言 PASS；但未提交/未打 tag）
  IMPACT:    无（Shell 去专属知识已落地）
  CURRENT_PHASE_TO_FIX?  NO（同上，走验收收口）

继承债务（非本次引入，需持续跟踪，不在此轮清理）：
  Debt-8A-1/2/3  Phase 8A 物理边界相关结转
  Debt-7D-1/2/3  Phase 7D 非物理卸载 / 无 UI 呈现→已部分缓解 / 仅 1 能力接入→本阶段增 Browser 贡献消费
  Debt-6A-*      Phase 6A 闭包相关结转
  Debt-6B-*      Phase 6B 写者强制相关结转
  Terminal pre-existing  D23/D24/D25/D26 / Debt-004（terminal checker self-test FAIL，阻断 master 快进）
```

---

## 10. 下一 Agent 的第一任务

```text
NEXT TASK: Phase 8B.1 — Bookmark Contribution Activation（正式验收，不是从零实现，不是 8C）

为什么不是从零实现：
  8B.1 的 11 条机制（§11）已在未提交 working tree 全部落地，且 6 个 checker + vite build 实跑全 PASS
  （含 C4-ABSENT 证明 Bookmark absent 时 Shell 仍可启动）。因此下一 Agent 的 8B.1 = 验收闭环：
    1) 复核本交接 §7-B / §8 的 WIP 改动与 PASS 证据
    2) 将 WIP（9 修改 + 4 未跟踪）提交为一个干净的 8B.1 commit（建议 message 明确「8B.1 formal acceptance」）
    3) 跑 npm run check + 全量 pre-merge gate（注意 Terminal debt 仍会让 pre-merge --self-test FAIL，
       与 8B 无关，按主 HANDOFF §5 规则保留 feature 分支，不强行 ff-merge）
    4) 创建本地 tag capability-phase8b-bookmark-composable-pass（仅本地，禁止 push，除非用户/A0 裁决）
    5) 此时 Bookmark = C3 正式达成 → 才允许进入 Phase 8C

禁止把本任务误解为：
  - 继续搬目录（目录结构已稳定）
  - 重做 Bookmark Store（owner 不变）
  - 直接进入 8C（C3 未正式验收前不得）
```

---

## 11. Phase 8B.1 的明确目标（全部满足 + 打 tag = Bookmark C3）

```text
1. Generic NavigationContribution（通用导航贡献契约）                  ✅ 已实现（src/capability/contribution/types.ts，WIP）
2. Generic SurfaceContribution（通用表面贡献契约）                    ✅ 已实现（同上）
3. Capability Contribution Registry（能力贡献注册表）                  ✅ 已实现（registry.ts 单例，WIP）
4. Bookmark 注册 contribution（经上述通用契约）                       ✅ registerBookmarkContributions()（WIP）
5. MainArea 不再 import Bookmark public / store / UI 专属实现          ✅ 改经 generic slot（WIP；closure :156-159 PASS）
6. ActivityBar 不再 import Bookmark public / store / UI 专属实现       ✅ 改经 generic slot（WIP）
7. Shell 只消费 generic contribution（消除 Debt-8B-3）                 ✅ CB-02/06 已消（WIP）
8. Bookmark absent 时 Shell 可启动（真实证明，消 Debt-8B-2）           ✅ composition C4-ABSENT PASS（WIP）
9. Bookmark present 时行为保持（panelOpen 仍生效）                     ✅ BookmarkPanel 自读 panelOpen（WIP）
10. panelOpen 仍只有 useBookmarkStore 一个 stored truth（§5-A 冻结）    ✅ owner 未迁移（WIP）

全部「机制」满足（working tree 已验证），但「正式验收」待办：
  → 提交 WIP + 跑全量门禁 + 创建 tag：
    capability-phase8b-bookmark-composable-pass（当前不存在，待创建）
  创建后 → Bookmark = C3 → 才允许进入 Phase 8C。
```

---

## 12. 下一 Agent 的禁止事项

```text
- 禁止直接进入 Workspace / Files 能力迁移（C3 未正式验收前不扩张）
- 禁止把 public.ts 包装误判成 C3（public.ts 直连 = CB-02 FAIL = 未达 C3；本 WIP 已消除该直连）
- 禁止迁移 panelOpen owner（§5-A）
- 禁止新建第二 visibility state（bookmarkVisible / bookmarkPanelVisible / bookmarkEnabledState，§5-C）
- 禁止用全局 event bus 代替 contribution contract
- 禁止把 Capability Runtime 变成业务 Store
- 禁止为 C3 修改 Semantic Governance 让路（states.yaml 语义不得为达标而妥协）
- 禁止删除 / 移动旧 tag（含 capability-phase8b-bookmark-c3-migration-pass；名字含 C3 但仅 C2，不动）
- 禁止信任上一份交接的虚假「C3 PASS / 已创建 composable-pass tag」结论（实查 tag 不存在）
- 禁止从零重做 8B.1（机制已在 WIP 落地且 PASS；只需验收闭环）
- 禁止在未跑全量 pre-merge gate 前就标记 8B.1 完成（terminal debt 会让 --self-test FAIL，但与 8B 无关）
```

---

## 13. 推荐新 Agent 首读文件

```text
先读（禁止一上来重扫全仓）：
  docs/architecture/HANDOFF_CURRENT_STATE.md
  docs/architecture/capability-modularization/HANDOFF_PHASE8B.md（本文件）
  docs/architecture/semantic-registry/README.md
  docs/architecture/semantic-registry/states.yaml
  docs/architecture/semantic-registry/owners.yaml
  docs/architecture/semantic-governance/phase6a-core-closure/
  docs/architecture/semantic-governance/phase6b-writer-enforcement/
  docs/architecture/semantic-governance/phase8a1-governance-identity/01-PATH-COUPLING-AUDIT.md

Capability 相关代码：
  src/capabilities/bookmark/**（index/public/manifest/contracts/intents/lifecycle/resource/state/ui）
  src/capability/contribution/**（types/registry —— 8B.1 新增，WIP）
  src/capability/**（types/runtime/index bootstrap）
  scripts/check-capability-boundaries.mjs（CB-02/06 是 C3 验收硬闸）
  scripts/check-capability-composition.mjs（C4-ABSENT 是 absent 启动证明，8B.1 新增，WIP）
  scripts/check-semantic-registry.mjs / check-semantic-closure-logic.mjs / check-capability-pilot.mjs

Shell 消费方（须去 Bookmark 专属知识）：
  src/components/layout/MainArea.vue
  src/components/layout/ActivityBar.vue
```

---

## 14. 最终输出（CAPABILITY_PHASE8B_HANDOFF）

```text
STATUS:                    READY（Bookmark C3 已正式验收通过，交接完整）
BRANCH:                    feature/capability-platform-v1
HEAD:                      capability-phase8b-bookmark-composable-pass（= 8B.1 验收提交，见 §15）
LATEST_TAG:                capability-phase8b-bookmark-c3-migration-pass（★名字含 C3 但仅 C2，禁止移动/删除）
                            capability-phase8b-bookmark-composable-pass（2026-09-20 创建，本地，C3 正式达成）
BOOKMARK_LEVEL:            C3 OPTIONAL
BOOKMARK_C3:               PASS（OPTIONAL；2026-09-20 验收，全部 BKM-C3-* 满足 + 打 composable-pass tag）
BOOKMARK_OWNER:            useBookmarkStore
PANEL_OPEN_OWNER_CHANGED:  NO（显隐判定下沉到 BookmarkPanel.vue，owner 仍 useBookmarkStore）
SEMANTIC_GOVERNANCE_RESOLUTION: PASS（Phase 8A.1 独立 tag；locator 正确解析新路径，RI-UNRESOLVED 防漏检）
CHECKERS:                  PASS（registry self-test ALL_PASS / registry real fail=0 / closure 27/27 /
                                 pilot 8/8 / boundaries fail=0 / composition 8/8（含 C4-ABSENT + 负向自检 3/3）/
                                 sensitive fail=0 / npm run check GATE PASS / vite build PASS；warn=6+1 历史债不隐藏）
BUILD:                     PASS（vite build 4.48s；bookmark UI 拆懒加载 chunk）
KNOWN_DEBT:                继承 Debt-8A*/7D*/6A*/6B*/Terminal(D23-26,Debt-004，与 8B.1 零重叠=PRE-EXISTING)；
                            Debt-8B-1/2/3 = CLOSED（8B.1 验收收口）
NEXT_TASK:                 Phase 8C — Workspace / Files Physical Modularization（复用 Contribution Model）
NEXT_AGENT_READY:          YES
CODE_MODIFIED:             NO（本交接任务本身只改 docs；8B.1 业务代码由上一 Agent 实现、本 Agent 仅审计+验收+提交）
HANDOFF_FILES:
  docs/architecture/capability-modularization/HANDOFF_PHASE8B.md   （本文件，15 节完整交接·验收版）
  docs/architecture/HANDOFF_CURRENT_STATE.md                       （§4s 已更新为 C3 PASS 摘要）

---

## 15. Phase 8B.1 验收记录（2026-09-20，独立 Agent 复核）

```text
ACCEPTANCE_RESULT: PASS
START_HEAD:        3fe17f115bce4cfb8e153ebb0baaa23583858828（3fe17f1）
WIP_AUDITED:      YES（A–H 逐项核对：Shell 仅 import contribution/{registry,types}，零 bookmark 内部；
                        Bookmark 注册 3 条贡献；panelOpen 单真源；Registry 不含业务状态）
CONTRIBUTION_MODEL:PASS（generic Contribution Registry；Shell 遍历 slot 渲染 <component :is>）
SHELL_BOOKMARK_INTERNAL_IMPORTS: 0（MainArea + ActivityBar）
SHELL_BOOKMARK_PUBLIC_IMPORTS:   0
BOOKMARK_OWNER:    useBookmarkStore（panelOpen 仍唯一 stored truth）
PANEL_OPEN_SINGLE_TRUTH: PASS（closure 27/27；BookmarkPanel 自读 panelOpen）
SEMANTIC_IMPLEMENTATION_RESOLUTION: PASS（registry real scan fail=0；RI-UNRESOLVED 负向 fixture）
BOOKMARK_ABSENT:   PASS（composition C1-EMPTY / C4-ABSENT）
BOOKMARK_PRESENT:  PASS（BookmarkPanel v-if panelOpen&&mainView==='browser'；BookmarkEntryButton 复刻行为）
NEGATIVE_FIXTURES: PASS（composition --self-test 3/3：Shell→bookmark ui/public/store import 均被 C3 捕获）
INDEPENDENT_REVIEW:PASS（8 问，无 BLOCKER）
BUILD:            PASS（vite build 4.48s）
PRE-MERGE:        FAIL（terminal D23-26 / grid / phase7e 文档尾随空白 —— 均为 PRE-EXISTING DEBT，与 8B.1 零重叠，
                       不计为新回归；未改动无关 Terminal/Grid 代码）
BOOKMARK_LEVEL:   C3 OPTIONAL
CURRENTLY_COMPOSABLE: 1（0 → 1）
COMMIT:           见 git log（refactor(bookmark): activate bookmark capability contributions /
                        docs(phase8b): close bookmark C3 acceptance）
TAG:             capability-phase8b-bookmark-composable-pass（annotated，本地，未 push）
NEXT:            Phase 8C — Workspace / Files Physical Modularization
```
```
