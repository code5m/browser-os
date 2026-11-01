//! Production adapter for the isolated BrowserOS Agent Channel contracts.
//! Existing Coding Agent remains the ONLY model loop and file writer.
//! A user-initiated coding chat permits proposals and scoped reads here; actual
//! writes still require the existing explicit review-and-apply path.
use std::path::Path;
use browseros_agent_channel::tools::{Approval,ToolCall,ToolCatalog,ToolRisk,ToolSpec};
use browseros_agent_transition::runtime::{Effect,RuntimeRequest,Workload};
use serde_json::json;
pub(crate) fn coding_catalog() -> Result<ToolCatalog,String> {
    let mut catalog=ToolCatalog::default();
    for (name,risk) in [
        ("list_files",ToolRisk::Read),("read_file",ToolRisk::Read),
        ("search_files",ToolRisk::Read),("list_skills",ToolRisk::Read),
        ("read_skill",ToolRisk::Read),("list_mcp_servers",ToolRisk::Read),
        ("search_knowledge",ToolRisk::Read),
        // A proposed edit is staged, not applied; the existing apply flow
        // requires separate user confirmation and source hash checks.
        ("write_file",ToolRisk::Write),("request_operation",ToolRisk::Write),
    ] {
        catalog.register(ToolSpec {
            name:name.to_string(),
            description:format!("Host-controlled coding tool: {name}"),
            input_schema:json!({"type":"object"}),
            risk,
        }).map_err(|_|"AGENT_CATALOG_INVALID".to_string())?;
    }
    Ok(catalog)
}
pub(crate) fn authorize_coding_proposal(
    catalog:&mut ToolCatalog,
    root:&Path,
    session_id:&str,
    call_id:&str,
    name:&str,
    is_plan:bool,
)->Result<(),String>{
    if call_id.trim().is_empty() || session_id.trim().is_empty(){
        return Err("AGENT_TOOL_CALL_INVALID".into());
    }
    if is_plan && (name=="write_file"||name=="request_operation") {
        return Err("AGENT_PLAN_READ_ONLY".into());
    }
    let id=format!("{session_id}/{call_id}");
    let scope=root.to_string_lossy().into_owned();
    let request=ToolCall{request_id:id.clone(),name:name.to_string(),scope:scope.clone(),arguments:json!({})};
    // This one-use host consent authorizes *invocation of a scoped tool or
    // creation of a proposal*, NOT an external effect. The caller has already
    // checked the trusted Tauri invoking WebView and authorized workspace.
    catalog.authorize(request,Approval::AllowOnce{
        request_id:id,name:name.to_string(),scope,
    }).map(|_|()).map_err(|_|"AGENT_TOOL_DENIED".into())
}
pub(crate) fn validate_coding_request(session_id:&str,task_id:&str,instruction:&str)
->Result<(),String>{
    RuntimeRequest{
        task_id:task_id.into(),session_id:session_id.into(),
        workload:Workload::Coding,effect:Effect::Infer,
        instruction:instruction.into(),contexts:vec![],requested_tool:None,
    }.validate().map_err(|_|"AGENT_REQUEST_INVALID".into())
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn approval_is_one_use_and_plan_is_read_only(){
        let mut cat=coding_catalog().unwrap();
        let root=Path::new("/test");
        assert!(authorize_coding_proposal(&mut cat,root,"s","t1","read_file",false).is_ok());
        assert!(authorize_coding_proposal(&mut cat,root,"s","t1","read_file",false).is_err());
        assert!(authorize_coding_proposal(&mut cat,root,"s","t2","write_file",true).is_err());
        assert!(authorize_coding_proposal(&mut cat,root,"s","t3","unknown",false).is_err());
        assert!(validate_coding_request("s","t","hello").is_ok());
    }
}
