/**
 * Cross-workbench document ownership and save serialization.
 *
 * This is a process-local coordination layer, never a replacement for the
 * native, version-checked atomic write. It intentionally stores only document
 * identities, dirty flags and path invalidations, never editor text or secrets.
 *
 * Workspace overlay, inline editor and Coding Workbench each retain their
 * existing document model and native save contract. This registry rejects an
 * ambiguous same-file write and protects directory mutation; it does not
 * claim all editor models have been migrated to one canonical DocumentManager.
 */
import { documentPathKey } from './documentPaths.mjs';

export const documentKey = documentPathKey;

export function createDocumentOwnership() {
  // owner => path => dirty. Keeping clean handles allows future conflict
  // notifications without retaining a second copy of file contents.
  const owners = new Map();
  const writes = new Map();
  const mutations = new Map();
  const invalidated = new Map();
  const invalidationListeners = new Map();
  const contains = (base, path) => path === base || path.startsWith(base.endsWith('/') ? base : base + '/');
  const overlaps = (a,b) => a.path === b.path || (a.isDirectory && contains(a.path,b.path)) || (b.isDirectory && contains(b.path,a.path));
  function replace(owner, documents) {
    if (!owner || !Array.isArray(documents)) throw Error('DOCUMENT_OWNER_INVALID');
    const next = new Map();
    for (const document of documents) {
      const path = documentKey(document.root || '', document.path);
      const dirty = !!document.dirty;
      next.set(path, next.get(path) || dirty);
    }
    if (next.size) owners.set(owner, next);
    else owners.delete(owner);
    const stale = invalidated.get(owner);
    if (stale) {
      for (const path of stale) if (!next.has(path)) stale.delete(path);
      if (!stale.size) invalidated.delete(owner);
    }
  }
  function conflicts(owner, path) {
    const target = documentKey('', path);
    const others = [];
    for (const [other, paths] of owners) {
      if (other !== owner && paths.get(target)) others.push(other);
    }
    return others;
  }
  function dirtyUnder(target, isDirectory = false) {
    const base = documentKey('', target);
    for (const paths of owners.values()) {
      for (const [path, dirty] of paths) {
        if (dirty && (path === base || (isDirectory && contains(base, path)))) {
          return true;
        }
      }
    }
    return false;
  }
  function writingUnder(target, isDirectory = false) {
    const base = documentKey('', target);
    for (const path of writes.keys()) {
      if (path === base || (isDirectory && contains(base, path))) {
        return true;
      }
    }
    return false;
  }
  // A successful native move/delete invalidates every Owner that has not
  // explicitly remapped its own documents. Keep metadata while a workbench is
  // unmounted; never let a retained clean buffer recreate its former path.
  function invalidatePath(path, isDirectory = false, updatedOwners = []) {
    const source = documentKey('', path);
    const handled = new Set(updatedOwners);
    for (const [owner, documents] of owners) {
      if (handled.has(owner)) continue;
      const stale = invalidated.get(owner) || new Set();
      let changed = false;
      for (const document of documents.keys()) {
        if (document === source || (isDirectory && contains(source, document))) {
          if (!stale.has(document)) changed = true;
          stale.add(document);
        }
      }
      if (!changed) continue;
      invalidated.set(owner, stale);
      try { invalidationListeners.get(owner)?.(); } catch { /* Write rejection remains authoritative. */ }
    }
  }
  function pathInvalidated(owner, path) {
    return invalidated.get(owner)?.has(documentKey('', path)) === true;
  }
  function onPathInvalidated(owner, listener) {
    invalidationListeners.set(owner, listener);
    return () => { if (invalidationListeners.get(owner) === listener) invalidationListeners.delete(owner); };
  }
  function beginSave(owner, path) {
    const target = documentKey('', path);
    if (invalidated.get(owner)?.has(target)) throw Error('DOCUMENT_PATH_CHANGED');
    if ([...mutations.values()].some(items => items.some(item => overlaps(item,{path:target,isDirectory:false})))) throw Error('DOCUMENT_PATH_MUTATION_IN_PROGRESS');
    if (conflicts(owner, target).length) throw Error('DOCUMENT_CONCURRENT_DIRTY_EDIT');
    if (writes.has(target)) throw Error('DOCUMENT_SAVE_IN_PROGRESS');
    const token = Symbol(target);
    writes.set(target, token);
    let released = false;
    return () => {
      if (released) return;
      released = true;
      if (writes.get(target) === token) writes.delete(target);
    };
  }
  function beginMutation(paths) {
    if (!Array.isArray(paths) || !paths.length) throw Error('DOCUMENT_PATH_INVALID');
    const items = paths.map(item => ({path:documentKey('',item.path),isDirectory:item.isDirectory === true}));
    if (items.some(item => dirtyUnder(item.path,item.isDirectory))) throw Error('DOCUMENT_CONCURRENT_DIRTY_EDIT');
    if (items.some(item => writingUnder(item.path,item.isDirectory))) throw Error('DOCUMENT_SAVE_IN_PROGRESS');
    if ([...mutations.values()].some(active => active.some(a => items.some(b => overlaps(a,b))))) throw Error('DOCUMENT_PATH_MUTATION_IN_PROGRESS');
    const token = Symbol('path-mutation');
    mutations.set(token,items);
    return () => { mutations.delete(token); };
  }
  function snapshot() {
    return [...owners].flatMap(([owner, paths]) => [...paths].map(([path, dirty]) => ({
      owner, path, dirty
    })));
  }
  return { replace, conflicts, dirtyUnder, writingUnder, beginSave, beginMutation, snapshot, invalidatePath, pathInvalidated, onPathInvalidated };
}

// One owner across the main Vue application, but no persistence across browser
// contexts. Separate processes are protected by native CAS and path locks.
export const documentOwnership = createDocumentOwnership();
