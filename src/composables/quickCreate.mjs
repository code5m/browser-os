/** Resolve shell shortcuts without consuming IME, repeats or other modifiers. */
export function quickCreateAction(event, view) {
  if (event.defaultPrevented || event.isComposing || event.repeat || !event.ctrlKey || event.altKey || event.metaKey || event.key.toLowerCase() !== 'n') return null;
  // Do not hijack native text editing (including Monaco and xterm) even when
  // the event bubbles through the main WebView to the global Shell handler.
  const target = event.target;
  if (target) {
    if (target.isContentEditable || ['INPUT','TEXTAREA','SELECT'].includes(target.tagName?.toUpperCase())) return null;
    if (target.closest?.('[role="textbox"], [role="combobox"], .monaco-editor, .xterm, [data-keyboard-shortcuts-owner="editor"]')) return null;
  }
  if (view === 'files') return event.shiftKey ? 'directory' : 'file';
  return event.shiftKey ? null : 'tab';
}
