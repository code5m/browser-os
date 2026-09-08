//! 宫格 IPC 协议（Phase 2）：主进程 ↔ 宫格子进程 的 UDS 线协议。
//!
//! 传输：Unix Domain Socket + 行分隔 JSON（serde_json，无新依赖）。
//! 双向三种帧：Request（主→子，带 seq）/ Response（子→主，带 seq）/ Event（子→主，异步）。
//! 子进程死亡/重启时连接断开，主进程 reader 线程置断连标记并唤醒挂起请求。

use serde::{Deserialize, Serialize};

/// 绝对屏幕物理像素矩形（主进程已按主窗 inner_position + scale_factor 换算）。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct IpcRect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// 主进程 → 子进程 命令。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum GridCmd {
    /// 在子进程主窗内 add_child 创建宫格 webview（id 即 label，如 "grid-0"）。
    CreateTab {
        id: String,
        url: String,
    },
    /// 移动/缩放子进程窗口到绝对屏幕矩形（收到后若窗口隐藏则 show）。
    UpdateRect {
        id: String,
        rect: IpcRect,
    },
    /// 隐藏子进程整个窗口（切视图/失焦，等价旧方案的移出屏幕）。
    HideWindow {
        id: String,
    },
    /// 在宫格 webview 内执行 JS。
    Eval {
        id: String,
        js: String,
    },
    ReadReplies {
        id: String,
    },
    /// 宫格 webview 导航到 url。
    Navigate {
        id: String,
        url: String,
    },
    /// 设置宫格缩放（WebKitGTK zoom_level）。
    SetZoom {
        id: String,
        zoom: f64,
    },
    /// 销毁宫格 webview（子进程随后通常被 kill）。
    CloseTab {
        id: String,
    },
    Ping,
}

/// 线协议帧。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Wire {
    Request {
        seq: u64,
        cmd: GridCmd,
    },
    Response {
        seq: u64,
        ok: bool,
        err: Option<String>,
        #[serde(default)]
        data: Option<String>,
    },
    /// 子进程 → 主进程 异步事件（导航完成/新窗口请求/桥命令事件转发）。
    Event {
        name: String,
        payload: serde_json::Value,
    },
}

impl Wire {
    pub fn ok(seq: u64) -> Self {
        Wire::Response {
            seq,
            ok: true,
            err: None,
            data: None,
        }
    }
    pub fn err(seq: u64, e: impl Into<String>) -> Self {
        Wire::Response {
            seq,
            ok: false,
            err: Some(e.into()),
            data: None,
        }
    }
}

/// 写一帧（JSON + '\n'）。
pub fn write_wire<W: std::io::Write>(w: &mut W, msg: &Wire) -> std::io::Result<()> {
    let mut line = serde_json::to_vec(msg)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    line.push(b'\n');
    w.write_all(&line)?;
    w.flush()
}

/// 读一帧。Ok(None) = 对端关闭（EOF）。
pub fn read_wire<R: std::io::BufRead>(r: &mut R) -> std::io::Result<Option<Wire>> {
    let mut line = String::new();
    let n = r.read_line(&mut line)?;
    if n == 0 {
        return Ok(None);
    }
    let msg = serde_json::from_str(line.trim())
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    Ok(Some(msg))
}
