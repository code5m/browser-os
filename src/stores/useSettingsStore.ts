import { defineStore } from "pinia";
import { ref, computed } from "vue";

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
};

const STORAGE_KEY = "browser-os-settings";

export const useSettingsStore = defineStore("settings", () => {
  const theme = ref<Theme>("light");
  const keymapScheme = ref<KeymapScheme>("vscode");
  const actionLabels = ACTION_LABELS;

  const currentKeymap = computed(() => KEYMAP_SCHEMES[keymapScheme.value]);

  function load() {
    try {
      const raw = localStorage.getItem(STORAGE_KEY);
      if (raw) {
        const j = JSON.parse(raw);
        if (j.theme) theme.value = j.theme;
        if (j.keymapScheme) keymapScheme.value = j.keymapScheme;
      }
    } catch {}
  }

  function save() {
    try {
      localStorage.setItem(
        STORAGE_KEY,
        JSON.stringify({ theme: theme.value, keymapScheme: keymapScheme.value })
      );
    } catch {}
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

  return {
    theme,
    keymapScheme,
    actionLabels,
    currentKeymap,
    setTheme,
    setKeymapScheme,
  };
});
