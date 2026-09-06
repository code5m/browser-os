//! M5-2 MCP 命令注册表 / 全局策略（首期切片，无 rmcp / 无 server runtime / 无网络监听）。
//!
//! 设计来源：A3 M5-2 主篇 §3~§8、A1 M5-2 卡 §4、A11 D46/D53。
//! 本模块是**纯注册表 + 纯策略**，不 spawn 进程、不起监听、不引入 rmcp。
//! 真正的 stdio 服务器与 rmcp 集成留待后续（B1/B7/B8 待 A0 裁决）。
//!
//! W3 硬停止守约：无 `rmcp` / 无 `tokio` / 无 `TcpListener` / 无 `npm` / 无新 Tauri 命令 /
//! 无 Agent 运行时。文件系统与 URL 守门**复用** `security_policy`（单一真源）。
//!
//! 机器守门：`scripts/check-mcp-policy.py`（M5-2.a 已落地）：
//!   - `MCP_CAPABILITY_DRIFT`：能力白名单单一真源（只在 `domain.rs` 定义一处）；
//!   - `MCP_FS_TOOL_PATH_POLICY`：注册表中 `touches_fs`/`returns_url` 项必须复用路径根 / URL 脱敏；
//!   - `MCP_*_PENDING`：rmcp/tokio/监听等红线，产物存在才判（现在已置真、零命中）。

use crate::domain::*;
use crate::security_policy::{check_path_within_roots, redact_sensitive_url};
use std::path::PathBuf;

/// 首期命令注册表（冻结）。每一项必须落在 `MCP_CAPABILITY_V1` 内，且 `touches_fs`/
/// `returns_url` 与策略守门一致。新增能力须同步改 `domain.rs` 的能力常量与本注册表，
/// 否则 `check-mcp-policy.py` 的奇偶 / 路径策略码位会判红。
pub const MCP_COMMAND_REGISTRY: &[McpCommandDef] = &[
    McpCommandDef {
        capability: "file_read",
        core_api: "workspace.read_file",
        touches_fs: true,
        returns_url: false,
    },
    McpCommandDef {
        capability: "file_list",
        core_api: "workspace.list_dir",
        touches_fs: true,
        returns_url: false,
    },
    McpCommandDef {
        capability: "tab_query",
        core_api: "session.tab_list",
        touches_fs: false,
        returns_url: true,
    },
    McpCommandDef {
        capability: "history_query",
        core_api: "session.list_sessions",
        touches_fs: false,
        returns_url: true,
    },
    McpCommandDef {
        capability: "bookmarks_query",
        core_api: "bookmarks.list",
        touches_fs: false,
        returns_url: false,
    },
    McpCommandDef {
        capability: "downloads_query",
        core_api: "downloads.list",
        touches_fs: false,
        returns_url: false,
    },
    McpCommandDef {
        capability: "console_query",
        core_api: "terminal.history",
        touches_fs: false,
        returns_url: false,
    },
];

/// 取某能力的注册项；未知能力 fail-closed 返回 None。
// 首期切片尚无 server 消费；M5-2.b 落地后由 rmcp handler 调用。
#[allow(dead_code)]
pub fn lookup_mcp_command(capability: &str) -> Option<&'static McpCommandDef> {
    MCP_COMMAND_REGISTRY
        .iter()
        .find(|c| c.capability == capability)
}

/// MCP 全局策略裁决（fail-closed：默认拒绝）。
///
/// 规则：
///   1. 能力不在白名单 ⇒ Deny；
///   2. 触碰文件系统但未通过 `check_path_within_roots` ⇒ Deny；
///   3. 其余 Allow（URL 脱敏是**调用方**在拼装返回前必须做的，由 `redact_mcp_url` 强制）。
// 首期切片尚无 server 消费；M5-2.b 落地后由 rmcp handler 调用。
#[allow(dead_code)]
pub fn evaluate_mcp_command(
    capability: &str,
    raw_path: Option<&str>,
    roots: &[PathBuf],
) -> McpDecision {
    let def = match lookup_mcp_command(capability) {
        Some(d) => d,
        None => return McpDecision::Deny,
    };
    if def.touches_fs {
        if let Some(p) = raw_path {
            if check_path_within_roots(p, roots).is_err() {
                return McpDecision::Deny;
            }
        }
    }
    McpDecision::Allow
}

/// URL 脱敏出口（MCP 回传前必须经过，与 M1-8 同源单一真源）。
#[allow(dead_code)]
pub fn redact_mcp_url(raw: &str) -> String {
    redact_sensitive_url(raw)
}

/// 构造当前策略快照（未来 `mcp_policy_get` 命令的返回值来源；首期仅冻结）。
#[allow(dead_code)]
pub fn current_policy_snapshot() -> McpPolicySnapshot {
    McpPolicySnapshot {
        capabilities: MCP_CAPABILITY_V1.iter().map(|s| s.to_string()).collect(),
        policy_version: "MCP_CAPABILITY_V1",
    }
}

#[cfg(test)]
mod mcp_policy_tests {
    use super::*;

    #[test]
    fn capability_whitelist_is_closed_and_fail_closed() {
        // 能力集合固定 7 项；其余一律拒绝。
        assert!(is_known_mcp_capability("file_read"));
        assert!(is_known_mcp_capability("console_query"));
        for bad in [
            "",
            "File_Read",
            "FILE_READ",
            "write_file",
            "exec",
            "mcp_server_start",
            "rmcp",
        ] {
            assert!(!is_known_mcp_capability(bad), "白名单外能力必须拒绝: {bad}");
        }
    }

    #[test]
    fn registry_entries_are_subset_of_capability_whitelist() {
        // 注册表每一项的能力必须在白名单内，且白名单每一项都有注册项（双向覆盖）。
        for entry in MCP_COMMAND_REGISTRY {
            assert!(
                MCP_CAPABILITY_V1.contains(&entry.capability),
                "注册表项 {cap} 不在能力白名单内",
                cap = entry.capability
            );
        }
        for cap in MCP_CAPABILITY_V1 {
            assert!(
                MCP_COMMAND_REGISTRY.iter().any(|e| e.capability == *cap),
                "能力 {cap} 缺少注册项",
                cap = cap
            );
        }
    }

    #[test]
    fn unknown_capability_is_denied() {
        assert_eq!(evaluate_mcp_command("exec", None, &[]), McpDecision::Deny);
    }

    #[test]
    fn fs_capability_outside_roots_is_denied() {
        let roots = vec![PathBuf::from("/workspace")];
        assert_eq!(
            evaluate_mcp_command("file_read", Some("/etc/passwd"), &roots),
            McpDecision::Deny,
            "越出路径根的文件读必须拒绝"
        );
    }

    #[test]
    fn fs_capability_inside_roots_is_allowed() {
        let root = std::env::temp_dir().join("mcp_fs_test");
        let _ = std::fs::create_dir_all(&root);
        let f = root.join("a.txt");
        let _ = std::fs::write(&f, "x");
        assert_eq!(
            evaluate_mcp_command("file_read", Some(f.to_str().unwrap()), &[root.clone()]),
            McpDecision::Allow
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn url_is_redacted_before_return() {
        let out = redact_mcp_url("https://u:p@h.example.com/t?token=abc#x=1");
        assert!(!out.contains("abc"));
        assert!(!out.contains("p@"));
    }
}
