"""Finalize the audited source revision, not a hidden CI-only transformation."""
import json
from pathlib import Path
p=Path('package.json'); data=json.loads(p.read_text()); data['scripts']['check:developer-studio']='node scripts/check-developer-studio.mjs && node scripts/check-pr35-regressions.mjs'
assert 'check:developer-studio' not in data['scripts']['check']
data['scripts']['check']+=' && npm run check:developer-studio'; p.write_text(json.dumps(data,indent=2,ensure_ascii=False)+'\n')
p=Path('vite.config.ts'); s=p.read_text(); assert s.count('cssCodeSplit: false,')==1
s=s.replace('cssCodeSplit: false,','cssCodeSplit: false,\n    // Build-only standards-aware CSS minification; source styles and selectors stay readable.\n    cssMinify: "lightningcss",')
p.write_text(s)
p=Path('.github/workflows/capability-v2-validation.yml'); s=p.read_text()
for name,ident,upload in [('Semantic dependency and bundle evidence','semantic_evidence','Upload semantic and bundle evidence'),('Resource governance and visual comparison','resource_evidence','Upload resource governance evidence')]:
    anchor='      - name: '+name+'\n'; assert s.count(anchor)==1;s=s.replace(anchor,anchor+'        id: '+ident+'\n')
    before='      - name: '+upload+"\n        if: always() && needs.impact.outputs.mode == 'full'"
    assert s.count(before)==1;s=s.replace(before,before+" && steps."+ident+".outcome != 'skipped'")
anchor='      - name: Full pre-merge gate\n';assert s.count(anchor)==1
s=s.replace(anchor,"      - name: Verify committed source is unchanged after validation\n        if: needs.impact.outputs.mode == 'full'\n        run: git diff --exit-code HEAD -- src src-tauri scripts docs package.json package-lock.json vite.config.ts .github\n"+anchor)
p.write_text(s)
p=Path('docs/engineering/governance.json'); data=json.loads(p.read_text()); before=len(data['workflows']);data['workflows']=[w for w in data['workflows'] if w['id']!='developer-studio-builder'];assert len(data['workflows'])==before-1;p.write_text(json.dumps(data,indent=2,ensure_ascii=False)+'\n')
p=Path('docs/engineering/DEVELOPER-STUDIO-UNIFIED-DELIVERY.md')
with p.open('a') as f:f.write('\n\n## PR35 audit repair consolidation (2026-10-16)\n\nThe seven previously runner-only integration changes are now ordinary committed source. The temporary source-mutating Builder and scripts/developer-studio are retired in this same commit. Final validation checks the committed source without rerunning prepare.py.\n\nAudit fixes: original-document table refresh, operation-state guards for selected Git commits and amend, acknowledged session cleanup, structured fatal driver errors, unknown commit/rollback outcomes and retired connections, SQLite Result error type, TableDataEditor registration. Regression coverage runs the real Vue/Pinia owner with native IO fixtures and native Rust tests exercise temporary Git repositories and SQLite.\n\nThe CSS minifier is build-only. The 795517-byte baseline and 25.2% limit are unchanged; a successful build is not proof of footprint compliance. Full Validation/pre-merge, real MySQL and packaged GUI results must be linked to the final head before acceptance. Workstation GUI is distinct and remains unverified while the device is offline. The broader IDEA parity scope is not certified by these audit repairs.\n')
# Remove the temporary executor and every old source-rewriting script.
import shutil
Path('.github/workflows/developer-studio-builder.yml').unlink()
shutil.rmtree('scripts/developer-studio')
print('PR35_SOURCE_CONSOLIDATED_BUILDER_RETIRED')
