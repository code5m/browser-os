//! Official RMCP SDK adapter. MCP tool metadata is untrusted; only an
//! explicit host allowlist can expose remote tools and ONLY as read-only.
use crate::{
    approval::ApprovalBinding,
    tool::{AgentTool, AuthorizedCall, ToolEffect, ToolFuture},
    FoundationError, Result,
};
use rmcp::{
    model::{CallToolRequestParams, CallToolResult, Tool},
    service::{RoleClient, RunningService},
};
use serde_json::Value;
use std::{collections::HashSet, sync::Arc};

/// Tool scopes come from BrowserOS host policy, never MCP server hints.
pub struct HostMcpPolicy {
    pub allowed_read_only: HashSet<String>,
}

impl HostMcpPolicy {
    /// Validate the server advertised the tool without trusting readOnlyHint.
    pub fn approved_request(
        &self,
        discovered: &[Tool],
        name: &str,
        args: Value,
    ) -> Result<CallToolRequestParams> {
        if !self.allowed_read_only.contains(name)
            || discovered.iter().filter(|tool| tool.name == name).count() != 1
        {
            return Err(FoundationError::PermissionDenied);
        }
        let Value::Object(map) = args else {
            return Err(FoundationError::InvalidInput(
                "mcp tool arguments must be object",
            ));
        };
        bounded_json(&map, 16_384,
            FoundationError::InvalidInput("mcp tool arguments too large"))?;
        Ok(CallToolRequestParams::new(name.to_owned()).with_arguments(map))
    }
}

type VerifyResource = dyn Fn(&ApprovalBinding, &Value) -> Result<()> + Send + Sync;

/// A single host-approved read-only tool. The host must provide a resource
/// verifier calling the actual current capability-owner resource revision.
/// NEVER register write/process/external MCP tools through this adapter.
pub struct RmcpReadOnlyTool {
    pub tool_name: String,
    pub client: Arc<RunningService<RoleClient, ()>>,
    pub verifier: Arc<VerifyResource>,
}

fn validate_mcp_binding(binding: &ApprovalBinding, args: &Value) -> Result<()> {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| FoundationError::PermissionDenied)?.as_millis();
    if now >= u128::from(binding.expires_at_unix_ms)
        || binding.principal_id.is_nil() || binding.workspace_id.is_nil()
        || binding.session_id.is_nil() {
        return Err(FoundationError::PermissionDenied);
    }
    if !args.is_object() {
        return Err(FoundationError::InvalidInput("mcp tool arguments invalid or too large"));
    }
    bounded_json(args, 16_384,
        FoundationError::InvalidInput("mcp tool arguments invalid or too large"))?;
    Ok(())
}

impl AgentTool for RmcpReadOnlyTool {
    fn name(&self) -> &str {
        &self.tool_name
    }
    fn effect(&self) -> ToolEffect {
        ToolEffect::ReadOnly
    }
    fn validate_binding(&self, binding: &ApprovalBinding, args: &Value) -> Result<()> {
        validate_mcp_binding(binding, args)?;
        (self.verifier)(binding, args)?;
        validate_mcp_binding(binding, args)
    }
    fn requires_authorized_call(&self) -> bool { true }
    fn call<'a>(&'a self, _arguments: Value) -> ToolFuture<'a> {
        Box::pin(async { Err(FoundationError::PermissionDenied) })
    }
    fn call_authorized<'a>(&'a self, call: AuthorizedCall) -> ToolFuture<'a> {
        Box::pin(async move {
            self.validate_binding(&call.binding, &call.arguments)?;
            let Value::Object(map) = call.arguments else {
                return Err(FoundationError::InvalidInput("mcp arguments"));
            };
            let request = CallToolRequestParams::new(self.tool_name.clone()).with_arguments(map);
            let response = self
                .client
                .call_tool(request)
                .await
                .map_err(|_| FoundationError::Tool("MCP_REMOTE_CALL_FAILED".into()))?;
            strict_result(response)
        })
    }
}

/// Bound local serialization before allocating an oversized duplicate of
/// SDK-decoded remote data. Transport-level limits remain provider/SDK-owned.
fn bounded_json<T: serde::Serialize>(value: &T, limit: usize, too_large: FoundationError) -> Result<Vec<u8>> {
    use std::io::{Error, ErrorKind, Write};
    struct Capped { bytes: Vec<u8>, limit: usize, overflow: bool }
    impl Write for Capped {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if bytes.len() > self.limit.saturating_sub(self.bytes.len()) {
                self.overflow = true;
                return Err(Error::new(ErrorKind::InvalidData, "MCP_JSON_LIMIT"));
            }
            self.bytes.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> { Ok(()) }
    }
    let mut sink = Capped { bytes: Vec::new(), limit, overflow: false };
    match serde_json::to_writer(&mut sink, value) {
        Ok(()) => Ok(sink.bytes),
        Err(_) if sink.overflow => Err(too_large),
        Err(error) => Err(FoundationError::Serialization(error.to_string())),
    }
}

/// Structured results are untrusted: never treat strings as permissions or
/// execute model-provided instructions. Bound size before returning to model.
pub fn strict_result(result: CallToolResult) -> Result<Value> {
    if result.is_error.unwrap_or(false) {
        return Err(FoundationError::Tool("MCP_TOOL_RETURNED_ERROR".into()));
    }
    let bytes = bounded_json(&result, 64 * 1024,
        FoundationError::Tool("MCP_TOOL_RESULT_TOO_LARGE".into()))?;
    serde_json::from_slice(&bytes).map_err(|e| FoundationError::Serialization(e.to_string()))
}

/// Validates untrusted discovery against a trusted allowlist; no execution.
pub fn names_for_host(discovered: &[Tool], allowlist: &HashSet<String>) -> Vec<String> {
    // Count only allowlisted names in one pass; a remote server cannot
    // induce quadratic duplicate scanning or allocate a set of all names.
    let mut seen = HashSet::new();
    let mut ambiguous = HashSet::new();
    for item in discovered {
        let name: &str = item.name.as_ref();
        if allowlist.contains(name) && !seen.insert(name) {
            ambiguous.insert(name);
        }
    }
    discovered.iter().filter(|item| {
        let name: &str = item.name.as_ref();
        allowlist.contains(name) && !ambiguous.contains(name)
    }).map(|item| item.name.to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::model::{
        CallToolResponse, ContentBlock, ListToolsResult, PaginatedRequestParams,
        ServerCapabilities, ServerConfig,
    };
    use rmcp::service::RequestContext;
    use rmcp::{ErrorData as McpError, RoleServer, ServerHandler, ServiceExt};
    use serde_json::Map;

    struct DemoServer;
    impl ServerHandler for DemoServer {
        fn get_info(&self) -> ServerConfig {
            ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
        }
        async fn list_tools(
            &self,
            _: Option<PaginatedRequestParams>,
            _: RequestContext<RoleServer>,
        ) -> std::result::Result<ListToolsResult, McpError> {
            Ok(ListToolsResult::with_all_items(vec![
                Tool::new("demo.inspect", "Read-only fake", Arc::new(Map::new())),
                Tool::new("danger.delete", "Untrusted fake", Arc::new(Map::new())),
            ]))
        }
        async fn call_tool(
            &self,
            request: CallToolRequestParams,
            _: RequestContext<RoleServer>,
        ) -> std::result::Result<CallToolResponse, McpError> {
            match request.name.as_ref() {
                "demo.inspect" => {
                    Ok(CallToolResult::success(vec![ContentBlock::text("safe")]).into())
                }
                _ => Err(McpError::invalid_params("tool denied", None)),
            }
        }
    }

    #[tokio::test]
    async fn official_rmcp_client_server_wire_and_allowlist() {
        let (server_io, client_io) = tokio::io::duplex(4096);
        let server = tokio::spawn(async move { DemoServer.serve(server_io).await });
        let mut client = ().serve(client_io).await.expect("SDK client initialization");
        let tools = client.list_tools(None).await.expect("SDK tools/list").tools;
        let policy = HostMcpPolicy {
            allowed_read_only: HashSet::from(["demo.inspect".into()]),
        };
        assert_eq!(
            names_for_host(&tools, &policy.allowed_read_only),
            vec!["demo.inspect"]
        );
        assert!(policy
            .approved_request(&tools, "danger.delete", Value::Object(Map::new()))
            .is_err());
        let request = policy
            .approved_request(&tools, "demo.inspect", Value::Object(Map::new()))
            .unwrap();
        let result = client.call_tool(request).await.expect("SDK tools/call");
        assert!(!result.is_error.unwrap_or(false));
        assert!(strict_result(result).is_ok());
        client.close().await.expect("SDK close");
        let service = server
            .await
            .expect("server join")
            .expect("server initialized");
        service.cancel().await.expect("server close");
    }

    #[test]
    fn remote_hint_is_not_enough_to_grant_access() {
        let mut tool = Tool::new("danger.exec", "Fake", Arc::new(Map::new()));
        tool.annotations = Some(rmcp::model::ToolAnnotations::new().read_only(true));
        let policy = HostMcpPolicy {
            allowed_read_only: HashSet::new(),
        };
        assert!(policy
            .approved_request(&[tool], "danger.exec", serde_json::json!({}))
            .is_err());
        assert!(strict_result(CallToolResult::error(vec![ContentBlock::text("error")])).is_err());
    }
}
