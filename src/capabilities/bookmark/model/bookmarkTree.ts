export interface BookmarkNode { id: string; title: string; url?: string; children: BookmarkNode[]; }
const barRoots = new Set(['书签栏', '收藏夹栏', 'Bookmarks bar', 'Bookmarks Bar', 'Favorites bar', 'bookmark_bar']);
/** Categories encode imported folder paths, including nested folders. */
export function bookmarkTree(items: {id:string;title:string;url:string;category?:string}[]): BookmarkNode[] {
  const roots: BookmarkNode[] = [];
  for (const item of items) {
    const parts = (item.category && item.category !== 'default' ? item.category.split('/') : []).filter(Boolean);
    if (barRoots.has(parts[0])) parts.shift();
    let children = roots, path = '';
    for (const title of parts) {
      path += '/' + title;
      let folder = children.find(node => !node.url && node.id === path);
      if (!folder) { folder = {id:path,title,children:[]}; children.push(folder); }
      children = folder.children;
    }
    children.push({id:item.id,title:item.title || item.url,url:item.url,children:[]});
  }
  return roots;
}
