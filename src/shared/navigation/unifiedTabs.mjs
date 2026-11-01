// A single tab contract without a second stored copy of Browser or Layout state.
// Browser owns native WebView lifecycle; Layout owns module views. Everything else
// consumes this typed immutable projection and delegates intents to those owners.
export function projectUnifiedTabs(browserTabs,moduleTabs,mainView,activeWebId,activeModuleId){
 const web=browserTabs.map(t=>Object.freeze({
   key:'web:'+t.id,id:t.id,kind:'web',view:'browser',path:undefined,
   label:t.title||t.url,tooltip:t.url,icon:'🌐',
   active:mainView==='browser'&&activeWebId===t.id
 }));
 const modules=moduleTabs.map(t=>Object.freeze({
   key:'module:'+t.id,id:t.id,kind:'module',view:t.view,path:t.path,
   label:t.label,tooltip:t.path||t.label,icon:t.icon,
   active:mainView===t.view&&activeModuleId===t.id
 }));
 return Object.freeze([...web,...modules]);
}
