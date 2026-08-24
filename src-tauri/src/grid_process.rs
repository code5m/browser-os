//! 宫格子进程工厂（Phase 1）：主进程侧的宫格子进程生命周期管理。
//!
//! 形态②（每宫格独立 tauri::app 子进程，隔离最干净）：每个宫格 `grid-N` 由独立的
//! 子进程承载（argv `--grid-child N` 启动，见 main.rs::run_grid_child）。子进程崩溃
//! （SIGSEGV）只杀该子进程，主进程 waitpid 拿到退出码后可重启该格，实现"一格崩不
//! 影响其它格 + 主窗 + 页签"。
//!
//! Phase 1 范围：仅 spawn/get_or_spawn/restart/shutdown_all 骨架，不接 IPC（Phase 2）、
//! 不切换前端宫格流程（仍走主进程 add_child，Phase 2 接通 UDS 后才切）。

use std::collections::HashMap;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;

/// 单个宫格子进程句柄。
pub struct GridChildHandle {
    pub index: u32,
    pub child: Child,
}

/// 宫格子进程管理器：登记所有存活子进程，负责 spawn/重启/关闭。
#[derive(Default)]
pub struct GridProcessManager {
    children: Mutex<HashMap<u32, GridChildHandle>>,
}

impl GridProcessManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// spawn 一个新的宫格子进程（不检查是否已存在，调用方需先确保没有重复）。
    fn spawn(&self, index: u32) -> Result<(), String> {
        let exe = std::env::current_exe().map_err(|e| format!("获取当前可执行文件失败: {e}"))?;
        let child = Command::new(exe)
            .arg("--grid-child")
            .arg(index.to_string())
            // 继承父进程环境（WEBKIT_DISABLE_DMABUF_RENDERER / GDK_BACKEND 等在
            // main.rs 已 set_var，子进程经环境继承获得）；额外标记子进程索引供
            // crashlog 等按角色区分（Phase 3 使用）。
            .env("GRID_CHILD_INDEX", index.to_string())
            // 子进程 stdio 继承主进程（日志汇入同一 stderr，便于 crashlog 镜像）。
            // Phase 2 接 UDS 后会改为独立日志 + socket 通道。
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| format!("spawn 宫格子进程 grid-{index} 失败: {e}"))?;
        let pid = child.id();
        self.children
            .lock()
            .unwrap()
            .insert(index, GridChildHandle { index, child });
        eprintln!("[grid-manager] spawned grid-child-{} pid={}", index, pid);
        Ok(())
    }

    /// 获取或 spawn：若该 index 的子进程不存在则创建。返回是否为新创建。
    pub fn get_or_spawn(&self, index: u32) -> Result<bool, String> {
        if self.children.lock().unwrap().contains_key(&index) {
            return Ok(false);
        }
        self.spawn(index)?;
        Ok(true)
    }

    /// 重启指定宫格子进程：先 kill 旧的（若还在），再 spawn 新的。
    pub fn restart(&self, index: u32) -> Result<(), String> {
        // 先移除并 kill 旧句柄（若子进程已死，kill 会失败，忽略错误）。
        if let Some(mut h) = self.children.lock().unwrap().remove(&index) {
            let _ = h.child.kill();
            let _ = h.child.wait();
            eprintln!("[grid-manager] killed old grid-child-{}", index);
        }
        self.spawn(index)?;
        eprintln!("[grid-manager] restarted grid-child-{}", index);
        Ok(())
    }

    /// 检查指定宫格子进程是否还存活（try_wait 非阻塞）。
    /// 返回 Some(exit_code) 表示已退出（信号杀死时为 128+signal，如 SIGSEGV=139），None 表示仍在运行。
    pub fn check_exited(&self, index: u32) -> Option<i32> {
        let mut children = self.children.lock().unwrap();
        if let Some(h) = children.get_mut(&index) {
            match h.child.try_wait() {
                Ok(Some(status)) => {
                    let real = exit_code_of(&status);
                    eprintln!("[grid-manager] grid-child-{} exited code={}", index, real);
                    // 已退出则从表中移除，返回退出码。
                    children.remove(&index);
                    Some(real)
                }
                Ok(None) => None, // 仍在运行
                Err(e) => {
                    eprintln!("[grid-manager] grid-child-{} try_wait error: {e}", index);
                    None
                }
            }
        } else {
            None
        }
    }

    /// 当前存活子进程数量。
    pub fn count(&self) -> usize {
        self.children.lock().unwrap().len()
    }

    /// 关闭所有宫格子进程（主进程退出前调用）。
    pub fn shutdown_all(&self) {
        let mut children = self.children.lock().unwrap();
        for (index, h) in children.iter_mut() {
            let _ = h.child.kill();
            let _ = h.child.wait();
            eprintln!("[grid-manager] shutdown grid-child-{}", index);
        }
        children.clear();
    }
}

/// 从 ExitStatus 取退出码：正常退出用 code()，被信号杀死则 128+signal（如 SIGSEGV=139）。
fn exit_code_of(status: &std::process::ExitStatus) -> i32 {
    if let Some(code) = status.code() {
        return code;
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(sig) = status.signal() {
            return 128 + sig;
        }
    }
    -1
}
