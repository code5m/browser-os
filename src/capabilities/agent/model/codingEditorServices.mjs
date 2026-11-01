// Compatibility facade: BrowserOS Editor owns the shared Monaco Worker service.
// Existing Coding Workbench consumers keep their stable import path.
export { retainEditorServices, ensureEditorWorkers, ensureTypeScriptServices, syncOpenScriptModels } from '../../../shared/editor/monacoWorkerServices.mjs';
