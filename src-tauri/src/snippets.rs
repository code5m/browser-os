//! M2-6.b 命令片段领域的纯函数层（无 IO、无 Tauri 依赖，全部可单测）。
//!
//! 契约：`logs/checkpoints/M2-6-20260905-1700.md` §7（M2-6.a 冻结）与
//! `logs/checkpoints/B-M2-6.a-command-snippet-contract-20260905-1710.md`。
//!
//! 范围：只做「**校验与可删除性判定**」，**不涉及执行**（执行接入归 M2-6.c）。
//!
//! 与 `scripts.rs` 的关系：参数模型、字段上限、参数名规则**全部复用** M2-3 的冻结口径
//! （`validate_param_name` / `MAX_*`），命令片段不另设一套，避免口径漂移；但错误类型是
//! 独立的 `SnippetError`（前端据此选择文案，与 `script.*` 错误码不混同）。
//!
//! 三条设计口径：
//!   1. **fail-closed**：定义期校验宁可误拒也不放行，坏片段不得进入 `snippets.json`；
//!   2. **整元素占位**（F2）：`argv` 中只有严格等于 `{NAME}` 的元素才被替换，
//!      「含 `{` 但不整元素」一律拒绝——部分插值会让一个参数值被词法切成多个
//!      argv 元素，重新开放注入面；
//!   3. **无正文文件**（F6）：命令片段没有 `path` 字段，故不存在路径校验分支，
//!      与 `scripts.rs` 的 `PathMismatch` / `BodyEmpty` 等分支无对应项。

use crate::domain::{CommandSnippet, ParamType, ScriptInterpreter, SNIPPET_MAX_TIMEOUT_SECS};
use crate::scripts::{
    validate_param_name, validate_script_id, MAX_CATEGORY_BYTES, MAX_DESCRIPTION_BYTES,
    MAX_LABEL_BYTES, MAX_NAME_BYTES, MAX_OPTIONS, MAX_PARAMS,
};

// ----------------------------- 上限常量 -----------------------------

/// 命令片段数量上限（与 `scripts::MAX_SCRIPTS` 同口径）
pub const MAX_SNIPPETS: usize = 200;
/// 单个片段的 argv 元素数上限
pub const MAX_ARGV_ELEMENTS: usize = 64;
/// 单个 argv 元素字节上限（与 `scripts::MAX_ARG_BYTES` 同口径）
pub const MAX_ARGV_ELEMENT_BYTES: usize = 4096;

// ----------------------------- 错误类型 -----------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnippetError {
    /// id 形态非法（`validate_script_id` 同源：会作为落盘与引用的键）
    InvalidId,
    /// 片段数量已达上限
    TooManySnippets,
    /// 参数数量超限
    TooManyParams,
    /// argv 元素数超限
    TooManyArgvElements,
    FieldTooLong {
        field: &'static str,
        limit: usize,
    },
    FieldEmpty {
        field: &'static str,
    },
    /// 参数名字符集非法
    InvalidParamName,
    /// 参数名重复
    DuplicateParamName,
    /// `Enum` 类型的 `options` 为空
    EnumOptionsEmpty,
    /// argv 为空（没有任何可执行内容）
    ArgvEmpty,
    /// argv 元素为空
    ArgvElementEmpty,
    /// argv 元素含「部分插值」：含 `{` 或 `}` 但不是整元素占位（F2 禁止）
    PartialInterpolation {
        element: String,
    },
    /// 占位符在 `params` 中无对应参数（执行时无法替换）
    UnknownPlaceholder {
        name: String,
    },
    /// `interpreter` 为 `Shebang`——命令片段**无正文文件**，shebang 与可执行位无从谈起
    ShebangNotSupported,
    /// **argv[0] 是占位符**（M2-6-fix1 / 复核 P1-2 最小收敛）。
    ///
    /// 程序名若来自运行期参数值，等于把「本次执行哪个程序」交给一次运行输入：
    /// 片段定义期静态可审计性归零，参数值绕过 `validate_param_value` 之后的
    /// 唯一残余影响面（选程序）也被放大。故 **argv[0] 必须是字面量**。
    /// 注意：这不是程序名白名单——`env`/`python3 -c`/`find -exec` 等仍可绕过任何
    /// 名字黑名单，本规则只关闭「程序名由运行期值决定」这一条路径。
    ArgvProgramPlaceholder,
    /// 超时超过 `SNIPPET_MAX_TIMEOUT_SECS`
    TimeoutTooLarge {
        limit: u32,
    },
    /// 内置片段不可删除（语义明确，不复用 `InvalidId`——
    /// M2-3 的 `scripts::can_delete` 复用 `InvalidId` 已登记为错误码语义不准）
    BuiltinImmutable,
}

impl SnippetError {
    /// 稳定错误码（前端据此选择文案，不得随意改名）
    pub fn code(&self) -> &'static str {
        match self {
            SnippetError::InvalidId => "INVALID_ID",
            SnippetError::TooManySnippets => "TOO_MANY_SNIPPETS",
            SnippetError::TooManyParams => "TOO_MANY_PARAMS",
            SnippetError::TooManyArgvElements => "TOO_MANY_ARGV_ELEMENTS",
            SnippetError::FieldTooLong { .. } => "FIELD_TOO_LONG",
            SnippetError::FieldEmpty { .. } => "FIELD_EMPTY",
            SnippetError::InvalidParamName => "INVALID_PARAM_NAME",
            SnippetError::DuplicateParamName => "DUPLICATE_PARAM_NAME",
            SnippetError::EnumOptionsEmpty => "ENUM_OPTIONS_EMPTY",
            SnippetError::ArgvEmpty => "ARGV_EMPTY",
            SnippetError::ArgvElementEmpty => "ARGV_ELEMENT_EMPTY",
            SnippetError::PartialInterpolation { .. } => "PARTIAL_INTERPOLATION",
            SnippetError::UnknownPlaceholder { .. } => "UNKNOWN_PLACEHOLDER",
            SnippetError::ShebangNotSupported => "SHEBANG_NOT_SUPPORTED",
            SnippetError::ArgvProgramPlaceholder => "ARGV_PROGRAM_PLACEHOLDER",
            SnippetError::TimeoutTooLarge { .. } => "TIMEOUT_TOO_LARGE",
            SnippetError::BuiltinImmutable => "BUILTIN_IMMUTABLE",
        }
    }
}

impl std::fmt::Display for SnippetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.code())
    }
}

// ----------------------------- 定义期校验 -----------------------------

/// 与 `scripts::check_field` 同口径（私有，两个域各持一份，避免为复用而放宽可见性）。
fn check_field(field: &'static str, value: &str, max: usize) -> Result<(), SnippetError> {
    if value.trim().is_empty() {
        return Err(SnippetError::FieldEmpty { field });
    }
    if value.len() > max {
        return Err(SnippetError::FieldTooLong { field, limit: max });
    }
    Ok(())
}

/// 命令片段定义期校验（**保存前**执行，fail-closed）。
///
/// 覆盖：id 形态、字段上限、参数名与唯一性、`Enum` options、
/// `Shebang` 拒绝、超时上限、argv 非空/元素非空/元素上限、
/// **整元素占位**（禁止部分插值）、占位符必须在 `params` 中有对应参数。
pub fn validate_snippet(s: &CommandSnippet) -> Result<(), SnippetError> {
    validate_script_id(&s.id).map_err(|_| SnippetError::InvalidId)?;
    check_field("name", &s.name, MAX_NAME_BYTES)?;
    check_field("category", &s.category, MAX_CATEGORY_BYTES)?;
    // description 允许为空（不是必填），但不得超长
    if s.description.len() > MAX_DESCRIPTION_BYTES {
        return Err(SnippetError::FieldTooLong {
            field: "description",
            limit: MAX_DESCRIPTION_BYTES,
        });
    }
    if s.params.len() > MAX_PARAMS {
        return Err(SnippetError::TooManyParams);
    }
    if matches!(s.interpreter, ScriptInterpreter::Shebang) {
        return Err(SnippetError::ShebangNotSupported);
    }
    if s.timeout_secs > SNIPPET_MAX_TIMEOUT_SECS {
        return Err(SnippetError::TimeoutTooLarge {
            limit: SNIPPET_MAX_TIMEOUT_SECS,
        });
    }

    // ---- 参数（复用 M2-3 冻结口径）----
    let mut seen: Vec<&str> = Vec::new();
    for p in &s.params {
        validate_param_name(&p.name).map_err(|_| SnippetError::InvalidParamName)?;
        check_field("label", &p.label, MAX_LABEL_BYTES)?;
        if seen.contains(&p.name.as_str()) {
            return Err(SnippetError::DuplicateParamName);
        }
        seen.push(p.name.as_str());

        if p.options.len() > MAX_OPTIONS {
            return Err(SnippetError::FieldTooLong {
                field: "options",
                limit: MAX_OPTIONS,
            });
        }
        if matches!(p.param_type, ParamType::Enum) && p.options.is_empty() {
            return Err(SnippetError::EnumOptionsEmpty);
        }
        // `raw` 在 argv 模式下无效果（M2-4.b-VERDICT §3.1），命令片段不依赖它，
        // 故不做 `InvalidRawParam` 判定——避免出现「校验通过但语义无效」的字段。
    }

    // ---- argv（F1 数组模型 + F2 整元素占位）----
    if s.argv.is_empty() {
        return Err(SnippetError::ArgvEmpty);
    }
    // argv[0] 必须是字面量程序名（M2-6-fix1 / 复核 P1-2 最小收敛）。
    // 允许 `["{PROG}", "--version"]` 等于让「执行哪个程序」由一次运行的输入决定，
    // 片段定义期将彻底失去静态可审计性；故在定义期 fail-closed 拒绝。
    if CommandSnippet::placeholder_of(&s.argv[0]).is_some() {
        return Err(SnippetError::ArgvProgramPlaceholder);
    }
    if s.argv.len() > MAX_ARGV_ELEMENTS {
        return Err(SnippetError::TooManyArgvElements);
    }
    for el in &s.argv {
        if el.trim().is_empty() {
            return Err(SnippetError::ArgvElementEmpty);
        }
        if el.len() > MAX_ARGV_ELEMENT_BYTES {
            return Err(SnippetError::FieldTooLong {
                field: "argv",
                limit: MAX_ARGV_ELEMENT_BYTES,
            });
        }
        // 只有「含花括号」的元素才需要占位判定：普通字面量（如 `grep`）直接放行。
        if el.contains('{') || el.contains('}') {
            match CommandSnippet::placeholder_of(el) {
                Some(name) => {
                    if !seen.contains(&name) {
                        return Err(SnippetError::UnknownPlaceholder {
                            name: name.to_string(),
                        });
                    }
                }
                // 含 `{` 但不是整元素占位 → 部分插值，拒绝
                None => {
                    return Err(SnippetError::PartialInterpolation {
                        element: el.clone(),
                    })
                }
            }
        }
    }

    Ok(())
}

/// 内置片段不可删除（与 `scripts::can_delete` 同职责，但错误码更准）。
pub fn can_delete(s: &CommandSnippet) -> Result<(), SnippetError> {
    if s.builtin {
        return Err(SnippetError::BuiltinImmutable);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ScriptParam;

    fn param(name: &str) -> ScriptParam {
        ScriptParam {
            name: name.to_string(),
            label: name.to_string(),
            param_type: ParamType::String,
            required: true,
            default: None,
            options: vec![],
            raw: false,
            secret: false,
        }
    }

    fn snippet(argv: Vec<&str>, params: Vec<ScriptParam>) -> CommandSnippet {
        let now = chrono::Utc::now();
        CommandSnippet {
            id: uuid::Uuid::new_v4().to_string(),
            name: "查找日志".to_string(),
            category: "text".to_string(),
            interpreter: ScriptInterpreter::Bash,
            argv: argv.into_iter().map(|s| s.to_string()).collect(),
            params,
            description: String::new(),
            dangerous: false,
            builtin: false,
            enabled: true,
            timeout_secs: 0,
            created_at: now,
            updated_at: now,
        }
    }

    // ---- s1：合法片段放行（含整元素占位）----
    #[test]
    fn s1_valid_snippet_with_placeholder() {
        let s = snippet(
            vec!["grep", "-r", "{PATTERN}", "{DIR}"],
            vec![param("PATTERN"), param("DIR")],
        );
        assert_eq!(validate_snippet(&s), Ok(()));
    }

    // ---- s2：argv 为空 → ArgvEmpty ----
    #[test]
    fn s2_argv_empty_rejected() {
        let s = snippet(vec![], vec![]);
        assert_eq!(validate_snippet(&s), Err(SnippetError::ArgvEmpty));
    }

    // ---- s3：部分插值（元素含占位但不等）→ PartialInterpolation ----
    #[test]
    fn s3_partial_interpolation_rejected() {
        let s = snippet(vec!["grep", "-r{PATTERN}"], vec![param("PATTERN")]);
        match validate_snippet(&s) {
            Err(SnippetError::PartialInterpolation { element }) => {
                assert_eq!(element, "-r{PATTERN}");
            }
            other => panic!("期望 PartialInterpolation，实得 {other:?}"),
        }
    }

    // ---- s4：占位符在 params 中无对应 → UnknownPlaceholder ----
    #[test]
    fn s4_unknown_placeholder_rejected() {
        let s = snippet(vec!["grep", "{MISSING}"], vec![param("PATTERN")]);
        match validate_snippet(&s) {
            Err(SnippetError::UnknownPlaceholder { name }) => assert_eq!(name, "MISSING"),
            other => panic!("期望 UnknownPlaceholder，实得 {other:?}"),
        }
    }

    // ---- s5：Shebang 不适用 → ShebangNotSupported ----
    #[test]
    fn s5_shebang_rejected() {
        let mut s = snippet(vec!["ls"], vec![]);
        s.interpreter = ScriptInterpreter::Shebang;
        assert_eq!(validate_snippet(&s), Err(SnippetError::ShebangNotSupported));
    }

    // ---- s6：超时超上限 → TimeoutTooLarge ----
    #[test]
    fn s6_timeout_too_large_rejected() {
        let mut s = snippet(vec!["ls"], vec![]);
        s.timeout_secs = SNIPPET_MAX_TIMEOUT_SECS + 1;
        assert_eq!(
            validate_snippet(&s),
            Err(SnippetError::TimeoutTooLarge {
                limit: SNIPPET_MAX_TIMEOUT_SECS
            })
        );
    }

    // ---- s7：内置不可删 ----
    #[test]
    fn s7_builtin_cannot_delete() {
        let mut s = snippet(vec!["ls"], vec![]);
        assert_eq!(can_delete(&s), Ok(()));
        s.builtin = true;
        assert_eq!(can_delete(&s), Err(SnippetError::BuiltinImmutable));
    }

    // ---- s8：参数名重复 ----
    #[test]
    fn s8_duplicate_param_name_rejected() {
        let s = snippet(vec!["grep", "{P}"], vec![param("P"), param("P")]);
        assert_eq!(validate_snippet(&s), Err(SnippetError::DuplicateParamName));
    }

    // ---- s9：Enum 无 options ----
    #[test]
    fn s9_enum_options_empty_rejected() {
        let mut p = param("LEVEL");
        p.param_type = ParamType::Enum;
        let s = snippet(vec!["echo", "{LEVEL}"], vec![p]);
        assert_eq!(validate_snippet(&s), Err(SnippetError::EnumOptionsEmpty));
    }

    // ---- s10：argv 元素为空 ----
    #[test]
    fn s10_argv_element_empty_rejected() {
        let s = snippet(vec!["grep", "   "], vec![]);
        assert_eq!(validate_snippet(&s), Err(SnippetError::ArgvElementEmpty));
    }

    // ---- s11：普通字面量（无花括号）不受占位规则影响 ----
    #[test]
    fn s11_plain_literals_pass() {
        let s = snippet(vec!["df", "-h", "--output=size"], vec![]);
        assert_eq!(validate_snippet(&s), Ok(()));
    }

    // ---- s13：argv[0] 为占位符 → ArgvProgramPlaceholder（M2-6-fix1 / P1-2）----
    // 程序名若来自运行期参数值，片段定义期的静态可审计性归零。
    #[test]
    fn s13_argv0_placeholder_rejected() {
        let s = snippet(vec!["{PROG}", "--version"], vec![param("PROG")]);
        assert_eq!(
            validate_snippet(&s),
            Err(SnippetError::ArgvProgramPlaceholder)
        );
        assert_eq!(
            SnippetError::ArgvProgramPlaceholder.code(),
            "ARGV_PROGRAM_PLACEHOLDER"
        );
    }

    // ---- s14：argv[0] 为字面量、后续元素为占位 → 放行（不误伤）----
    #[test]
    fn s14_literal_argv0_with_placeholder_args_passes() {
        let s = snippet(
            vec!["grep", "-r", "{PATTERN}", "{DIR}"],
            vec![param("PATTERN"), param("DIR")],
        );
        assert_eq!(validate_snippet(&s), Ok(()));
    }

    // ---- s12：整元素占位解析（placeholder_of 边界）----
    #[test]
    fn s12_placeholder_of_boundaries() {
        assert_eq!(CommandSnippet::placeholder_of("{DIR}"), Some("DIR"));
        assert_eq!(CommandSnippet::placeholder_of("{}"), None); // 空名
        assert_eq!(CommandSnippet::placeholder_of("{A-B}"), None); // 字符集越界
        assert_eq!(CommandSnippet::placeholder_of("x{DIR}"), None); // 部分插值
        assert_eq!(CommandSnippet::placeholder_of("grep"), None);
    }
}
