/**
 * Cross-workbench document ownership and save serialization.
 *
 * This is a process-local coordination layer, never a replacement for the
 * native, version-checked atomic write. It intentionally stores only document
 * identities and dirty flags, never editor text, secrets, or filesystem data.
 *
 * Workspace overlay, inline editor and Coding Workbench each retain their
 * existing document model and native save contract. This registry rejects an
 * ambiguous same-file write and protects directory mutation; it does not
 * claim all editor models have been migrated to one canonical DocumentManager.
 */
export function documentKey(root, path) {
  if (typeof path !== 'string' || !path || path.includes('\0') ||
      typeof root !== 'string' || root.includes('\0')) {
    throw Error('DOCUMENT_PATH_INVALID');
  }
  if (!path.startsWith('/') && !root.startsWith('/')) throw Error('DOCUMENT_PATH_INVALID');
  const absolute = path.startsWith('/') ? path : root.replace(/\/+$/, '') + '/' + path;
  if (!absolute.startsWith('/') || absolute.includes('\\')) throw Error('DOCUMENT_PATH_INVALID');
  const segments = absolute.split('/').filter(Boolean);
  if (segments.some(p => p === '..')) throw Error('DOCUMENT_PATH_INVALID');
  return '/' + segments.filter(p => p !== '.').join('/');
}

export function createDocumentOwnership() {
  // owner => path => dirty. Keeping clean handles allows future conflict
  // notifications without retaining a second copy of file contents.
  const owners = new Map();
  const writes = new Map();
  const mutations = new Map();
  const contains = (base, path) => path === base || path.startsWith(base === '/' ? '/' : base + '/');
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
        if (dirty && (path === base || (isDirectory && path.startsWith(base === '/' ? '/' : base + '/')))) {
          return true;
        }
      }
    }
    return false;
  }
  function writingUnder(target, isDirectory = false) {
    const base = documentKey('', target);
    for (const path of writes.keys()) {
      if (path === base || (isDirectory && path.startsWith(base === '/' ? '/' : base + '/'))) {
        return true;
      }
    }
    return false;
  }
  function beginSave(owner, path) {
    const target = documentKey('', path);
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
  return { replace, conflicts, dirtyUnder, writingUnder, beginSave, beginMutation, snapshot };
}

// One owner across the main Vue application, but no persistence across browser
// contexts. Separate processes are protected by native CAS and path locks.
export const documentOwnership = createDocumentOwnership();
