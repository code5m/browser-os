/**
 * Update only this editor surface's own encrypted journal after native save.
 * A recovered source can still belong to another live process. Keep it until
 * an explicit catalog deletion binds the selected recovery id and revision.
 */
export async function persistDraftAfterSave({
  path,
  currentSessionId,
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

}
