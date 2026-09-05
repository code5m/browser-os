use serde::{Deserialize, Serialize};

/// Logical rectangle in CSS pixels (DPI-independent).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct LogicalRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl LogicalRect {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.width <= 0.0 || self.height <= 0.0
    }
}

/// Unique identifier for a browser tab.
pub type TabId = String;

/// Configuration for creating a new tab.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTabOptions {
    pub id: TabId,
    pub url: String,
    pub rect: LogicalRect,
    #[serde(default = "default_visible")]
    pub visible: bool,
    #[serde(default)]
    pub auto_resize: bool,
    #[serde(default)]
    pub user_agent: Option<String>,
    #[serde(default)]
    pub transparent: bool,
    /// JavaScript injected into every page before any other script runs.
    #[serde(default)]
    pub initialization_script: Option<String>,
}

fn default_visible() -> bool {
    true
}

/// Events emitted by the plugin on the `browser-tabs://event` channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum BrowserTabEvent {
    NavigationStarted {
        id: TabId,
        url: String,
    },
    NavigationFinished {
        id: TabId,
        url: String,
    },
    TitleChanged {
        id: TabId,
        title: String,
    },
    FaviconChanged {
        id: TabId,
        favicon: String,
    },
    /// A page requested `window.open` / `target="_blank"`. The plugin always
    /// denies the new window and emits this event instead, so the host app
    /// can decide to open a new tab.
    NewWindowRequested {
        id: TabId,
        url: String,
    },
    /// 页面加载失败（网络/TLS 错误）。WebKit 内部错误页不注入用户脚本，
    /// 前端/注入脚本无法感知，必须由原生 load-failed 信号上报。
    LoadFailed {
        id: TabId,
        url: String,
        error: String,
    },
    /// 子资源加载完成或失败（M1-8，Linux WebKitGTK 原生信号
    /// resource-load-started + finished/failed）。字段只来自原生信号：
    /// status/mime/size_bytes 在无真实响应时为 None，绝不伪造。
    /// 注意：url 为页面上报的原始 URL，**未经脱敏**；主进程必须先经
    /// `redact_sensitive_url` 过滤再入库/转发前端，本事件不外泄到前端。
    ResourceReceived {
        id: TabId,
        url: String,
        method: Option<String>,
        status: Option<u32>,
        mime: Option<String>,
        size_bytes: Option<u64>,
        started_at: i64,
        finished_at: i64,
    },
    Closed {
        id: TabId,
    },
}

/// Error types for the plugin.
#[derive(Debug, thiserror::Error)]
pub enum BrowserTabError {
    #[error("tab not found: {0}")]
    TabNotFound(TabId),

    #[error("tab already exists: {0}")]
    TabAlreadyExists(TabId),

    #[error("invalid rect: {0:?}")]
    InvalidRect(LogicalRect),

    #[error("invalid url: {0}")]
    InvalidUrl(String),

    #[error("window not found")]
    WindowNotFound,

    #[error("platform error: {0}")]
    Platform(String),

    #[error("tauri error: {0}")]
    Tauri(#[from] tauri::Error),
}

/// Tauri commands require the error type to be serializable.
impl Serialize for BrowserTabError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}

pub type Result<T> = std::result::Result<T, BrowserTabError>;
