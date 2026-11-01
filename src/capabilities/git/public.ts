// Git 能力公共边界（Capability Library Expansion v1）
//
// Shell（App / RepoPanel）与其它能力经此消费 Git，而不直接 import 能力内部
// （src/stores/useGitStore.ts 的物理路径或 ui/ 内部组件）。本文件提供再导出与无状态只读适配器，
// 不持有也不创建任何状态（第二真源禁止）。
// 语义 owner（useGitStore）与物理路径解耦，由 Semantic Registry 的 owner_implementations locator 解析。
export { default as GitStudioSurface } from "./ui/GitStudio.vue";
export * from "./state/useGitStore"

// 写操作确认对话框：经公共边界暴露给 Shell（App.vue 全局挂载），
// 而非让 Shell 直连 src/capabilities/git/ui/ 内部（CB-02/06 禁止）。
export { default as GitWriteConfirmDialog } from "./ui/GitWriteConfirmDialog.vue"

// Stateless, read-only adapter; no store mutation and no second repository owner.
import { bridge } from "../../bridge";
import { isCapabilityActive } from "../../capability/runtimeSingleton";
export async function readCodingGitContext(root:string){
  if(!isCapabilityActive('git'))throw new Error('GIT_CAPABILITY_INACTIVE');
  const result=await bridge.gitLocalInspect(root);
  if(!isCapabilityActive('git'))throw new Error('GIT_CAPABILITY_INACTIVE');
  return {root:result.root,branch:result.branch,head:result.head,has_conflicts:result.has_conflicts,files:result.files.slice(0,100),truncated:result.truncated||result.files.length>100};
}

export const gitAgentAdapter=Object.freeze({id:'git',scope:'git',label:'Git Agent',readContext:(root:string)=>{if(!root.trim())throw Error('ROOT_REQUIRED');return readCodingGitContext(root);}});
