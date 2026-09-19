# Phase 3 — Bookmark Semantic Governance

> 状态：DESIGN（实现见 `docs/architecture/semantic-registry/*.yaml` 扩展 + 复用 `scripts/check-semantic-registry.mjs` R2/R3/R4/R6）
> 基线：继承 `semantic-phase2-workspace-pass` + Semantic Registry + Semantic Gate + Recovery Layer + Handoff System
> 触发：Bookmark 域（useBookmarkStore.ts）承载收藏夹真源，存在第二收藏源 / 重复收藏 / 与主页快捷方式混淆风险。

---

## 1. State Model（受治理状态）

```text
items       [STORED, single source of truth]
  = 收藏夹列表（后端 data_dir/bookmarks.json 的前端镜像；后端为权威持久化）
  = 唯一收藏列表真源；任何“是否收藏”判定必须经 items + normalizeUrl 派生，禁止第二份 Bookmark[]

loaded      = 是否已从后端加载过（避免重复 load）
busy        = 异步操作进行中（add/remove/import/toggle）
error       = 最近一次 IPC 失败信息（必须可见，否则 ⭐ 点了无反馈）
panelOpen   = 收藏夹侧栏开关（状态放在本 store，刻意不进 useLayoutStore）

sorted      [DERIVED]
  = items 按 created_at 倒序的稳定视图（同时间戳按 url 排序）
  = 纯派生，禁止存储（R6 护栏，类比 desiredGridVisibility / currentLocalPath）
```

### 关键不变量（文档级，非 checker 强制）

```text
INV-3-1  items 是唯一收藏列表真源。add 必须按 id / normalizeUrl 替换（splice），
         禁止 push 重复项（后端同 URL 视为更新，push 会导致侧栏重复）。
INV-3-2  收藏“身份”= normalizeUrl(url)（忽略协议/主机大小写与末尾斜杠）。
         任何“是否收藏”判定必须经 normalizeUrl，禁止裸 url 比较。
INV-3-3  Bookmark（后端持久化）≠ 主页快捷方式（useHomeStore，localStorage）。
         二者完全独立，禁止合并为一个源（rejected: mergeBookmarksIntoHome）。
```

---

## 2. Intent Model（意图）

```text
intent: add
  语义：新增/更新收藏（同 URL 视为更新，保留 id/created_at）
  owner：useBookmarkStore.add
  duplicate_names：bookmarkAdd / createBookmark / saveBookmark

intent: remove
  语义：取消收藏（按 id）
  owner：useBookmarkStore.remove
  duplicate_names：deleteBookmark / removeBookmark

intent: toggle
  语义：地址栏 ⭐ 唯一入口：已收藏→按 id 移除，未收藏→写入
  owner：useBookmarkStore.toggle
  duplicate_names：toggleBookmark / starUrl / bookmarkToggle

intent: importFile
  语义：从 HTML/JSON 书签文件批量导入（按 normalizeUrl 去重）
  owner：useBookmarkStore.importFile
  duplicate_names：importBookmarks

intent: togglePanel
  语义：切换收藏夹侧栏显隐
  owner：useBookmarkStore.togglePanel
  duplicate_names：openBookmarkPanel / showBookmarkPanel
```

### rejected（明确不做）

```text
rejected: mergeBookmarksIntoHome
  原因：收藏夹（后端持久化、跨会话、可导入导出）与主页快捷方式（localStorage、仅本机）
        语义与生命周期完全不同；合并会破坏第二真源边界并引入数据错位

rejected: 在组件里直接维护第二份 Bookmark[] 或裸 url 收藏判定
  原因：任何第二列表都会与 items 漂移，导致 ⭐ 高亮与实际收藏对不上
```

---

## 3. Owner Model（Owner）

```text
bookmark
  owner = useBookmarkStore
  owns  = items / loaded / busy / error / panelOpen / sorted
  owner_only_api = add / remove / toggle / importFile / togglePanel / load / findByUrl
  forbidden_callers = components/**（不得直写 items / panelOpen；只能调 store action）
  authorized_callers = useLayoutStore（仅切视图）、BookmarkStar/BookmarkPanel（消费 store）
```

---

## 4. Lifecycle（生命周期）

```text
启动 / 进入收藏相关视图
  └─ load()  → items = bridge.bookmarkList()；loaded=true（单次，避免重复 load）

用户点 ⭐（地址栏）
  └─ toggle(url,title) → findByUrl → 有则 remove(id) / 无则 add(url,title)
     （add 内部按 id/normalizeUrl splice 替换，绝不 push 重复）

用户导入书签文件
  └─ importFile(file) → 解析 → normalizeUrl 去重 → 逐条 add（后端去重兜底）

面板显隐
  └─ togglePanel() → panelOpen = !panelOpen
```

---

## 5. Side Effect（副作用）

```text
bookmarkPersist（bridge.bookmarkAdd / bookmarkRemove）
  - 真实行为：写后端 data_dir/bookmarks.json（持久化，跨会话）
  - 声明：registry 文档化（requires_declaration=false，避免对既有合法调用产生 R5 噪声）
  - 与 useHomeStore 主页快捷方式（localStorage）完全隔离，不共享写入路径
```

---

## 6. Checker Plan（门禁设计）

### 6.1 复用 `scripts/check-semantic-registry.mjs`（registry 驱动）

- **R2（扩展）**：将 `src/stores/useBookmarkStore.ts` 加入 `states.yaml` 的 `governed_files`。
  其内 6 个声明（items/loaded/busy/error/panelOpen/sorted）全部纳入 `states`，
  任何新 `ref/reactive/computed` 声明若未登记 → FAIL（强制单一列表真源，禁止第二 Bookmark[]）。
- **R4（扩展）**：5 个 bookmark intent 的 `duplicate_names` 在任意文件被定义即 FAIL（一个意图一个入口）。
- **R3（扩展）**：`bookmark` owner 的 `forbidden_callers`（components/** 直写 items/panelOpen）→ FAIL。
- **R6（复用）**：`sorted` 标记 `derived: true` → 禁止声明为 ref/reactive、禁止 `.value =`，
  固化派生不变量（与 Phase 1/2 同机制）。

### 6.2 接入 pre-merge

`check-semantic-registry.mjs` 自 Phase 1.5 已接入 pre-merge；本 Phase 扩展 YAML 后自动覆盖。

---

## 7. Acceptance Matrix（验收矩阵）

```text
AC-1  R2：useBookmarkStore.ts 内 6 声明全部登记于 states → 真实仓库 fail=0            [REAL]
AC-2  R4：bookmark intent 的 duplicate_names 在真实仓库无定义 → fail=0                [REAL]
AC-3  R6：sorted 为 computed 且无误赋值 → 真实仓库 R6 fail=0                          [REAL]
AC-4  R2：在 useBookmarkStore.ts 注入第二份 ref<Bookmark[]>（未登记）→ 检出 FAIL       [NEG FIXTURE]
AC-5  R6：注入 fixture（sorted = ref([])）→ 检出 SEMANTIC_DERIVED_STATE_STORED         [NEG FIXTURE]
AC-6  R6：positive fixture（sorted = computed(...)）→ 0 fail/warn                     [POS FIXTURE]
AC-7  check-semantic-registry.mjs --self-test 全量 ALL_PASS（含 bookmark 域）          [SELF-TEST]
AC-8  pre-merge 中 semantic-registry 项仍绿（不引入新 FAIL；terminal 债为既有）        [GATE]
AC-9  不降低任何既有 Checker；未改业务代码                                          [NO REGRESSION]
AC-10 HANDOFF_CURRENT_STATE.md 更新 Phase 3 状态 + tag 分类                          [HANDOFF]
```

---

## 8. 与既有基础设施的关系

```text
check-semantic-registry.mjs   复用并扩展（R2/R3/R4 域扩展 + 复用 R6），不重造
states/intents/owners/side-effects.yaml   扩展 bookmark 域，不重造
pre-merge.sh               已接入（registry checker），无需改接线
useHomeStore 主页快捷方式   显式独立（observed / rejected merge），不并入 bookmark
```

---

## 9. Known Debt（本 Phase 内）

```text
DEBT-3-1  normalizeUrl 身份键未做 checker 强制
  当前仅文档化（INV-3-2）；静态强制“add 必须走 normalizeUrl 替换”成本高、易误报，交专项。

DEBT-3-2  panelOpen 置于 bookmark store 而非 layout
  属有意设计（注释明确），非缺陷；登记为 CURRENT_FACT，不移动。

DEBT-3-3  bookmarkPersist 副作用仅文档化（requires_declaration=false）
  与 Phase 2 writeFile 同口径；收紧需改 side-effects.yaml + 业务代码加标记。
```
