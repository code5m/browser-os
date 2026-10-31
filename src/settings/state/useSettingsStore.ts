import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { bridge } from "../../bridge";

export type KeymapScheme = "vscode" | "idea" | "eclipse";
export type Theme = "light" | "dark";

const KEYMAP_SCHEMES: Record<KeymapScheme, Record<string, string>> = {
  vscode: {
    newTab: "Ctrl+T",
    closeTab: "Ctrl+W",
    nextTab: "Ctrl+Tab",
    prevTab: "Ctrl+Shift+Tab",
    terminal: "Ctrl+`",
    grid: "Ctrl+Shift+G",
    home: "Ctrl+Shift+H",
    reload: "Ctrl+R",
    focusAddr: "Ctrl+L",
    recentlyClosed: "Ctrl+Shift+T",
  },
  idea: {
    newTab: "Ctrl+T",
    closeTab: "Ctrl+W",
    nextTab: "Alt+Right",
    prevTab: "Alt+Left",
    terminal: "Alt+F12",
    grid: "Ctrl+Shift+G",
    home: "Ctrl+Shift+H",
    reload: "Ctrl+F5",
    focusAddr: "Ctrl+L",
    recentlyClosed: "Ctrl+Shift+T",
  },
  eclipse: {
    newTab: "Ctrl+T",
    closeTab: "Ctrl+W",
    nextTab: "Ctrl+PageDown",
    prevTab: "Ctrl+PageUp",
    terminal: "Ctrl+Alt+T",
    grid: "Ctrl+Shift+G",
    home: "Ctrl+Shift+H",
    reload: "F5",
    focusAddr: "Ctrl+L",
    recentlyClosed: "Ctrl+Shift+T",
  },
};

const ACTION_LABELS: Record<string, string> = {
  newTab: "新建页签",
  closeTab: "关闭页签",
  nextTab: "下一个页签",
  prevTab: "上一个页签",
  terminal: "打开终端",
  grid: "打开宫格",
  home: "打开主页",
  reload: "刷新页面",
  focusAddr: "聚焦地址栏",
  recentlyClosed: "恢复最近关闭",
};

const STORAGE_KEY = "browser-os-settings";

export const useSettingsStore = defineStore("settings", () => {
  const theme = ref<Theme>("light");
  const keymapScheme = ref<KeymapScheme>("vscode");
  // 自适应资源休眠：默认关。开启后按内存压力缩短后台闲置阈值，
  // 保护正在编辑/播放/选择文件的页面，销毁闲置 WebView 与隐藏 Grid 子进程。
  const tabHibernation = ref(false);
  const actionLabels = ACTION_LABELS;

  const currentKeymap = computed(() => KEYMAP_SCHEMES[keymapScheme.value]);

  function load() {
    try {
      const raw = localStorage.getItem(STORAGE_KEY);
      if (raw) {
        const j = JSON.parse(raw);
        if (j.theme) theme.value = j.theme;
        if (j.keymapScheme) keymapScheme.value = j.keymapScheme;
        if (typeof j.tabHibernation === "boolean") tabHibernation.value = j.tabHibernation;
      }
    } catch {}
  }

  function save() {
    try {
      localStorage.setItem(
        STORAGE_KEY,
        JSON.stringify({
          theme: theme.value,
          keymapScheme: keymapScheme.value,
          tabHibernation: tabHibernation.value,
        })
      );
    } catch {}
  }

  function setTabHibernation(v: boolean) {
    tabHibernation.value = v;
    save();
    bridge.setTabHibernation(v).catch(() => {});
  }

  function applyTheme() {
    document.documentElement.dataset.theme = theme.value;
  }

  function setTheme(t: Theme) {
    theme.value = t;
    applyTheme();
    save();
  }

  function setKeymapScheme(s: KeymapScheme) {
    keymapScheme.value = s;
    save();
  }

  load();
  applyTheme();
  // 启动时把持久化的休眠开关同步给后端（后端默认关）
  if (tabHibernation.value) bridge.setTabHibernation(true).catch(() => {});

  return {
    theme,
    keymapScheme,
    tabHibernation,
    actionLabels,
    currentKeymap,
    setTheme,
    setKeymapScheme,
    setTabHibernation,
  };
});
