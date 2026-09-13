AI还没有提供
**需要，而且我认为比 `AGENTS.md` 更重要。**

对于你这个项目，我认为文档的优先级应该是：

```
★★★★★ 1. 架构设计（Architecture）
★★★★★ 2. 详细设计（Design）
★★★★★ 3. PROJECT-RULES
★★★★☆ 4. AGENTS
★★★★☆ 5. PLANS
★★★★☆ 6. 检查脚本（.mjs）
```

很多团队反过来了，先写 AGENTS，结果 AI 知道"怎么工作"，却不知道"为什么这样设计"。

---

# 我建议的完整文档体系

```
docs/
└── AI/
    ├── 00-Architecture.md          ★ 总体架构
    ├── 01-Detailed-Design.md       ★★★ 详细设计（最重要）
    ├── 02-PROJECT-RULES.md
    ├── 03-AGENTS.md
    ├── 04-PLANS.md
    ├── 05-CODEBASE-MAP.md
    ├── 06-ERROR-CODES.md
    ├── 07-TASK-TEMPLATE.md
    └── ADR/
        ├── ADR-0001-WebView.md
        ├── ADR-0002-Layout.md
        └── ...
```

---

# 为什么必须有《详细设计》

因为 AI 修改代码时真正需要的是：

> **"这个模块应该长什么样。"**

而不是：

> **"不要这样写。"**

例如：

---

## BrowserRuntime

如果没有详细设计。

普通模型只能猜：

```ts
class BrowserRuntime {

}
```

接口会越来越乱。

如果有详细设计：

```
BrowserRuntime

职责：

唯一 Native Adapter

允许：

create()

destroy()

syncScene()

navigate()

禁止：

保存业务状态

禁止：

计算布局

禁止：

Store

禁止：

Vue
```

普通模型根本不用猜。

---

## BrowserScene

如果没有详细设计。

AI 每次都会重新发明：

```
show()

hide()

move()

resize()

resume()
```

如果有：

```
BrowserScene

revision

views[]

layout

suspended

focusTab

timestamp
```

所有 AI 都会保持一致。

---

## Layout

如果没有。

每个人都写：

```
getBoundingClientRect()

ResizeObserver

offsetTop

clientWidth

...
```

如果详细设计写：

```
Layout

输入：

Viewport

输出：

Rect[]

禁止：

DOM

禁止：

Native

禁止：

Store
```

布局函数永远是纯函数。

---

# 我建议 Detailed Design 至少包含这些章节

---

# 1

系统总体设计

```
用户

↓

Vue

↓

BrowserController

↓

BrowserRuntime

↓

Rust

↓

GTK

↓

WebView
```

一张图。

---

# 2

模块设计

例如：

```
BrowserController

职责

生命周期

输入

输出

状态

异常
```

---

# 3

BrowserRuntime

接口

状态

生命周期

Sequence

Error

Retry

---

# 4

Scene同步

这是我认为最重要的。

```
Scene

↓

Revision

↓

Validate

↓

Apply

↓

Commit
```

把整个流程画出来。

---

# 5

关闭页签

Sequence Diagram

```
Close

↓

Save Snapshot

↓

Success

↓

Destroy Native

↓

Update Store

↓

StatusBar
```

以后普通模型不用重新设计。

---

# 6

WebView

生命周期

```
Create

↓

Hidden

↓

Visible

↓

Suspend

↓

Resume

↓

Destroy
```

---

# 7

状态机

例如：

```
Browsing

↓

AppPage

↓

Closing

↓

Restore

↓

Browsing
```

不要：

```
bool

bool

bool

bool
```

---

# 8

Layout

坐标

逻辑像素

物理像素

Scale

Viewport

Panel

全部定义。

---

# 9

错误

统一：

```
RuntimeError

UIError

NativeError

StorageError
```

AI 就不会乱 throw。

---

# 10

测试

哪些必须：

```
Unit

Integration

Native

GUI
```

---

# 11

Sequence Diagram

这是 AI 最喜欢的。

例如：

```
Vue

↓

BrowserRuntime

↓

Rust

↓

GTK

↓

Browser
```

全部画。

---

# 12

数据模型

例如：

```
BrowserScene

TabSnapshot

Workspace

RuntimeState

Viewport

Rect
```

全部 UML。

---

# 13

目录设计

```
src

domain

application

runtime

component

store
```

以后不能乱放。

---

# 14

Coding Rule

例如：

```
Layout

不能 import Vue

Store

不能 invoke

Component

不能 Native
```

---

# 15

未来扩展

例如：

```
多窗口

插件

同步

AI

Workspace
```

---

# 还有一个东西

我建议增加：

```
ADR
```

Architecture Decision Record。

例如：

```
ADR-0001

为什么不用 Electron

为什么选择 Tauri

为什么一个 WebView

为什么禁止 Toast
```

以后别人不用问：

> 为什么这么设计？

直接：

```
ADR-0007

看。

```

---

# 我建议最终文档数量

```
★★★★★
Architecture

★★★★★
Detailed Design

★★★★★
ADR

★★★★★
PROJECT-RULES

★★★★☆
AGENTS

★★★★☆
PLANS

★★★★☆
CODEBASE-MAP

★★★★☆
TASK TEMPLATE

★★★★☆
ERROR CODE
```

---

## 我的建议（结合你的项目）

**不要只做 AI 文档，而是做一套真正的软件工程文档。**

建议最终形成这四层：

```
L1：Architecture（总体架构，系统为什么这样设计）
L2：Detailed Design（详细设计，每个模块如何实现、接口、状态机、时序）
L3：PROJECT-RULES + ADR（产品决策、架构决策、不变量）
L4：AGENTS + .mjs（AI 如何工作、如何自动检查）
```

这样无论是普通模型、高级模型，还是以后新加入的开发者，都能基于同一套设计工作，而不是每次从代码里重新推断架构。