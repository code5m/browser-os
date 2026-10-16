import fs from 'node:fs';
import postcss from 'postcss';
const path='src/capabilities/database/ui/DatabasePanel.vue';let s=fs.readFileSync(path,'utf8');
for(const line of ['const copied = ref(false);','const page = ref(0);',"const filter = ref('');",'const sortColumn = ref(-1);','const sortAscending = ref(true);','const showHistory = ref(false);']){if(!s.includes(line))throw Error('Retired source differs: '+line);s=s.replace(line+'\n','');}
const a=s.indexOf('const pageSize = 50;'),b=s.indexOf('const fields = computed',a);if(a<0||b<a)throw Error('Retired rows boundary missing');s=s.slice(0,a)+s.slice(b);
const c=s.indexOf('const displayRows = computed'),d=s.indexOf('let stopIdeaProjectScan',c);if(c<0||d<c)throw Error('Retired display boundary missing');s=s.slice(0,c)+s.slice(d);
const e=s.indexOf('const sqlEditor = ref'),f=s.indexOf('</script>',e);if(e<0||f<e)throw Error('Retired editor boundary missing');s=s.slice(0,e)+s.slice(f);
s=s.replace('ref, watch }','ref }');
const dead=new Set(['confirm','confirm-text','danger','empty','grid-wrap','history-item','query','query-head','query-history','result','result-head','result-paging','sec-title','server-page','sql','sql-documents','sql-snippets','tag','warn','warn-list']);
const template=s.slice(s.indexOf('<template>'),s.indexOf('<style'));
for(const name of dead){if(new RegExp(`(?<![\\w-])${name}(?![\\w-])`).test(template))throw Error('Still referenced in template: '+name);}
let removed=0;
s=s.replace(/<style scoped>([\s\S]*?)<\/style>/g,(_,css)=>{
 const tree=postcss.parse(css);tree.walkRules(rule=>{const kept=rule.selectors.filter(sel=>![...sel.matchAll(/\.([a-zA-Z_][\w-]*)/g)].some(m=>dead.has(m[1])));if(!kept.length){removed++;rule.remove();}else if(kept.length!==rule.selectors.length){rule.selector=kept.join(', ');}});
 return '<style scoped>'+tree.toString()+'</style>';
});
fs.writeFileSync(path,s);console.log('Retired DatabasePanel rules removed:',removed);
