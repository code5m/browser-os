// Native shell adapter: deliberately outside framework-agnostic shared/ui.
import { isTauri } from '@tauri-apps/api/core';
import { Menu, type MenuOptions } from '@tauri-apps/api/menu';
import { LogicalPosition } from '@tauri-apps/api/dpi';

export interface NativeMenuEntry {
  text: string;
  enabled?: boolean;
  checked?: boolean;
  children?: NativeMenuEntry[];
  action?: () => void | Promise<void>;
}
export interface MenuAnchor { x: number; y: number; }
/** DOM client coordinates are logical pixels; Tauri performs native scale conversion. */
export function menuAnchor(element: Element): MenuAnchor {
  const rect=element.getBoundingClientRect();
  return {x:Math.max(0,Math.round(rect.left)),y:Math.max(0,Math.round(rect.bottom))};
}

/** Reuse the existing core:menu permission. Never creates windows or moves WebViews. */
export function createNativeMenu(onError: (error: unknown) => void) {
  let disposed=false, busy=false;
  function options(entries: NativeMenuEntry[]): NonNullable<MenuOptions['items']> {
    return entries.map(entry=>({
      // GTK uses & as its mnemonic marker. User bookmark titles are literal text.
      text:entry.text.replace(/&/g,'&&'), enabled:entry.enabled ?? true,
      ...(entry.children ? {items:options(entry.children)} : {
        ...(entry.checked === undefined ? {} : {checked:entry.checked}),
        action:()=>{
          if(disposed)return;
          return Promise.resolve().then(()=>entry.action?.()).catch(onError);
        },
      }),
    }));
  }
  return {
    async open(entries: NativeMenuEntry[], anchor: MenuAnchor): Promise<boolean> {
      if(disposed || busy)return false;
      busy=true;
      let menu:Menu|undefined;
      try {
        if(!isTauri())throw Error('原生菜单仅在桌面客户端可用');
        menu=await Menu.new({items:options(entries.length ? entries : [{text:'暂无内容',enabled:false}])});
        if(!disposed)await menu.popup(new LogicalPosition(anchor.x,anchor.y));
        return !disposed;
      } catch(error) { if(!disposed)onError(error); return false; }
      finally {
        // Linux popup resolves on selection/cancel, so the resource stays alive while visible.
        if(menu)await menu.close().catch(error=>{if(!disposed)onError(error);});
        busy=false;
      }
    },
    dispose(){disposed=true;},
  };
}
