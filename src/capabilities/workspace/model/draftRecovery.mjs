/**
 * Retire only the journals involved in a successful recovered-document save.
 * The recovered session is cleared only after the current process journal is
 * either cleared (clean document) or durably replaced (new unsaved edits).
 */
export async function persistDraftAfterSave({
  path,
  currentSessionId,
  recoveredSessionId,
  savedText,
  currentText,
  save,
  clear,
}) {
  if (typeof path !== 'string' || !path || typeof currentSessionId !== 'string' ||
      !currentSessionId || typeof save !== 'function' || typeof clear !== 'function') {
    throw new Error('WORKSPACE_DRAFT_SAVE_ARGUMENT_INVALID');
  }

  if (currentText === savedText) {
    await clear(path, currentSessionId);
  } else {
    await save(path, currentSessionId, savedText, currentText);
  }

  if (recoveredSessionId && recoveredSessionId !== currentSessionId) {
    await clear(path, recoveredSessionId);
  }
}
