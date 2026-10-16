#!/usr/bin/env node
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
const source = readFileSync(new URL('../src-tauri/src/capabilities/git/studio.rs', import.meta.url), 'utf8');
for (const command of ['git_studio_read','git_studio_preview','git_studio_confirm','git_studio_cancel']) assert(source.includes(`fn ${command}`));
for (const guard of ['GIT_STUDIO_CHANGED_SINCE_PREVIEW','GIT_STUDIO_PR_CHANGED','GIT_STUDIO_PREMERGE_NOT_EXECUTED','GIT_STUDIO_CONFIRMATION_REQUIRED','GIT_STUDIO_PATH_ESCAPE']) assert(source.includes(guard));
assert(!source.includes('"--force"'));
console.log('DEVELOPER_STUDIO_SOURCE_FIXTURES=PASS (native behavior requires cargo test studio)');
