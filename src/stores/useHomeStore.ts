import { defineStore } from "pinia";
import { reactive, computed } from "vue";
import { bridge } from "../bridge";
import { useBrowserStore } from "./useBrowserStore";
import { useLayoutStore } from "./useLayoutStore";

// 主页快捷方式：网页（url）或系统应用（exec）
export interface HomeShortcut {
  id: string;
  type: "url" | "app";
  name: string;
  // type=url 时为网址；type=app 时为 .desktop 的 exec 命令
  target: string;
  icon: string; // emoji 或图标
}

const STORAGE_KEY = "browser-os-home-shortcuts";

// 默认快捷方式（首次使用 / 未配置时）
function defaultShortcuts(): HomeShortcut[] {
  return [
    { id: "d1", type: "url", name: "百度", target: "https://www.baidu.com", icon: "🔍" },
    { id: "d2", type: "url", name: "Kimi", target: "https://kimi.moonshot.cn", icon: "🌙" },
    { id: "d3", type: "url", name: "DeepSeek", target: "https://chat.deepseek.com", icon: "🐋" },
    { id: "d4", type: "url", name: "B站", target: "https://www.bilibili.com", icon: "📺" },
    { id: "d5", type: "url", name: "GitHub", target: "https://github.com", icon: "🐙" },
    { id: "d6", type: "url", name: "Gitee", target: "https://gitee.com", icon: "🐴" },
  ];
}

export const useHomeStore = defineStore("home", () => {
  const browser = useBrowserStore();
  const layout = useLayoutStore();

  const shortcuts = reactive<HomeShortcut[]>(load());
  const editing = reactive<{
    open: boolean;
    id: string; // 空串 = 新增
    type: "url" | "app";
    name: string;
    target: string;
    icon: string;
  }>({ open: false, id: "", type: "url", name: "", target: "", icon: "🔗" });

  function load(): HomeShortcut[] {
    try {
      const raw = localStorage.getItem(STORAGE_KEY);
      if (raw) {
        const arr = JSON.parse(raw);
        if (Array.isArray(arr) && arr.length) return arr;
      }
    } catch {}
    return defaultShortcuts();
  }
  function save() {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(shortcuts));
    } catch {}
  }

  const has = computed(() => shortcuts.length > 0);

  // 打开快捷方式：url → 内嵌浏览器；app → 启动系统应用
  async function open(s: HomeShortcut) {
    if (s.type === "url") {
      browser.url = s.target;
      layout.setView("browser");
      await browser.openBrowser();
    } else {
      try {
        await bridge.launchApp(s.target);
        layout.showToast("已启动: " + s.name);
      } catch (e: any) {
        layout.showToast("启动失败: " + (e?.message ?? e));
      }
    }
  }

  // ===== 编辑 =====
  function startAdd() {
    editing.open = true;
    editing.id = "";
    editing.type = "url";
    editing.name = "";
    editing.target = "";
    editing.icon = "🔗";
  }
  function startEdit(s: HomeShortcut) {
    editing.open = true;
    editing.id = s.id;
    editing.type = s.type;
    editing.name = s.name;
    editing.target = s.target;
    editing.icon = s.icon;
  }
  function cancelEdit() {
    editing.open = false;
    editing.id = "";
  }
  function saveEdit() {
    const name = editing.name.trim();
    const target = editing.target.trim();
    if (!name || !target) {
      layout.showToast("请填写名称与目标");
      return;
    }
    if (editing.id) {
      const s = shortcuts.find((x) => x.id === editing.id);
      if (s) {
        s.type = editing.type;
        s.name = name;
        s.target = target;
        s.icon = editing.icon || "🔗";
      }
    } else {
      shortcuts.push({
        id: Math.random().toString(36).slice(2),
        type: editing.type,
        name,
        target,
        icon: editing.icon || "🔗",
      });
    }
    save();
    editing.open = false;
    editing.id = "";
    layout.showToast("已保存快捷方式");
  }
  function remove(id: string) {
    const i = shortcuts.findIndex((x) => x.id === id);
    if (i >= 0) {
      shortcuts.splice(i, 1);
      save();
      layout.showToast("已删除");
    }
  }
  function resetDefault() {
    shortcuts.splice(0, shortcuts.length, ...defaultShortcuts());
    save();
    layout.showToast("已恢复默认");
  }

  return {
    shortcuts,
    editing,
    has,
    open,
    startAdd,
    startEdit,
    cancelEdit,
    saveEdit,
    remove,
    resetDefault,
  };
});
