#!/usr/bin/env node
import { readFileSync } from "node:fs";
import assert from "node:assert/strict";
const read=(p)=>readFileSync(new URL("../"+p,import.meta.url),"utf8");
const bar=read("src/components/layout/ActivityBar.vue"), bookmark=read("src/capabilities/bookmark/index.ts"),star=read("src/capabilities/bookmark/ui/BookmarkStar.vue"),nav=read("src/capabilities/grid/ui/GridNav.vue"),grid=read("src/capabilities/grid/ui/GridRows.vue"),files=read("src/capabilities/workspace/ui/FilePanel.vue"),tools=read("src/capabilities/tools/ui/ToolBox.vue"),layout=read("src/stores/useLayoutStore.ts");
function check(s,b){assert.ok(b,s);console.log("PASS "+s)}
check("四目的一级导航",["home","browser","files","tools"].every(v=>layout.includes('view: "'+v+'"')));
check("不再显示前往按钮",!bar.includes('>前往</button>'));
check("收藏夹单一导航贡献",!bar.includes('v-for="c in trailingActions"') && star.includes('aria-label="收藏夹"'));
check("宫格单一入口",!nav.includes('caret-btn'));
check("地址栏混合建议有界且来自唯一纯函数",bar.includes("filterOmnibox(") && bar.includes('role="listbox"') && read("src/utils/omniboxSuggestions.ts").includes(".slice(0,6)"));
check("工具页具备搜索和可操作模块",tools.includes('搜索工具') && tools.includes('openModule(id)'));
check("文件快捷栏只保留必要动作",files.includes('fileActionsOpen') && files.includes('更多文件操作'));
check("宫格高级功能上下文隔离",grid.includes('grid-advanced') && grid.includes('grid-context'));
check("原生浏览器与宫格永不使用 HTML 悬浮建议",/omniOverlayAllowed = computed\(\(\) =>\s*layout\.mainView !== "browser" && layout\.mainView !== "grid"/.test(bar)
  && bar.includes(":class=\"{ 'suggestion-floating': omniOverlayAllowed }\"")
  && !/position:\s*fixed/.test(bar));
check("地址建议悬浮仅在受控工作台",bar.includes(".suggestion-list.suggestion-floating { position: absolute;")
  && bar.includes("ref=\"omniAnchor\"") && bar.includes("ref=\"tbarEl\"")
  && bar.includes("anchor.left - bar.left") && bar.includes("anchor.bottom - bar.top"));
check("地址栏左右边界与窗口布局同步",bar.includes("Math.round(anchor.width)")
  && bar.includes("suggestionsObserver.observe(el)") && bar.includes("suggestionsObserver?.disconnect()")
  && bar.includes('window.addEventListener("resize", syncWidth)'));
console.log("UI_PRODUCT_QUALITY=PASS");
