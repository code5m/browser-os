/**
 * Lexical document identities and UI path operations.
 *
 * These helpers never authorize filesystem access. Original paths still go to
 * the native Owner, which checks allowed roots, handles, reparse points and CAS.
 * Windows identity follows the ordinary case-insensitive namespace. A native
 * case-sensitive directory can conservatively share a UI write lock; this is
 * not a claim to resolve hard links, short names or native file IDs.
 */
const INVALID = 'DOCUMENT_PATH_INVALID';
const DRIVE = /^[A-Za-z]:[\\/]/;
const VERBATIM_DISK = /^\\\\\?\\[A-Za-z]:\\/;
const RESERVED = /^(?:con|prn|aux|nul|com[1-9¹²³]|lpt[1-9¹²³])(?:\.|$)/i;

function text(value, allowEmpty = false) {
  if (typeof value !== 'string' || (!allowEmpty && !value) || /[\u0000-\u001f]/.test(value)) {
    throw Error(INVALID);
  }
  return value;
}

function parts(value, windows) {
  if (!windows && value.includes('\\')) throw Error(INVALID);
  const segments = value.split(windows ? /[\\/]/ : '/').filter(Boolean);
  for (const segment of segments) {
    if (segment === '..') throw Error(INVALID);
    if (windows && segment !== '.' &&
        (/[<>:"|?*]/.test(segment) || /[. ]$/.test(segment) || RESERVED.test(segment))) {
      throw Error(INVALID);
    }
  }
  return segments.filter(segment => segment !== '.');
}

function absolute(value) {
  text(value);
  // Native Windows canonicalize returns a verbatim DISK path. Only that exact
  // local-drive prefix is supported; UNC, device and NT namespaces stay closed.
  if (VERBATIM_DISK.test(value)) {
    if (value.includes('/')) throw Error(INVALID);
    return { windows: true, root: value.slice(0, 7), separator: '\\', segments: parts(value.slice(7), true) };
  }
  if (DRIVE.test(value)) {
    return { windows: true, root: value.slice(0, 3), separator: value[2], segments: parts(value.slice(3), true) };
  }
  if (value.startsWith('/') && !value.startsWith('//')) {
    return { windows: false, root: '/', separator: '/', segments: parts(value.slice(1), false) };
  }
  throw Error(INVALID);
}

function render(path, segments = path.segments) {
  return path.root + segments.join(path.separator);
}

function key(path) {
  if (!path.windows) return '/' + path.segments.join('/');
  const drive = path.root.slice(-3, -1).toLowerCase();
  return drive + '/' + path.segments.map(segment => segment.toLowerCase()).join('/');
}

function isAbsolute(value) {
  return DRIVE.test(value) || VERBATIM_DISK.test(value) || value.startsWith('/');
}

function relativeParts(value, windows) {
  text(value, true);
  // Root-relative paths, drive-relative C:foo and all alternate namespaces are
  // ambiguous here. A relative path never changes its supplied project root.
  if (isAbsolute(value) || value.startsWith('\\') || /^[A-Za-z]:/.test(value)) throw Error(INVALID);
  return parts(value, windows);
}

export function documentPathKey(root, path) {
  text(root, true);
  text(path);
  if (isAbsolute(path)) {
    const target = absolute(path);
    // /src/a is not a drive-qualified Windows path. Do not accidentally treat
    // a rooted Windows spelling as an unrelated POSIX document.
    if (root && absolute(root).windows !== target.windows) throw Error(INVALID);
    return key(target);
  }
  const base = absolute(root);
  return key({ ...base, segments: [...base.segments, ...relativeParts(path, base.windows)] });
}

export function sameDocumentPath(a, b) {
  if (!a || !b) return false;
  // Display predicates must not break an entire file tree because an entry is
  // outside the supported path grammar. Mutation/write entry points use the
  // strict documentPathKey and still reject that entry before any side effect.
  try { return documentPathKey('', a) === documentPathKey('', b); }
  catch { return false; }
}

export function containsDocumentPath(base, path) {
  if (!base || !path) return false;
  const parent = documentPathKey('', base);
  const target = documentPathKey('', path);
  return target === parent || target.startsWith(parent.endsWith('/') ? parent : parent + '/');
}

export function documentPathName(path) {
  if (!path) return '';
  const parsed = absolute(path);
  return parsed.segments.at(-1) || parsed.root;
}

export function documentPathParent(path) {
  const parsed = absolute(path);
  return render(parsed, parsed.segments.slice(0, -1));
}

export function joinDocumentPath(base, relative) {
  const parsed = absolute(base);
  return render(parsed, [...parsed.segments, ...relativeParts(relative, parsed.windows)]);
}

export function relativeDocumentPath(base, path) {
  if (!containsDocumentPath(base, path)) return null;
  const parent = absolute(base);
  const target = absolute(path);
  // Slice by component count, never by a case-folded string's length.
  return target.segments.slice(parent.segments.length).join('/');
}

export function replaceDocumentPathPrefix(path, from, to) {
  if (!path || !containsDocumentPath(from, path)) return path;
  const source = absolute(from);
  const target = absolute(to);
  if (source.windows !== target.windows) throw Error(INVALID);
  const current = absolute(path);
  return render(target, [...target.segments, ...current.segments.slice(source.segments.length)]);
}
