// One lazy Monaco distribution and feature registration shared by BrowserOS surfaces.
// Workers and package loading are shared; document state remains with each owner.
import {ensureEditorWorkers} from './monacoWorkerServices.mjs';
let loading;
export function loadMonacoEditor(){
  if(!loading)loading=(async()=>{
    await ensureEditorWorkers();
    const monaco=await import('monaco-editor/editor/editor.api.js');
    await Promise.all([import('monaco-editor/editor/contrib/codelens/browser/codeLensCache.js'),import('monaco-editor/editor/common/services/treeViewsDndService.js'),import('monaco-editor/editor/contrib/documentSymbols/browser/outlineModel.js')]);
    await Promise.all([import('monaco-editor/features/gotoLine/register.js'),import('monaco-editor/features/quickCommand/register.js')]);
    await Promise.all([import('monaco-editor/editor/contrib/find/browser/findController.js'),import('monaco-editor/editor/contrib/folding/browser/folding.js'),import('monaco-editor/editor/contrib/bracketMatching/browser/bracketMatching.js'),import('monaco-editor/editor/contrib/cursorUndo/browser/cursorUndo.js'),import('monaco-editor/editor/contrib/multicursor/browser/multicursor.js')]);
    await Promise.all([import('monaco-editor/editor/contrib/suggest/browser/suggestController.js'),import('monaco-editor/editor/contrib/hover/browser/hoverContribution.js'),import('monaco-editor/editor/contrib/gotoSymbol/browser/goToCommands.js'),import('monaco-editor/editor/contrib/rename/browser/rename.js'),import('monaco-editor/editor/contrib/format/browser/formatActions.js')]);
    await Promise.all([import('monaco-editor/languages/definitions/typescript/register.js'),import('monaco-editor/languages/definitions/javascript/register.js'),import('monaco-editor/languages/definitions/rust/register.js'),import('monaco-editor/languages/definitions/python/register.js'),import('monaco-editor/languages/definitions/sql/register.js'),import('monaco-editor/languages/definitions/markdown/register.js'),import('monaco-editor/languages/definitions/html/register.js'),import('monaco-editor/languages/definitions/css/register.js'),import('monaco-editor/languages/definitions/java/register.js'),import('monaco-editor/languages/definitions/shell/register.js')]);
    return monaco;
  })().catch(error=>{loading=undefined;throw error;});
  return loading;
}
