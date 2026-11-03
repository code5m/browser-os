/** Resolve shell shortcuts without consuming IME, repeats or other modifiers. */
export function quickCreateAction(event, view) {
  if (event.defaultPrevented || event.isComposing || event.repeat || !event.ctrlKey || event.altKey || event.metaKey || event.key.toLowerCase() !== 'n') return null;
  if (view === 'files') return event.shiftKey ? 'directory' : 'file';
  return event.shiftKey ? null : 'tab';
}
