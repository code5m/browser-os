// BrowserOS-owned incremental synchronization policy for Monaco TS/JS workers.
// Pure policy: Monaco, browser workers, filesystem and native IPC stay outside.
const SCRIPT_LANGUAGES = ['typescript', 'javascript'];

export function createIncrementalScriptModelSync() {
  // Both TS and JS workers need every opened script URI for cross-language imports.
  const acknowledged = new Map(SCRIPT_LANGUAGES.map(language => [language, new Map()]));
  let epoch = 0;
  let tail = Promise.resolve();

  function invalidate() {
    ++epoch;
    for (const cache of acknowledged.values()) cache.clear();
  }

  async function perform(models, send) {
    const live = new Map();
    for (const model of models || []) {
      if (!model || model.isDisposed()) continue;
      const language = model.getLanguageId();
      if (!SCRIPT_LANGUAGES.includes(language)) continue;
      live.set(model.uri.toString(), {
        model, language, uri: model.uri, version: model.getVersionId()
      });
    }
    const activeLanguages = new Set([...live.values()].map(entry => entry.language));
    for (const [language, cache] of acknowledged) {
      // When its language disappears, the worker can be discarded by Monaco.
      if (!activeLanguages.has(language)) cache.clear();
      else for (const key of cache.keys()) if (!live.has(key)) cache.delete(key);
    }

    const startingEpoch = epoch;
    let submitted = 0, reused = 0;
    for (const language of SCRIPT_LANGUAGES) {
      if (!activeLanguages.has(language)) continue;
      const cache = acknowledged.get(language);
      const pending = [];
      for (const [key, entry] of live) {
        const previous = cache.get(key);
        if (previous?.model === entry.model && previous.language === entry.language && previous.version === entry.version) {
          ++reused;
        } else if (!entry.model.isDisposed() && entry.model.getVersionId() === entry.version &&
                   entry.model.getLanguageId() === entry.language) {
          pending.push({key, ...entry});
        }
      }
      if (!pending.length) continue;
      const completed = await send(language, pending.map(entry => entry.uri));
      if (completed === false) continue;
      submitted += pending.length;
      // Terminated workers or revisions edited while awaiting cannot be cached.
      if (epoch !== startingEpoch) continue;
      for (const entry of pending) {
        if (!entry.model.isDisposed() && entry.model.getVersionId() === entry.version &&
            entry.model.getLanguageId() === entry.language && entry.model.uri.toString() === entry.key) {
          cache.set(entry.key, entry);
        }
      }
    }
    return { eligible: live.size, submitted, reused };
  }

  function sync(models, send) {
    // Serialize concurrent tab switches; a rejected sync must not poison the queue.
    const next = tail.then(() => perform(models, send));
    tail = next.then(() => undefined, () => undefined);
    return next;
  }
  return { sync, invalidate };
}
