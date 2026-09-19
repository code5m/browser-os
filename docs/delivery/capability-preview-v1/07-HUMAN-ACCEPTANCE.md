# 07 — 人工验收表（明天用）

> 状态：**HUMAN_GUI_ACCEPTANCE = PENDING**
>
> 自动验证已完成的部分：构建、日志、进程、状态断言、各类门禁、能力清单生成。
> **界面观感、布局、交互体验必须由人来判断** —— Agent 没有可靠的视觉能力，不做任何"看起来没问题"的断言。

---

## 一、明天请重点确认（5 项）

| # | 验收项 | 期望 | 结果 |
|---|---|---|---|
| G-1 | **应用能正常启动** | 启动无报错，主界面正常出现 | ⬜ |
| G-2 | **原有功能都还在** | 浏览器、宫格、文件、终端、书签等原有功能均可正常使用 | ⬜ |
| G-3 | **书签行为无变化** | 书签的增删改查、面板开关与之前完全一致 | ⬜ |
| G-4 | **启动日志出现能力层信息** | 日志含 `[capability] bootstrap activated=true error=none` | ⬜ |
| G-5 | **界面无异常** | 无错位、无空白面板、无报错弹窗 | ⬜ |

---

## 二、如何查看能力层是否生效

启动后在日志中查找：

```text
[capability] bootstrap activated=true error=none
```

看到这行，说明能力运行时已成功登记并激活了"书签"能力。
若出现 `activated=false` 或 `error=...`，说明能力层初始化失败（**不影响原有功能**，因为已做降级处理）。

---

## 三、本次**不需要**您验证的（已自动验证）

```text
✅ 前端构建通过                    ✓ built in 4.96s
✅ 能力登记表校验                  13/13 自检 + 真实扫描 fail=0 warn=0
✅ 运行时行为断言                  RT-01..RT-15  15/15
✅ 书签试点集成断言                PLT-01..PLT-08  8/8
✅ 资源报告护栏                    RPT-01..RPT-08  8/8
✅ 语义治理回归                    Registry ALL_PASS / Closure 27/27 / Sensitive ALL_PASS
✅ Git 完整性                      fsck 无 missing/corrupt
```

---

## 四、验收后如何定标签

| 结果 | 动作 |
|---|---|
| 人工验收**通过** | 打 `capability-preview-v1-pass`（**正式验收标签**） |
| 人工验收**发现问题** | 不打正式标签；保留 `capability-preview-v1-code-pass` 作为代码完成点，修复后重新验收 |

> 今晚**不会**提前打 `capability-preview-v1-pass`。
> 只有 `capability-preview-v1-code-pass`（表示"代码与自动验证完成"）。
