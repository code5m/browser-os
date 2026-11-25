import { bridge } from '../bridge';

export type AiScope = 'default'|'database'|'git'|'browser'|'workbench';
export type AiProfile = {id:string;name:string;model:string;base_url:string;kind:'chat'|'image'|'video'|'unsupported';source:string;credential_ref:string};
export type AiConfiguration = {profiles:AiProfile[];bindings:Record<string,string>;plugins:{name:string;source:string;transport:string;credential_refs:string[];status:string}[];import_sources:string[];credential_cleanup_pending?:boolean};
export type TextRunSummary = {
  id:string; kind:'chat'|'legacy_agent'|'local_chat'; phase:string; task_id:string; session_id:string;
  previous_run:string|null; successor:string|null; reserved_input_available:boolean;
  updated_at_ms:number; has_payload:boolean; has_result:boolean; forgotten:boolean;
  can_resume:boolean; can_continue:boolean;
};
export type TextRunRecord = TextRunSummary & {
  profile_id?:string|null; model?:string; archived?:boolean; messages:{role:'user'|'assistant';content:string}[];
};
export const aiService = {
  textRuns: (cursor=''):Promise<{runs:TextRunSummary[];next_cursor:string|null;unavailable:number}> =>
    bridge.aiService({action:'run_history',id:cursor}),
  restoreTextRun: (id:string):Promise<TextRunRecord> => bridge.aiService({action:'run_restore',id}),
  continueTextRun: (id:string,requestId:string,message=''):Promise<{content:string;model:string}> =>
    bridge.aiService({action:'run_continue',id,request_id:requestId,message,confirmed:true}),
  forgetTextRun: (id:string):Promise<{forgotten:boolean;archived:boolean}> => bridge.aiService({action:'run_forget',id,confirmed:true}),
  list: ():Promise<AiConfiguration> => bridge.aiService({action:'list'}),
  importLocal: ():Promise<{imported:number;total:number;plugin_credentials:number;plugins:number;total_plugins:number;sources:string[];skipped:string[];credential_cleanup_pending?:boolean}> => bridge.aiService({action:'import'}),
  save: (profile:AiProfile,credential:string):Promise<{saved:boolean;credential_cleanup_pending?:boolean}> => bridge.aiService({action:'save',profile,credential}),
  bind: (scope:AiScope,id:string) => bridge.aiService({action:'bind',scope,id}),
  remove: (id:string):Promise<{removed:boolean;credential_removed:boolean}> => bridge.aiService({action:'remove',id}),
  chat: (scope:AiScope,message:string,id='',history:{role:'user'|'assistant';content:string}[]=[],requestId=''):Promise<{content:string;model:string}> => bridge.aiService({action:'chat',scope,message,id,history,request_id:requestId}),
  legacyAgentChat: (agentDefinition:unknown,message:string,requestId:string):Promise<{content:string;model:string;agent_id:string;tools_executed:boolean}> => bridge.aiService({action:'legacy_chat',scope:'default',agent_definition:agentDefinition,message,request_id:requestId}),
  legacyAgentList: ():Promise<{id:string;version:string;display_name:string}[]> => bridge.aiService({action:'legacy_list'}),
  legacyAgentInstall: (definition:unknown,confirmed:boolean):Promise<{installed:boolean;id:string;changed:boolean}> => bridge.aiService({action:'legacy_install',agent_definition:definition,confirmed}),
  legacyAgentUninstall: (id:string,confirmed:boolean):Promise<{removed:boolean;id:string}> => bridge.aiService({action:'legacy_uninstall',id,confirmed}),
  legacyAgentRun: (id:string,message:string,requestId:string):Promise<{content:string;model:string;agent_id:string;tools_executed:boolean}> => bridge.aiService({action:'legacy_run',id,scope:'default',message,request_id:requestId}),
  cancelChat: (requestId:string):Promise<{cancelled:boolean}> => bridge.aiService({action:'cancel',scope:'default',request_id:requestId}),
};
