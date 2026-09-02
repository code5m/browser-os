# M2-image-ui-static-shell（2026-09-02 11:46）

> 批次：`free-model-prework-4-full` · 路由 `AI:FAST`
> 锚定：WBS §4.4 **M2-9**（图片预览 UI）· 需求 **#10 支持图片展示**
> 状态：⏸ **静态壳方案 / mock 数据 / 未接真实后端 / 不宣称 PASS**
> 契约依赖：`M2-1.a-prework-20260902-1055.md`（ImageRef 契约）→ 本卡的 mock 必须与之同构

---

## 1. 目标

产出**图片预览 UI 的静态壳方案**：画廊、灯箱、缩放、旋转、删除确认、错误态、空态、键盘操作、mock 数据、组件拆分、验收截图点。
**不接真实后端**——所有数据来自 `src/mocks/images.ts`（新建，仅前端），不调任何 Tauri 命令。

---

## 2. 现状证据（2026-09-02 实测）

| 项 | 实测 |
|---|---|
| 图片相关前端组件 | ❌ 无。22 个 `.vue` 中无 Gallery/Lightbox/ImageViewer |
| 现有面板 | `src/components/workspace/ArtifactPanel.vue`（成果库）、`FilePanel.vue`、`RepoPanel.vue`、`AuditPanel.vue` |
| 图片相关命令 | ❌ 59 个白名单命令中无 `list_images` / `read_image` |
| 领域模型 | `src-tauri/src/domain.rs` 无 `ImageRef` |
| 图片入口 | 浏览器侧资源采集见 `src-tauri/injected/collect.js:38-85`（上报资源，非展示） |
| 前端依赖 | `vue@^3.4` / `pinia@^2.3`；**无路由、无 UI 库、无图片库** |

→ 本卡是**纯新增前端**，无历史包袱。

---

## 3. 必改文件候选（本批次**只写方案，不创建**）

| 文件 | 性质 | 说明 |
|---|---|---|
| `src/mocks/images.ts` | 新增 | mock 数据 + 类型（与 `ImageRef` 同构） |
| `src/types/image.ts` | 新增 | `ImageRef` 前端类型定义 |
| `src/components/workspace/ImageGallery.vue` | 新增 | 画廊（网格 + 虚拟滚动占位） |
| `src/components/workspace/ImageLightbox.vue` | 新增 | 灯箱（大图 + 缩放 + 旋转 + 导航） |
| `src/components/workspace/ImageThumb.vue` | 新增 | 单张缩略图卡片（含错误态/加载态） |
| `src/stores/useImageStore.ts` | 新增 | 状态机（静态壳阶段读 mock） |
| `src/components/workspace/ImagePanel.vue` | 新增 | 容器（挂到 workspace 侧栏） |
| `src/components/shared/ConfirmModal.vue` | **复用** | 删除确认（已存在） |
| `src/utils/format.ts` | 扩展 | 字节数 → 可读（已有，复用） |
| `src/components/layout/ActivityBar.vue` | 改动 | 增加「图片」入口图标 |

---

## 4. 数据结构契约（与 `M2-1.a` 对齐）

```ts
// src/types/image.ts
export type ImageMime = 'image/png' | 'image/jpeg' | 'image/gif' | 'image/webp' | 'image/svg+xml';

export interface ImageRef {
  id: string;              // 稳定 id（内容 sha256 前 16 位，见 M2-1.a）
  name: string;            // 文件名
  path: string;            // 绝对路径（canonicalize 后）
  mime: ImageMime | string;
  size: number;            // 字节
  width?: number;          // 可能为 0（读取失败）
  height?: number;
  createdAt: string;       // ISO 8601
  source: 'workspace' | 'download' | 'clipboard' | 'web';
  thumbPath?: string;      // 缩略图路径（后续异步生成）
}

/** UI 层的运行态，不属于 ImageRef 本体，禁止混入持久化结构 */
export interface ImageViewState {
  scale: number;           // 1.0 = 适应窗口
  rotation: 0 | 90 | 180 | 270;
  offsetX: number;
  offsetY: number;
  loading: boolean;
  error?: string;
}
```

**红线**：`ImageViewState` **不得**写回 `ImageRef`（避免把 UI 状态持久化）。

---

## 5. 组件拆分

```
ImagePanel.vue                    （容器：工具栏 + 过滤 + 列表）
├── ImageToolbar.vue              （排序 / 过滤 / 视图密度 / 全选）
├── ImageGallery.vue              （网格）
│   └── ImageThumb.vue            （单卡片：缩略图 / 加载骨架 / 错误占位 / 选中框）
├── ImageEmptyState.vue           （空态）
├── ImageLightbox.vue             （Teleport 到 body 的灯箱）
│   ├── ImageCanvas.vue           （img + transform: scale/rotate/translate）
│   ├── LightboxToolbar.vue       （放大/缩小/1:1/适应/旋转/下载/删除）
│   └── LightboxNav.vue           （上一张/下一张/计数）
└── ConfirmModal.vue              （复用 shared，删除确认）
```

| 组件 | 职责 | 不做什么 |
|---|---|---|
| `ImagePanel` | 布局 + 数据编排 | 不直接操作 DOM |
| `ImageGallery` | 网格渲染 + 多选 | 不做大图 |
| `ImageThumb` | 单卡片三态（loading/ok/error） | 不发起删除 |
| `ImageLightbox` | 键盘/手势 + 视图状态机 | 不做持久化 |
| `ImageCanvas` | 纯 `transform` 渲染 | 不做业务逻辑 |

**静态壳阶段的硬约束**：所有数据来自 `mocks/images.ts`，组件内**零 `invoke`**。

---

## 6. 交互与状态机

### 6.1 画廊

| 项 | 方案 |
|---|---|
| 布局 | CSS Grid `repeat(auto-fill, minmax(140px, 1fr))`，`gap: 8px` |
| 缩略图 | `object-fit: cover`，固定 `aspect-ratio: 1 / 1` |
| 加载态 | 骨架屏（灰色块 + 微光动画），不要转圈 |
| 排序 | 创建时间 / 名称 / 大小（升序降序切换） |
| 过滤 | MIME 类型多选 + 名称关键字（前端过滤） |
| 多选 | `Ctrl/⌘ + 点击` 切换、`Shift + 点击` 区间；选中态蓝色边框 |
| 删除 | 工具栏「删除」或右键菜单 → `ConfirmModal` |
| 数量 | 静态壳不做虚拟滚动，mock 控制在 60 张以内；>200 张再引入 `vue-virtual-scroller`（会加依赖，需单独评估） |

### 6.2 灯箱

| 项 | 方案 |
|---|---|
| 打开 | 单击缩略图；`Teleport to body`，`z-index: 1000` |
| 背景 | `rgba(0,0,0,.85)`；点击背景关闭（**拖拽中**不关闭） |
| 初始缩放 | 「适应窗口」：`scale = min(vw/imgW, vh/imgH, 1)`（不放大超过 1:1 除非用户操作） |
| 缩放范围 | `0.1 ~ 8.0`；滚轮以**光标位置**为锚点 |
| 旋转 | 每次 +90°，旋转后自动重新计算「适应」基准 |
| 拖拽 | 仅在 `scale > fitScale` 时可拖；边界回弹（不允许拖出视口太多） |
| 导航 | 左右按钮 + 键盘；切换时**重置** scale/rotation/offset |
| 元信息 | 顶部/底部浮层显示 名称 / 尺寸 / 体积 / 序号（3/60） |
| 关闭 | `Esc` / 点击背景 / 右上角 ✕ |

### 6.3 键盘操作表（必须全部实现）

| 键 | 行为 |
|---|---|
| `←` / `→` | 上一张 / 下一张 |
| `+` / `=` | 放大 1.2× |
| `-` / `_` | 缩小 1.2× |
| `0` | 适应窗口 |
| `1` | 1:1 原始尺寸 |
| `r` / `R` | 顺时针旋转 90° |
| `Shift + r` | 逆时针旋转 90° |
| `Delete` / `Backspace` | 删除当前（弹 `ConfirmModal`） |
| `Esc` | 关闭灯箱 |
| `空格` | 播放/暂停自动轮播（静态壳可选） |

> 焦点陷阱：灯箱打开时 `focus` 到灯箱容器，`Esc` 关闭后焦点回到触发的缩略图。

---

## 7. 错误态 / 空态矩阵

| 场景 | 展示 | 禁止 |
|---|---|---|
| 无图片（空态） | 居中插画 + 「暂无图片」+ 副文案「从浏览器右键保存或拖入图片」 | ❌ 空白区域 |
| 过滤后无结果 | 「没有符合过滤条件的图片」+ 「清除过滤」按钮 | ❌ 与真·空态混淆 |
| 缩略图加载失败 | 灰色占位 + 破损图标 + 文件名 tooltip | ❌ 无限重试 / ❌ 白块 |
| 大图加载失败 | 灯箱内居中错误文案 + 「重试」按钮 | ❌ 关闭灯箱 |
| 不支持的 MIME | 卡片显示「不支持预览：`image/xxx`」+ 「用外部应用打开」 | ❌ 直接渲染（可能 XSS / 崩溃） |
| 图片过大（>20 MB） | 提示「文件较大，预览可能缓慢」+ 仍允许打开 | ❌ 静默卡死 |
| SVG 预览 | ⚠️ 按 `M2-1.a` 契约：**禁止 `v-html` 内联渲染**（XSS 面），统一走 `<img src>` | ❌ `v-html` / ❌ `innerHTML` |
| 删除中 | 卡片置灰 + 不可重复点击 | ❌ 重复提交 |
| 删除失败 | Toast 提示 + 恢复卡片 | ❌ 静默消失 |

**XSS 红线（K8）**：任何来自不可信来源（网页采集）的图片名/路径，渲染时**只能**用 `{{ }}` 文本插值或 `:alt`，**禁止 `v-html`**。

---

## 8. mock 数据设计（`src/mocks/images.ts`）

```ts
import type { ImageRef, ImageMime } from '../types/image';

const MIME: ImageMime[] = ['image/png', 'image/jpeg', 'image/gif', 'image/webp', 'image/svg+xml'];

/** 生成 n 张 mock 图片；用固定种子保证每次刷新顺序一致（便于截图比对） */
export function makeMockImages(n = 42): ImageRef[] { /* … */ }

export const MOCK_IMAGES: ImageRef[] = [
  // 0) 正常 png
  { id: 'a1b2c3d4e5f60718', name: 'screenshot-2026-09-01.png', path: '/home/u/Documents/极智笔记/a.png',
    mime: 'image/png', size: 248_123, width: 1920, height: 1080,
    createdAt: '2026-09-01T10:22:31+08:00', source: 'workspace' },
  // 1) 超大图（触发「较大」提示）
  { id: 'b2c3d4e5f6071829', name: 'raw-photo.jpg', path: '/home/u/Pictures/raw.jpg',
    mime: 'image/jpeg', size: 24_117_964, width: 8000, height: 6000,
    createdAt: '2026-08-30T19:03:00+08:00', source: 'download' },
  // 2) 尺寸未知（读取失败）
  { id: 'c3d4e5f60718293a', name: 'broken-header.png', path: '/home/u/x.png',
    mime: 'image/png', size: 1024, width: 0, height: 0,
    createdAt: '2026-08-29T08:00:00+08:00', source: 'web' },
  // 3) 不支持的 MIME
  { id: 'd4e5f60718293a4b', name: 'icon.svg.avif', path: '/home/u/i.avif',
    mime: 'image/avif', size: 8192, width: 256, height: 256,
    createdAt: '2026-08-28T12:00:00+08:00', source: 'web' },
  // 4) SVG（XSS 反向用例）
  { id: 'e5f60718293a4b5c', name: 'untrusted.svg', path: '/home/u/u.svg',
    mime: 'image/svg+xml', size: 2048, width: 100, height: 100,
    createdAt: '2026-08-27T16:45:00+08:00', source: 'web' },
  // 5) 超长中文名（布局反向用例）
  { id: 'f60718293a4b5c6d', name: '这是一个非常非常非常长的中文文件名用来测试布局是否会被撑破的截图文件.png',
    path: '/home/u/长名.png', mime: 'image/png', size: 512_000, width: 800, height: 600,
    createdAt: '2026-08-26T09:10:00+08:00', source: 'clipboard' },
  // … 其余由 makeMockImages 生成，覆盖 4 种 MIME × 3 种 source
];

/** 空态数据集（供 QA 切换） */
export const MOCK_EMPTY: ImageRef[] = [];
/** 全失败数据集（全部缩略图加载失败） */
export const MOCK_ALL_BROKEN: ImageRef[] = MOCK_IMAGES.map(i => ({ ...i, width: 0, height: 0 }));
```

**mock 缩略图来源**：静态壳阶段用**内联 data-URI 占位图**（1×1 或 8×8 纯色 PNG，Base64 内联），**不得**引用 `https://picsum.photos` 之类的外链（离线红线）。

```ts
export const PLACEHOLDER_PNG =
  'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==';
```

**mock 切换开关**：`src/mocks/index.ts` 导出 `const MOCK_MODE = import.meta.env.VITE_MOCK_MODE ?? 'normal'`，支持 `normal | empty | allBroken`，便于人工截图验收。

---

## 9. 验收截图点（人工 GUI 必拍）

| # | 场景 | 截图点 | 判据 |
|---|---|---|---|
| S1 | 空态 | `ImageEmptyState` | 有插画+文案，非空白 |
| S2 | 正常画廊（42 张） | 网格首屏 | 无骨架残留、无错位 |
| S3 | 缩略图加载中 | 骨架屏 | 不闪、不跳版 |
| S4 | 部分加载失败 | 破损占位 | 文件名可见 |
| S5 | 不支持 MIME | 卡片提示 | 有「不支持预览」 |
| S6 | 灯箱打开（适应窗口） | 大图 + 浮层 | 图片完整可见、不越界 |
| S7 | 灯箱放大 4× | 放大态 | 可拖动、锚点正确 |
| S8 | 旋转 90° | 旋转态 | 不裁切、容器自适应 |
| S9 | 删除确认弹窗 | `ConfirmModal` | 文案含文件名、有取消 |
| S10 | 过滤后无结果 | 空结果态 | 与真·空态文案不同 |
| S11 | 超长中文名 | 卡片 | 省略号截断，不撑破布局 |
| S12 | SVG untrusted | 卡片 | 走 `<img>` 渲染，**脚本不执行**（DevTools 无报错/无 alert） |
| S13 | 键盘 `←/→/+/-/0/1/r` | 连拍或录屏 | 每个键都生效 |
| S14 | 窄屏（<900 px） | 单栏 | 不横向滚动 |

---

## 10. 风险

| 级别 | 风险 | 缓解 |
|---|---|---|
| 高 | SVG XSS（采集自网页的 SVG 可含脚本） | 一律 `<img src>`；**禁止 `v-html`**；见 `M2-1.a` |
| 中 | 大图（>20 MB / 8000 px）渲染卡顿或 OOM | 静态壳加提示；真实实现需缩略图优先 + 懒加载 |
| 中 | 引入图片库（如 `viewerjs`）破坏「零 UI 库」现状 | 静态壳**零新依赖**；若后续要加，需单独评估主 JS 体积（baseline 505 KB 硬门槛） |
| 中 | `asset:` 协议 scope 未覆盖 `~/Pictures` | 见 `plugin-permission-taskcard`；静态壳用 data-URI 绕开 |
| 低 | 中文文件名布局撑破 | `text-overflow: ellipsis` + `min-width: 0` |

---

## 11. 反向用例

| # | 用例 | 期望 |
|---|---|---|
| R1 | 断网打开图片面板 | mock 数据正常渲染（零外链） |
| R2 | 上传一个含 `<script>alert(1)</script>` 的 `.svg` | 以 `<img>` 渲染，脚本**不执行** |
| R3 | 文件名含 `<img src=x onerror=alert(1)>` | 文本插值转义，不触发 |
| R4 | 缩略图全部加载失败 | 全部显示破损占位，页面不崩溃 |
| R5 | 在灯箱中放大后按 `←` | 切到上一张且**缩放重置** |
| R6 | 拖拽图片时点击背景 | **不关闭**灯箱 |
| R7 | 连续快速点删除 3 次 | 只弹一次确认，执行后按钮禁用 |
| R8 | 缩放到 `0.1` 再缩放到 `8` | 不越界，不出现负 scale |
| R9 | 主 JS 体积 | 构建后对照 `logs/baseline-2026-08-27.md`（505 KB），增量需说明 |

---

## 12. 验收命令（**均未执行**）

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3

# 12.1 静态壳不得触碰真实执行通道（红线 K9）
grep -rn "invoke\|__TAURI__" src/components/workspace/Image*.vue src/stores/useImageStore.ts | wc -l
# 期望 0

# 12.2 禁用 v-html（红线 K8）
grep -rn "v-html\|innerHTML" src/components/workspace/Image*.vue | wc -l
# 期望 0（除明确注释说明的白名单）

# 12.3 零外链 mock
grep -rn "https\?://" src/mocks/images.ts | wc -l
# 期望 0

# 12.4 键盘绑定齐全
grep -n "ArrowLeft\|ArrowRight\|Escape\|Delete\|KeyR" src/components/workspace/ImageLightbox.vue | wc -l
# 期望 >= 5

# 12.5 构建与体积门槛
npm run build && ls -lh dist/assets/*.js
# 对照 logs/baseline-2026-08-27.md 的 505 KB 硬门槛

# 12.6 人工截图
# 按 §9 的 S1~S14 逐点截图，归档到 logs/assist/images-ui-<date>/
```

---

## 13. 失败动作

| 失败 | 动作 |
|---|---|
| 出现 `invoke` 调用 | 立即回退到纯 mock；接后端是**另一张卡** |
| `v-html` 未清零 | 改为 `<img>` / 文本插值；不得加例外 |
| 构建后主 JS 超 505 KB | 评估组件懒加载；不得放宽阈值 |
| 大图卡死 | 静态壳先加「较大」提示；真实实现再上缩略图管道 |
| 截图点缺失 | 补齐后再验收；不得跳过截图点宣称完成 |

---

## 14. 推荐模型

`AI:BALANCED`（Vue 3 组件拆分 + 状态机 + CSS，逻辑清晰量大）。
**人工 GUI 验收必做**（14 个截图点 + 9 个反向用例），AI 不得代签。
