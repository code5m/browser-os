import { defineStore } from 'pinia';
import { ref, watch } from 'vue';
import { useLayoutStore, MODULE_META, type MainView } from '../../../stores/useLayoutStore';
import { hostServices, type BrowserContextPort } from "../../../capability/platform/host-services";

export const useWorkbenchStore = defineStore('workbench', () => {
  const commandOpen = ref(false);
  const collapsed = ref(false);
  const layout = useLayoutStore();
  const browser = hostServices.require<BrowserContextPort>("browser-context");
  let snapshot = { sidebar: true, dock: false, ai: false };
  let previous: MainView = 'home';
  try { collapsed.value = JSON.parse(localStorage.getItem('workbench-layout-v1') || '{}').collapsed === true; } catch {}
  watch(collapsed, value => {
    try { localStorage.setItem('workbench-layout-v1', JSON.stringify({ version: 1, collapsed: value })); } catch {}
  });
  function open(view: string) {
    commandOpen.value = false;
    if (view === 'browser') layout.setView('browser');
    else if (view in MODULE_META) layout.openModule(view as MainView);
    if (view === 'grid') {
      void browser.activateGrid();
    }
  }
  function toggleTools() {
    if (!collapsed.value) {
      snapshot = { sidebar: layout.sidebarOpen, dock: layout.browserDockOpen, ai: browser.aiNavOpen };
      previous = layout.mainView;
      layout.sidebarOpen = false; layout.browserDockOpen = false; browser.setAiNavOpen(false);
      layout.navSection = '';
    } else {
      layout.sidebarOpen = snapshot.sidebar; layout.browserDockOpen = snapshot.dock; browser.setAiNavOpen(snapshot.ai);
    }
    collapsed.value = !collapsed.value;
  }
  function returnToTool() { open(previous); }
  return { commandOpen, collapsed, open, toggleTools, returnToTool };
});
