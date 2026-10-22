import { createApp } from 'vue';
import { createPinia } from 'pinia';
import { bridge } from '../../src/bridge';
import { activateGitLifecycle } from '../../src/capabilities/git/lifecycle';
import GitStudio from '../../src/capabilities/git/ui/GitStudio.vue';

const now = Math.floor(Date.now() / 1000);
const commits = Array.from({ length: 18 }, (_, index) => ({
  oid: `${['c1c9c29a', 'bb827564', '84a70d21', '0c99a180', '286981d2', 'e12fd933'][index % 6]}${String(index).padStart(32, '0')}`.slice(0, 40),
  summary: ['fixes from merge', 'Polish Rebased Git layout', 'Add historical file editor', 'Align commit and shelf controls', 'Add three-way conflict editor', 'Harden Git preview boundary'][index % 6],
  author: ['detachhead', 'ainfinit', 'Codex'][index % 3],
  time: now - index * 86400,
  refs: index === 0 ? ['HEAD', 'master'] : index === 3 ? ['origin/master'] : [],
  parents: index === 17 ? [] : [`parent${index}`],
}));

const files = [
  { path: 'src/capabilities/git/ui/GitStudio.vue', index: 'M', worktree: 'M', staged: true, unstaged: true, conflicted: false },
  { path: 'src/capabilities/git/ui/GitCommitFileTree.vue', index: 'A', worktree: ' ', staged: true, unstaged: false, conflicted: false },
  { path: 'docs/design/rebased-visual-migration-matrix.md', index: ' ', worktree: 'M', staged: false, unstaged: true, conflicted: false },
  { path: 'tests/git-studio.visual.spec.ts', index: '?', worktree: '?', staged: false, unstaged: true, conflicted: false },
];

const changedFiles = [
  { status: 'M', path: 'src/capabilities/git/ui/GitStudio.vue' },
  { status: 'A', path: 'src/capabilities/git/ui/GitCommitFileTree.vue' },
  { status: 'M', path: 'docs/design/rebased-visual-migration-matrix.md' },
  { status: 'M', path: 'scripts/check-rebased-source-audit.mjs' },
  { status: 'A', path: 'src/capabilities/git/model/gitPreferences.ts' },
];

bridge.gitStudioRead = async (_source, input) => {
  switch (input.action) {
    case 'status':
      return {
        root: '/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3',
        branch: 'master',
        head: commits[0].oid,
        state: 'Clean',
        trusted: true,
        files,
        branches: [
          ['master', commits[0].oid, '[ahead 2]', 'refs/heads/master'],
          ['feature/git-ui', commits[2].oid, '', 'refs/heads/feature/git-ui'],
          ['origin/master', commits[3].oid, '', 'refs/remotes/origin/master'],
        ],
      };
    case 'log': return { commits, has_more: false };
    case 'commit_files': return { files: changedFiles, selected_parent: 'parent0' };
    case 'commit_file_content': return { content: '<script setup lang="ts">\nimport { computed, ref } from \'vue\';\n\nconst selectedCommit = ref(\'c1c9c29a\');\nconst visibleFiles = computed(() => []);\n</script>\n\n<template>\n  <section class="git-studio">Rebased Git workspace</section>\n</template>\n', available: true };
    case 'commit_file_diff': return { patch: '@@ -1,4 +1,6 @@\n <script setup lang="ts">\n-import { ref } from \'vue\';\n+import { computed, ref } from \'vue\';\n+const selectedCommit = ref(\'c1c9c29a\');\n </script>' };
    case 'diff': return { patch: '@@ -1,2 +1,3 @@\n-old line\n+new line\n+another line' };
    case 'file': return { content: 'new line\nanother line' };
    case 'stash': return { text: 'stash@{0}\tWIP on master\tRebased layout polish' };
    default: return {};
  }
};
bridge.gitStudioPreview = async () => ({ id: 'visual-preview', dangerous: false, title: 'Preview', summary: 'Visual fixture only' });
bridge.gitStudioConfirm = async () => ({ ok: true });

await activateGitLifecycle();
createApp(GitStudio, {
  path: '/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3',
  mode: 'history',
}).use(createPinia()).mount('#app');
