#!/usr/bin/env node
import { readFileSync } from "node:fs";
import assert from "node:assert/strict";
const read=(p)=>readFileSync(new URL("../"+p,import.meta.url),"utf8");
const bar=read("src/components/layout/ActivityBar.vue"), bookmark=read("src/capabilities/bookmark/index.ts"),star=read("src/capabilities/bookmark/ui/BookmarkStar.vue"),nav=read("src/capabilities/grid/ui/GridNav.vue"),grid=read("src/capabilities/grid/ui/GridRows.vue"),files=read("src/capabilities/workspace/ui/FilePanel.vue"),tools=read("src/capabilities/tools/ui/ToolBox.vue"),layout=read("src/stores/useLayoutStore.ts");
function check(s,b){assert.ok(b,s);console.log("PASS "+s)}
check("四目的一级导航",["home","browser","files","tools"].every(v=>layout.includes('view: "'+v+'"')));
check("不再显示前往按钮",!bar.includes('>前往</button>'));
check("收藏夹单一导航贡献",!bar.includes('trailingActions') && star.includes('aria-label="收藏夹"'));
check("宫格单一入口",!nav.includes('caret-btn'));
check("地址栏建议最多六项",bar.includes(".slice(0,6)") && bar.includes('role="listbox"'));
check("工具页具备搜索和可操作模块",tools.includes('搜索工具') && tools.includes('workbench.open(id)'));
check("文件快捷栏只保留必要动作",files.includes('fileActionsOpen') && files.includes('更多文件操作'));
check("宫格高级功能上下文隔离",grid.includes('grid-advanced') && grid.includes('grid-context'));
check("不可用原生视图绝不增加覆盖浮层",!bar.includes("position: fixed") && !bar.includes("position:absolute"));
console.log("UI_PRODUCT_QUALITY=PASS");
