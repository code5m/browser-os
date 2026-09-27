//! 跨能力共享的「输入校验」原语（体积上限守卫 + 校验报告），属
//! `SHARED_NATIVE_INFRASTRUCTURE`。
//!
//! 背景：Agent / Skill 定义解析前有统一的「超大 payload 拒绝」守卫，避免 DoS
//! （A4 W7 评审 R5-2 / F2）；解析结果统一以 `ValidationReport` 返回供前端消费。
//! 二者均被 agent + skill 两能力复用，故落在此处而非任一单一能力模块——
//! 否则会制造能力 → 能力的反向依赖。
//!
//! 迁移自 `src-tauri/src/bridge.rs`（Native Physical Boundary Matrix /
//! native-physical-batch-skill）。

/// Agent / Skill 定义为小结构，解析/校验前的输入体积上限（防超大 payload DoS）。
/// 256 KiB 已留足余量；超出在来源校验之后直接拒绝。
pub const AGENT_DEF_MAX_BYTES: usize = 256 * 1024;
pub const SKILL_DEF_MAX_BYTES: usize = 256 * 1024;

/// 输入体积上限守卫：超 `max` 直接拒绝解析（防超大 payload DoS）。
///
/// 纯函数，无副作用；被 agent / skill 两能力的 `*_inner` 解析 helper 复用。
pub fn reject_oversized_def(text: &str, max: usize, kind: &str) -> Result<(), String> {
    if text.len() > max {
        return Err(format!(
            "{kind} 定义超出大小上限（{} 字节），已拒绝解析",
            max
        ));
    }
    Ok(())
}

/// 校验报告（`agent_validate` / `skill_validate` 返回）。
///
/// 可序列化供前端消费；`valid` 与 `errors` 双字段表达「通过 / 失败原因列表」。
#[derive(Debug, Clone, serde::Serialize)]
pub struct ValidationReport {
    pub valid: bool,
    pub errors: Vec<String>,
}

impl ValidationReport {
    pub fn ok() -> Self {
        ValidationReport {
            valid: true,
            errors: Vec::new(),
        }
    }
    pub fn with_errors(errors: Vec<String>) -> Self {
        ValidationReport {
            valid: errors.is_empty(),
            errors,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reject_oversized_def_guard() {
        assert!(reject_oversized_def("small", AGENT_DEF_MAX_BYTES, "Agent").is_ok());
        let huge = "x".repeat(AGENT_DEF_MAX_BYTES + 1);
        let e = reject_oversized_def(&huge, AGENT_DEF_MAX_BYTES, "Agent").unwrap_err();
        assert!(
            e.contains("大小上限") || e.to_lowercase().contains("limit"),
            "got {e}"
        );
    }

    #[test]
    fn report_ok_and_err() {
        assert!(ValidationReport::ok().valid);
        assert!(!ValidationReport::with_errors(vec!["x".into()]).valid);
    }

    #[test]
    fn report_ok_has_empty_errors() {
        let r = ValidationReport::ok();
        assert!(r.valid);
        assert!(r.errors.is_empty());
    }

    #[test]
    fn report_with_errors_shape() {
        let r = ValidationReport::with_errors(vec!["e1".into(), "e2".into()]);
        assert!(!r.valid);
        assert_eq!(r.errors.len(), 2);
    }
}
