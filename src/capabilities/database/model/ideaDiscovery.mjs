/** Bounded, explicit IDEA project discovery over already-authorized workspace roots.
 * The browser UI never traverses HOME in the background and never reads
 * dataSources.local.xml, Keyrings, credential stores or ~/.config/JetBrains.
 * All reads are delegated to the native allowlisted file facade.
 */
const IGNORED = new Set(['.git','.cache','.local','.config','node_modules','target','dist','build','.gradle','.m2','venv','.venv','vendor','.npm','.rustup','.cargo','snap','Pictures','Videos','Music']);
const priority = name => /^(IdeaProjects|Projects|projects|Documents|workspace|workspaces|src|dev|code)$/i.test(name) ? 0 : 1;
export async function discoverIdeaProjects(roots, io, stopped = () => false) {
  const queue = [...new Set((roots || []).filter(p => typeof p === 'string' && p.startsWith('/') && p.indexOf(String.fromCharCode(0)) < 0))]
    .slice(0, 12).map(path => ({path:path.replace(/\\/$/, '') || '/', depth:0}));
  const seen = new Set(), results = [], errors = [];
  let scanned = 0;
  const MAX_DIRS = 180, MAX_DEPTH = 5, MAX_SOURCES = 128;
  while (queue.length && scanned < MAX_DIRS && results.length < MAX_SOURCES) {
    if (stopped()) break;
    const {path,depth} = queue.shift();
    if (!path || seen.has(path)) continue;
    seen.add(path); scanned++;
    try {
      const xml = await io.readFile(path + '/.idea/dataSources.xml');
      if (typeof xml === 'string' && xml.length <= 512*1024) {
        const parsed = io.parse(xml);
        for (const source of parsed) {
          if (results.length >= MAX_SOURCES) break;
          results.push({path, source});
        }
      }
    } catch (_) { /* Not a project; permission and missing files are expected. */ }
    if (depth >= MAX_DEPTH || stopped()) continue;
    try {
      const children = (await io.listDir(path)).filter(entry => entry && entry.is_dir &&
        !IGNORED.has(entry.name) && !entry.name.startsWith('.') &&
        typeof entry.path === 'string' && entry.path.startsWith(path.replace(/\\/$/, '') + '/'));
      children.sort((a,b)=>priority(a.name)-priority(b.name)||a.name.localeCompare(b.name));
      for (const child of children.slice(0, 80)) {
        if (queue.length + seen.size >= MAX_DIRS) break;
        queue.push({path:child.path,depth:depth+1});
      }
    } catch (_) {
      if (errors.length < 10) errors.push(path);
    }
  }
  return {sources:results,scanned,partial:queue.length>0 || stopped(),permissionErrors:errors.length};
}
