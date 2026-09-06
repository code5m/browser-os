//! M5-1.b 切片 1：抽取 Tauri 运行时耦合的三个薄抽象（seam）。
//!
//! 这些 trait 让 B 类模块（script_runner / workspace / sync / tasks / scheduler）
//! 在搬入 `core` 时能通过依赖注入消除「核心逻辑 → 命令层全局状态」的反向边
//! （见 `logs/assist/A2-M5-core-20260906-0749.md` §1.1 V-1/V-4/V-5/V-6 与
//! `logs/checkpoints/M5-20260906/M5-1.b-seam-trait-injection-and-b-extract.md`）。
//!
//! 约束：本文件零 Tauri 依赖、零行为。真实实现在二进制侧 `bridge.rs`
//! （`TauriProgressSink` / `TauriPathResolver` / `TauriRootsProvider`），
//! mock 实现见本文件测试。

use std::path::PathBuf;

use serde::Serialize;

/// 脚本运行进度事件载荷（脱 Tauri 核心侧抽象）。
///
/// 二进制侧 `TauriProgressSink` 以 `app.emit("script:progress", …)` 落地；
/// 此处仅定义载荷形态，不依赖任何 Tauri 类型。
#[derive(Debug, Clone, Serialize)]
pub struct Progress {
    pub run_id: String,
    pub phase: String,
    pub detail: Option<String>,
}

/// 进度推送抽象：替换 `script_runner` 对 `tauri::Emitter` 的硬依赖（V-6）。
pub trait ProgressSink {
    fn emit(&self, p: Progress);
}

/// 数据/基础目录解析抽象：替换 `tasks`/`workspace`/`sync` 对 `app.path().data_dir()` 的硬依赖（V-4/V-5）。
pub trait PathResolver {
    fn base_dir(&self) -> PathBuf;
}

/// 允许根目录集合抽象：替换 `scheduler` 对 `bridge::allowed_roots(app)` 的硬依赖（V-1）。
pub trait RootsProvider {
    fn allowed_roots(&self) -> Vec<PathBuf>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct NoopSink;
    impl ProgressSink for NoopSink {
        fn emit(&self, _p: Progress) {}
    }

    struct FixedResolver {
        dir: PathBuf,
    }
    impl PathResolver for FixedResolver {
        fn base_dir(&self) -> PathBuf {
            self.dir.clone()
        }
    }

    struct FixedRoots {
        roots: Vec<PathBuf>,
    }
    impl RootsProvider for FixedRoots {
        fn allowed_roots(&self) -> Vec<PathBuf> {
            self.roots.clone()
        }
    }

    #[test]
    fn progress_sink_is_object_safe() {
        // 切片 2 注入形态为 `&dyn ProgressSink`，这里守其对象安全性。
        let sink: &dyn ProgressSink = &NoopSink;
        sink.emit(Progress {
            run_id: "r1".into(),
            phase: "start".into(),
            detail: None,
        });
    }

    #[test]
    fn path_and_roots_seams_are_usable() {
        let r = FixedResolver {
            dir: PathBuf::from("/data"),
        };
        assert_eq!(r.base_dir(), PathBuf::from("/data"));

        let p = FixedRoots {
            roots: vec![PathBuf::from("/a"), PathBuf::from("/b")],
        };
        assert_eq!(
            p.allowed_roots(),
            vec![PathBuf::from("/a"), PathBuf::from("/b")]
        );
    }
}
