import { bridge } from '../bridge';

export type AiScope = 'default'|'database'|'git'|'browser'|'workbench';
export type AiProfile = {id:string;name:string;model:string;base_url:string;kind:'chat'|'image'|'video'|'unsupported';source:string;credential_ref:string};
export type AiConfiguration = {profiles:AiProfile[];bindings:Record<string,string>;plugins:{name:string;source:string;transport:string;credential_refs:string[];status:string}[];import_sources:string[]};
export const aiService = {
  list: ():Promise<AiConfiguration> => bridge.aiService({action:'list'}),
  importLocal: () => bridge.aiService({action:'import'}),
  save: (profile:AiProfile,credential:string) => bridge.aiService({action:'save',profile,credential}),
  bind: (scope:AiScope,id:string) => bridge.aiService({action:'bind',scope,id}),
  remove: (id:string) => bridge.aiService({action:'remove',id}),
  chat: (scope:AiScope,message:string,id='',history:{role:'user'|'assistant';content:string}[]=[],requestId=''):Promise<{content:string;model:string}> => bridge.aiService({action:'chat',scope,message,id,history,request_id:requestId}),
  legacyAgentChat: (agentDefinition:unknown,message:string,requestId:string):Promise<{content:string;model:string;agent_id:string;tools_executed:boolean}> => bridge.aiService({action:'legacy_chat',scope:'default',agent_definition:agentDefinition,message,request_id:requestId}),
  cancelChat: (requestId:string):Promise<{cancelled:boolean}> => bridge.aiService({action:'cancel',scope:'default',request_id:requestId}),
};
