//! M2-3 脚本领域的纯函数层（无 IO、无 Tauri 依赖，全部可单测）。
//!
//! 契约冻结：`logs/checkpoints/M2-3.a-20260903-1604.md`。
//!
//! 范围：只做「校验与脱敏」，**不涉及执行**（`run_script` / 进程组 kill / 超时 /
//! 输出上限 / 并发互斥 / 运行记录全部归 M2-4）。
//!
//! 三条设计口径：
//!   1. **fail-closed**：参数值检查宁可误拒也不放行；但 T-scr-17 要求中文/空格/连字符
//!      等正常用法必须放行，故拒绝清单只针对**真的会改变命令语义**的字符。
//!   2. **不复用 `security_policy::check_shell_command`**：它校验的是「整条命令行」
//!      （`.desktop` Exec 启动链路依赖），语义、错误码与本卡的「单个参数值」不同，
//!      改它会波及既有链路。本文件的元字符集合以它为**下界**并扩展。
//!   3. **纯函数不承担 IO**：`Path` 类型的目录白名单校验（`check_path_within_roots`）
//!      由命令层在落盘/执行前完成，本层只做字符级拒绝（含 `..`）。

use crate::domain::{ParamType, ScriptMeta, ScriptParam};

// ----------------------------- 上限常量 -----------------------------

pub const MAX_SCRIPTS: usize = 200;
pub const MAX_PARAMS: usize = 20;
pub const MAX_OPTIONS: usize = 50;

pub const MAX_NAME_BYTES: usize = 128;
pub const MAX_CATEGORY_BYTES: usize = 64;
pub const MAX_DESCRIPTION_BYTES: usize = 1024;
pub const MAX_LABEL_BYTES: usize = 64;
pub const MAX_PARAM_NAME_BYTES: usize = 32;

/// 单个参数值上限（**按字节**判定，多字节字符下不得 panic）
pub const MAX_ARG_BYTES: usize = 4096;
/// 脚本正文上限：1 MiB。超出不落盘，避免把 MB 级文本反复搬进内存。
pub const MAX_BODY_BYTES: usize = 1024 * 1024;

/// 脱敏占位符
pub const REDACTED: &str = "***";

/// 参数名命中即视为敏感（包含匹配、大小写不敏感）的兜底词表。
/// 与 `security_policy::SENSITIVE_QUERY_KEYS`（精确匹配 21 键）互补。
const SECRET_NAME_HINTS: [&str; 5] = ["pass", "secret", "token", "key", "apikey"];

// ----------------------------- 错误类型 -----------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptError {
    /// id 形态非法（会拼进文件名）
    InvalidId,
    /// 脚本数量已达上限
    TooManyScripts,
    /// 参数数量超限
    TooManyParams,
    /// 文本字段为空或超长
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
    /// `raw = true` 只允许出现在 `Path` / `Enum`
    InvalidRawParam,
    /// `path` 与 `id` / `interpreter` 推导出的文件名不一致
    PathMismatch,
    /// 正文为空或超长
    BodyEmpty,
    BodyTooLarge {
        limit: usize,
    },
    /// 命令替换：`` ` ``、`$(`、`<(`、`>(`
    DangerousSubshell,
    /// 链接/管道：`;`、`&`、`|`
    DangerousChain,
    /// 重定向：`>`、`<`
    DangerousRedirect,
    /// 通配符：`*`、`?`、`[`
    DangerousGlob,
    /// 换行 / 回车 / NUL
    DangerousNewline,
    /// 变量注入：`${` 或 `$` 后跟标识符 / `(`
    DangerousVarExpansion,
    /// 其余控制字符（0x01–0x1F、0x7F）
    DangerousControlChar,
    /// 路径穿越：`..`
    PathEscape,
    /// 参数值超长
    ArgTooLong {
        limit: usize,
    },
    /// 类型不匹配（Int 不可解析 / Bool 不是 true-false）
    ParamTypeMismatch {
        expected: &'static str,
    },
    /// Enum 取值不在 options 内
    ParamNotInOptions,
    /// 必填参数为空
    ParamRequired {
        name: String,
    },
}

impl ScriptError {
    /// 稳定错误码（前端据此选择文案，不得随意改名）
    pub fn code(&self) -> &'static str {
        match self {
            ScriptError::InvalidId => "INVALID_ID",
            ScriptError::TooManyScripts => "TOO_MANY_SCRIPTS",
            ScriptError::TooManyParams => "TOO_MANY_PARAMS",
            ScriptError::FieldTooLong { .. } => "FIELD_TOO_LONG",
            ScriptError::FieldEmpty { .. } => "FIELD_EMPTY",
            ScriptError::InvalidParamName => "INVALID_PARAM_NAME",
            ScriptError::DuplicateParamName => "DUPLICATE_PARAM_NAME",
            ScriptError::EnumOptionsEmpty => "ENUM_OPTIONS_EMPTY",
            ScriptError::InvalidRawParam => "INVALID_RAW_PARAM",
            ScriptError::PathMismatch => "PATH_MISMATCH",
            ScriptError::BodyEmpty => "BODY_EMPTY",
            ScriptError::BodyTooLarge { .. } => "BODY_TOO_LARGE",
            ScriptError::DangerousSubshell => "DANGEROUS_SUBSHELL",
            ScriptError::DangerousChain => "DANGEROUS_CHAIN",
            ScriptError::DangerousRedirect => "DANGEROUS_REDIRECT",
            ScriptError::DangerousGlob => "DANGEROUS_GLOB",
            ScriptError::DangerousNewline => "DANGEROUS_NEWLINE",
            ScriptError::DangerousVarExpansion => "DANGEROUS_VAREXP",
            ScriptError::DangerousControlChar => "DANGEROUS_CONTROL_CHAR",
            ScriptError::PathEscape => "PATH_ESCAPE",
            ScriptError::ArgTooLong { .. } => "ARG_TOO_LONG",
            ScriptError::ParamTypeMismatch { .. } => "PARAM_TYPE_MISMATCH",
            ScriptError::ParamNotInOptions => "PARAM_NOT_IN_OPTIONS",
            ScriptError::ParamRequired { .. } => "PARAM_REQUIRED",
        }
    }
}

impl std::fmt::Display for ScriptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.code())
    }
}

// ----------------------------- 定义期校验 -----------------------------

/// 脚本 id 形态（会作为文件名主干，与图片 id 共用同一套规则）。
pub fn validate_script_id(id: &str) -> Result<(), ScriptError> {
    // 与 `images::validate_id` 同源：都是「落盘文件名主干」的形态约束。
    crate::images::validate_id(id).map_err(|_| ScriptError::InvalidId)
}

fn check_field(field: &'static str, value: &str, max: usize) -> Result<(), ScriptError> {
    if value.trim().is_empty() {
        return Err(ScriptError::FieldEmpty { field });
    }
    if value.len() > max {
        return Err(ScriptError::FieldTooLong { field, limit: max });
    }
    Ok(())
}

/// 参数名：字符集 `[A-Za-z0-9_]`，非空，≤ 32 字符。
pub fn validate_param_name(name: &str) -> Result<(), ScriptError> {
    if name.is_empty() || name.len() > MAX_PARAM_NAME_BYTES {
        return Err(ScriptError::InvalidParamName);
    }
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(ScriptError::InvalidParamName);
    }
    Ok(())
}

/// 元数据定义期校验（**保存前**执行，fail-closed）。
///
/// 覆盖：字段上限、参数数量、options、`raw` 合法性、`Enum` options 非空、
/// 参数名唯一、`path` 与 `id`+`interpreter` 推导结果一致。
pub fn validate_meta(meta: &ScriptMeta) -> Result<(), ScriptError> {
    validate_script_id(&meta.id)?;
    check_field("name", &meta.name, MAX_NAME_BYTES)?;
    check_field("category", &meta.category, MAX_CATEGORY_BYTES)?;
    // description 允许为空（不是必填），但不得超长
    if meta.description.len() > MAX_DESCRIPTION_BYTES {
        return Err(ScriptError::FieldTooLong {
            field: "description",
            limit: MAX_DESCRIPTION_BYTES,
        });
    }
    if meta.params.len() > MAX_PARAMS {
        return Err(ScriptError::TooManyParams);
    }

    let mut seen: Vec<&str> = Vec::new();
    for p in &meta.params {
        validate_param_name(&p.name)?;
        check_field("label", &p.label, MAX_LABEL_BYTES)?;
        if seen.contains(&p.name.as_str()) {
            return Err(ScriptError::DuplicateParamName);
        }
        seen.push(p.name.as_str());

        if p.options.len() > MAX_OPTIONS {
            return Err(ScriptError::FieldTooLong {
                field: "options",
                limit: MAX_OPTIONS,
            });
        }
        match p.param_type {
            ParamType::Enum => {
                if p.options.is_empty() {
                    return Err(ScriptError::EnumOptionsEmpty);
                }
            }
            ParamType::String | ParamType::Int | ParamType::Bool => {
                // raw 意味着跳过单引号包裹，只允许出现在取值域可控的类型上
                if p.raw {
                    return Err(ScriptError::InvalidRawParam);
                }
            }
            ParamType::Path => {}
        }
        // 默认值同样要过危险字符检查：默认值会被 UI 预填，等于用户实际提交的值
        if let Some(d) = &p.default {
            validate_param_value(d, p.param_type)?;
        }
    }

    let expected = format!("{}.{}", meta.id, meta.interpreter.ext());
    if meta.path != expected {
        return Err(ScriptError::PathMismatch);
    }
    Ok(())
}

/// 正文校验：非空、不超 1 MiB。
pub fn validate_body(body: &str) -> Result<(), ScriptError> {
    if body.trim().is_empty() {
        return Err(ScriptError::BodyEmpty);
    }
    if body.len() > MAX_BODY_BYTES {
        return Err(ScriptError::BodyTooLarge {
            limit: MAX_BODY_BYTES,
        });
    }
    Ok(())
}

/// 是否允许删除（内置脚本不可删除，命令层在改元数据之前调用）。
pub fn can_delete(meta: &ScriptMeta) -> Result<(), ScriptError> {
    if meta.builtin {
        return Err(ScriptError::InvalidId);
    }
    Ok(())
}

// ----------------------------- 运行期参数值校验 -----------------------------

/// `$` 后是否是「变量展开」的起手（标识符字符或 `(`），用于区分 `$5`、`$` 结尾等。
fn is_var_expansion_at(chars: &[char], i: usize) -> bool {
    match chars.get(i + 1) {
        Some(c) => c.is_ascii_alphanumeric() || *c == '_' || *c == '(',
        None => false,
    }
}

/// 参数值校验（**纯函数、fail-closed**）。
///
/// 检查顺序即契约顺序（先具体后宽泛，保证错误码稳定）：
/// 换行/NUL → 其余控制字符 → 超长 → `..` → 命令替换 → 变量展开
/// → 链接管道 → 重定向 → 通配符 → 类型专属校验。
///
/// `Path` 类型的**目录白名单**校验不在本函数（需要 IO），由命令层用
/// `security_policy::check_path_within_roots` 在落盘/执行前完成。
pub fn validate_param_value(value: &str, param_type: ParamType) -> Result<(), ScriptError> {
    // 1) 换行 / 回车 / NUL：最常被用来截断命令
    if value.chars().any(|c| matches!(c, '\n' | '\r' | '\0')) {
        return Err(ScriptError::DangerousNewline);
    }
    // 2) 其余控制字符（含 DEL 与 C1）
    if value.chars().any(|c| c.is_control()) {
        return Err(ScriptError::DangerousControlChar);
    }
    // 3) 超长（按字节）
    if value.len() > MAX_ARG_BYTES {
        return Err(ScriptError::ArgTooLong {
            limit: MAX_ARG_BYTES,
        });
    }
    // 4) 路径穿越：单引号包裹下 `..` 本无意义，出现即说明有人在试探
    if value.contains("..") {
        return Err(ScriptError::PathEscape);
    }

    let chars: Vec<char> = value.chars().collect();

    // 5) 命令替换
    for (i, c) in chars.iter().enumerate() {
        match c {
            '`' => return Err(ScriptError::DangerousSubshell),
            '$' => {
                if matches!(chars.get(i + 1), Some('(')) {
                    return Err(ScriptError::DangerousSubshell);
                }
            }
            '<' | '>' => {
                if matches!(chars.get(i + 1), Some('(')) {
                    return Err(ScriptError::DangerousSubshell);
                }
            }
            _ => {}
        }
    }
    // 6) 变量展开：${...} 或 $FOO
    for (i, c) in chars.iter().enumerate() {
        if *c == '$' && (matches!(chars.get(i + 1), Some('{')) || is_var_expansion_at(&chars, i)) {
            return Err(ScriptError::DangerousVarExpansion);
        }
    }
    // 7) 链接 / 管道
    if chars.iter().any(|c| matches!(c, ';' | '&' | '|')) {
        return Err(ScriptError::DangerousChain);
    }
    // 8) 重定向
    if chars.iter().any(|c| matches!(c, '>' | '<')) {
        return Err(ScriptError::DangerousRedirect);
    }
    // 9) 通配符
    if chars.iter().any(|c| matches!(c, '*' | '?' | '[')) {
        return Err(ScriptError::DangerousGlob);
    }

    // 10) 类型专属校验
    match param_type {
        ParamType::Int => {
            if value.trim().parse::<i64>().is_err() {
                return Err(ScriptError::ParamTypeMismatch { expected: "int" });
            }
        }
        ParamType::Bool => {
            if value != "true" && value != "false" {
                return Err(ScriptError::ParamTypeMismatch { expected: "bool" });
            }
        }
        ParamType::Enum => {
            // 取值域校验需要 options，由 `validate_enum_value` 承担
        }
        ParamType::String | ParamType::Path => {}
    }
    Ok(())
}

/// `Enum` 参数的取值必须在 options 内（字符校验之后再判）。
/// 契约预留：`Enum` 取值域校验由 **M2-4 执行通道**在拼 argv 前调用
/// （本卡无执行能力，故当前无调用者；已有单测覆盖）。
#[allow(dead_code)]
pub fn validate_enum_value(value: &str, options: &[String]) -> Result<(), ScriptError> {
    validate_param_value(value, ParamType::Enum)?;
    if !options.iter().any(|o| o == value) {
        return Err(ScriptError::ParamNotInOptions);
    }
    Ok(())
}

/// 必填参数是否为空（**空串与全空白都算未填**）。
/// 契约预留：必填校验由 **M2-4 执行通道**在启动进程前调用（本卡无执行能力）。
#[allow(dead_code)]
pub fn check_required(param: &ScriptParam, value: Option<&str>) -> Result<(), ScriptError> {
    if !param.required {
        return Ok(());
    }
    match value.map(str::trim) {
        Some(v) if !v.is_empty() => Ok(()),
        _ => Err(ScriptError::ParamRequired {
            name: param.name.clone(),
        }),
    }
}

// ----------------------------- 脱敏 -----------------------------

/// 参数名是否敏感：`secret` 标记优先，其次既有 21 键精确匹配，最后包含匹配兜底。
/// 契约预留：脱敏由 **M2-4 审计写入**时调用（本卡审计不含参数值，见冻结裁定书 §4）。
#[allow(dead_code)]
pub fn is_secret_param(param: &ScriptParam) -> bool {
    if param.secret {
        return true;
    }
    let lower = param.name.to_ascii_lowercase();
    if crate::security_policy::is_sensitive_query_key(&lower) {
        return true;
    }
    SECRET_NAME_HINTS.iter().any(|h| lower.contains(h))
}

/// 审计用参数值（敏感一律 `***`）。
/// 契约预留：与 `is_secret_param` 同，由 M2-4 审计写入时调用。
#[allow(dead_code)]
pub fn redact_param_value(param: &ScriptParam, value: &str) -> String {
    if is_secret_param(param) {
        return REDACTED.to_string();
    }
    value.to_string()
}

#[cfg(test)]
mod script_domain_tests {
    use super::*;
    use crate::domain::{ScriptInterpreter, ScriptMeta, ScriptParam};
    use chrono::Utc;

    fn param(name: &str, t: ParamType) -> ScriptParam {
        ScriptParam {
            name: name.into(),
            label: name.into(),
            param_type: t,
            required: false,
            default: None,
            options: vec![],
            raw: false,
            secret: false,
        }
    }

    fn meta(id: &str, interp: ScriptInterpreter, params: Vec<ScriptParam>) -> ScriptMeta {
        let now = Utc::now();
        ScriptMeta {
            id: id.into(),
            name: "示例脚本".into(),
            category: "文件".into(),
            path: format!("{}.{}", id, interp.ext()),
            interpreter: interp,
            params,
            description: String::new(),
            builtin: false,
            enabled: true,
            timeout_secs: 0,
            created_at: now,
            updated_at: now,
        }
    }

    fn uuid() -> String {
        uuid::Uuid::new_v4().to_string()
    }

    // ---- T-scr-11：危险值必须被拒（9 条规则逐条） ----
    #[test]
    fn dangerous_values_are_rejected() {
        let cases: Vec<(&str, ScriptError)> = vec![
            ("$(whoami)", ScriptError::DangerousSubshell),
            ("`id`", ScriptError::DangerousSubshell),
            ("<(cat /etc/passwd)", ScriptError::DangerousSubshell),
            (">(tee /tmp/x)", ScriptError::DangerousSubshell),
            ("${HOME}", ScriptError::DangerousVarExpansion),
            ("$USER", ScriptError::DangerousVarExpansion),
            ("; rm -rf /tmp/x", ScriptError::DangerousChain),
            ("a && b", ScriptError::DangerousChain),
            ("a | b", ScriptError::DangerousChain),
            ("a &", ScriptError::DangerousChain),
            ("a > b", ScriptError::DangerousRedirect),
            ("a < b", ScriptError::DangerousRedirect),
            ("*.log", ScriptError::DangerousGlob),
            ("a?", ScriptError::DangerousGlob),
            ("[a-z]", ScriptError::DangerousGlob),
            ("line1\nline2", ScriptError::DangerousNewline),
            ("a\rb", ScriptError::DangerousNewline),
            ("a\0b", ScriptError::DangerousNewline),
            ("a\x07b", ScriptError::DangerousControlChar),
            ("a\x7fb", ScriptError::DangerousControlChar),
            ("../../etc/passwd", ScriptError::PathEscape),
        ];
        for (value, expected) in cases {
            let got = validate_param_value(value, ParamType::String);
            assert_eq!(
                got,
                Err(expected.clone()),
                "值 {value:?} 应被拒为 {expected:?}"
            );
        }
    }

    // 超长：按字节判定，多字节内容不得 panic 也不得绕过
    #[test]
    fn arg_too_long_is_rejected_by_bytes() {
        let long = "a".repeat(MAX_ARG_BYTES + 1);
        assert_eq!(
            validate_param_value(&long, ParamType::String),
            Err(ScriptError::ArgTooLong {
                limit: MAX_ARG_BYTES
            })
        );
        // 4096 字节的中文（1228 个字符）同样超限
        let cjk = "中".repeat(MAX_ARG_BYTES / 3 + 1);
        assert!(cjk.len() > MAX_ARG_BYTES);
        assert_eq!(
            validate_param_value(&cjk, ParamType::String),
            Err(ScriptError::ArgTooLong {
                limit: MAX_ARG_BYTES
            })
        );
        // 恰好等于上限则放行
        let exact = "a".repeat(MAX_ARG_BYTES);
        assert_eq!(validate_param_value(&exact, ParamType::String), Ok(()));
    }

    // ---- T-scr-17：正常值必须放行（防止 fail-closed 过头） ----
    #[test]
    fn normal_values_are_allowed() {
        for v in [
            "hello",
            "中文文件名",
            "带 空格 的值",
            "a-b_c",
            "/home/u/我的目录/file.txt",
            "v1.2.3",
            "2026-09-03",
            "true",
            "42",
            "-1",
            "a.b:c=d",
            "path/to/dir",
            "~/notes",
            "%",
            "#tag",
            "a,b",
        ] {
            assert_eq!(
                validate_param_value(v, ParamType::String),
                Ok(()),
                "正常值 {v:?} 必须放行"
            );
        }
    }

    // ---- T-scr-15：类型校验 ----
    #[test]
    fn type_specific_validation() {
        assert_eq!(validate_param_value("42", ParamType::Int), Ok(()));
        assert_eq!(validate_param_value("-7", ParamType::Int), Ok(()));
        assert_eq!(
            validate_param_value("abc", ParamType::Int),
            Err(ScriptError::ParamTypeMismatch { expected: "int" })
        );
        assert_eq!(validate_param_value("true", ParamType::Bool), Ok(()));
        assert_eq!(validate_param_value("false", ParamType::Bool), Ok(()));
        assert_eq!(
            validate_param_value("yes", ParamType::Bool),
            Err(ScriptError::ParamTypeMismatch { expected: "bool" })
        );
        // Enum：值在 options 内才放行
        let opts = vec!["a".to_string(), "b".to_string()];
        assert_eq!(validate_enum_value("a", &opts), Ok(()));
        assert_eq!(
            validate_enum_value("c", &opts),
            Err(ScriptError::ParamNotInOptions)
        );
    }

    // ---- T-scr-14：脱敏 ----
    #[test]
    fn secret_params_are_redacted() {
        let mut p = param("api_token", ParamType::String);
        assert!(is_secret_param(&p));
        assert_eq!(redact_param_value(&p, "abc123"), REDACTED);

        // 既有 21 键（精确匹配）
        p = param("password", ParamType::String);
        assert!(is_secret_param(&p));

        // 显式标记优先
        p = param("workspace_dir", ParamType::Path);
        assert!(!is_secret_param(&p));
        p.secret = true;
        assert!(is_secret_param(&p));
        assert_eq!(redact_param_value(&p, "/data/x"), REDACTED);

        // 普通参数不脱敏
        p = param("target", ParamType::String);
        assert_eq!(redact_param_value(&p, "/data/x"), "/data/x");
    }

    // ---- T-scr-9 / T-scr-10：定义期校验 ----
    #[test]
    fn raw_only_allowed_on_path_and_enum() {
        let id = uuid();
        for (t, ok) in [
            (ParamType::String, false),
            (ParamType::Int, false),
            (ParamType::Bool, false),
            (ParamType::Path, true),
            (ParamType::Enum, true),
        ] {
            let mut p = param("p1", t);
            p.raw = true;
            if t == ParamType::Enum {
                p.options = vec!["a".into()];
            }
            let m = meta(&id, ScriptInterpreter::Bash, vec![p]);
            let got = validate_meta(&m);
            if ok {
                assert_eq!(got, Ok(()), "{t:?} 的 raw=true 应放行");
            } else {
                assert_eq!(
                    got,
                    Err(ScriptError::InvalidRawParam),
                    "{t:?} 的 raw=true 应拒绝"
                );
            }
        }
    }

    #[test]
    fn enum_without_options_is_rejected() {
        let id = uuid();
        let m = meta(
            &id,
            ScriptInterpreter::Bash,
            vec![param("mode", ParamType::Enum)],
        );
        assert_eq!(validate_meta(&m), Err(ScriptError::EnumOptionsEmpty));
    }

    #[test]
    fn duplicate_and_invalid_param_names_rejected() {
        let id = uuid();
        let m = meta(
            &id,
            ScriptInterpreter::Bash,
            vec![param("a", ParamType::String), param("a", ParamType::String)],
        );
        assert_eq!(validate_meta(&m), Err(ScriptError::DuplicateParamName));

        let m2 = meta(
            &id,
            ScriptInterpreter::Bash,
            vec![param("a b", ParamType::String)],
        );
        assert_eq!(validate_meta(&m2), Err(ScriptError::InvalidParamName));

        let m3 = meta(
            &id,
            ScriptInterpreter::Bash,
            vec![param("", ParamType::String)],
        );
        assert_eq!(validate_meta(&m3), Err(ScriptError::InvalidParamName));

        let m4 = meta(
            &id,
            ScriptInterpreter::Bash,
            vec![param("${X}", ParamType::String)],
        );
        assert_eq!(validate_meta(&m4), Err(ScriptError::InvalidParamName));
    }

    #[test]
    fn default_values_are_validated_too() {
        let id = uuid();
        let mut p = param("dir", ParamType::String);
        p.default = Some("$(whoami)".into());
        let m = meta(&id, ScriptInterpreter::Bash, vec![p]);
        assert_eq!(validate_meta(&m), Err(ScriptError::DangerousSubshell));
    }

    // ---- T-scr-6 / T-scr-5：id 形态与 path 一致性 ----
    #[test]
    fn id_and_path_must_match() {
        for bad in ["../../etc/passwd", "/etc", "..", "", "a/b", "a b"] {
            assert_eq!(
                validate_script_id(bad),
                Err(ScriptError::InvalidId),
                "{bad:?} 应被拒"
            );
        }
        assert_eq!(validate_script_id(&uuid()), Ok(()));

        let id = uuid();
        let mut m = meta(&id, ScriptInterpreter::Bash, vec![]);
        m.path = format!("{}.py", id); // 与 Bash 的 .sh 不符
        assert_eq!(validate_meta(&m), Err(ScriptError::PathMismatch));
    }

    #[test]
    fn interpreter_ext_mapping_is_stable() {
        assert_eq!(ScriptInterpreter::Bash.ext(), "sh");
        assert_eq!(ScriptInterpreter::Sh.ext(), "sh");
        assert_eq!(ScriptInterpreter::Python3.ext(), "py");
        assert_eq!(ScriptInterpreter::Node.ext(), "js");
        assert_eq!(ScriptInterpreter::Shebang.ext(), "sh");
    }

    // ---- 字段上限 ----
    #[test]
    fn field_limits_are_enforced() {
        let id = uuid();
        let mut m = meta(&id, ScriptInterpreter::Bash, vec![]);
        m.name = "x".repeat(MAX_NAME_BYTES + 1);
        assert_eq!(
            validate_meta(&m),
            Err(ScriptError::FieldTooLong {
                field: "name",
                limit: MAX_NAME_BYTES
            })
        );

        let mut m2 = meta(&id, ScriptInterpreter::Bash, vec![]);
        m2.name = "  ".into();
        assert_eq!(
            validate_meta(&m2),
            Err(ScriptError::FieldEmpty { field: "name" })
        );

        // description 可以为空，但不能超长
        let mut m3 = meta(&id, ScriptInterpreter::Bash, vec![]);
        m3.description = "d".repeat(MAX_DESCRIPTION_BYTES + 1);
        assert_eq!(
            validate_meta(&m3),
            Err(ScriptError::FieldTooLong {
                field: "description",
                limit: MAX_DESCRIPTION_BYTES
            })
        );
    }

    // ---- 正文校验 ----
    #[test]
    fn body_validation() {
        assert_eq!(validate_body(""), Err(ScriptError::BodyEmpty));
        assert_eq!(validate_body("   \n "), Err(ScriptError::BodyEmpty));
        assert_eq!(
            validate_body(&"x".repeat(MAX_BODY_BYTES + 1)),
            Err(ScriptError::BodyTooLarge {
                limit: MAX_BODY_BYTES
            })
        );
        assert_eq!(validate_body("#!/bin/bash\necho hi"), Ok(()));
    }

    // ---- 必填校验 ----
    #[test]
    fn required_params_must_be_present() {
        let mut p = param("target", ParamType::String);
        p.required = true;
        assert_eq!(
            check_required(&p, None),
            Err(ScriptError::ParamRequired {
                name: "target".into()
            })
        );
        assert_eq!(
            check_required(&p, Some("  ")),
            Err(ScriptError::ParamRequired {
                name: "target".into()
            })
        );
        assert_eq!(check_required(&p, Some("/data")), Ok(()));
        // 非必填不校验
        let q = param("opt", ParamType::String);
        assert_eq!(check_required(&q, None), Ok(()));
    }

    // ---- T-scr-13 补充：错误码稳定（前端据此选文案） ----
    #[test]
    fn error_codes_match_frozen_contract() {
        assert_eq!(ScriptError::InvalidId.code(), "INVALID_ID");
        assert_eq!(ScriptError::DangerousSubshell.code(), "DANGEROUS_SUBSHELL");
        assert_eq!(ScriptError::DangerousChain.code(), "DANGEROUS_CHAIN");
        assert_eq!(ScriptError::DangerousRedirect.code(), "DANGEROUS_REDIRECT");
        assert_eq!(ScriptError::DangerousGlob.code(), "DANGEROUS_GLOB");
        assert_eq!(ScriptError::DangerousNewline.code(), "DANGEROUS_NEWLINE");
        assert_eq!(
            ScriptError::DangerousVarExpansion.code(),
            "DANGEROUS_VAREXP"
        );
        assert_eq!(
            ScriptError::DangerousControlChar.code(),
            "DANGEROUS_CONTROL_CHAR"
        );
        assert_eq!(ScriptError::PathEscape.code(), "PATH_ESCAPE");
        assert_eq!(ScriptError::ArgTooLong { limit: 1 }.code(), "ARG_TOO_LONG");
        assert_eq!(
            ScriptError::ParamNotInOptions.code(),
            "PARAM_NOT_IN_OPTIONS"
        );
        assert_eq!(ScriptError::InvalidRawParam.code(), "INVALID_RAW_PARAM");
    }
}

#[cfg(test)]
mod script_persistence_tests {
    use super::*;
    use crate::domain::ScriptInterpreter;
    use std::fs;
    use std::path::PathBuf;

    /// 真实临时目录（与 `images.rs` 单测同款做法：显式收路径参数，
    /// 因此不需要 AppHandle 就能做真实读写）
    fn temp_root(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("mvp-m23-{tag}-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&d).expect("创建临时目录");
        d
    }

    fn sample_meta(id: &str) -> ScriptMeta {
        let now = chrono::Utc::now();
        ScriptMeta {
            id: id.to_string(),
            name: "示例脚本".into(),
            category: "文件".into(),
            path: format!("{id}.sh"),
            interpreter: ScriptInterpreter::Bash,
            params: vec![],
            description: String::new(),
            builtin: false,
            enabled: true,
            timeout_secs: 0,
            created_at: now,
            updated_at: now,
        }
    }

    // T-scr-1：空库返回空列表，不报错
    #[test]
    fn empty_library_is_empty() {
        let root = temp_root("empty");
        assert!(crate::workspace::load_scripts_at(&root.join("scripts.json")).is_empty());
        let _ = fs::remove_dir_all(&root);
    }

    // T-scr-2：保存后元数据与正文都能读回（真实磁盘往返）
    #[test]
    fn metadata_and_body_round_trip() {
        let root = temp_root("roundtrip");
        let id = uuid::Uuid::new_v4().to_string();
        let meta = sample_meta(&id);
        let file = root.join("scripts.json");

        crate::workspace::save_scripts_at(&file, std::slice::from_ref(&meta)).expect("元数据落盘");
        crate::workspace::write_body_at(&root, &meta.path, "#!/bin/bash\necho hi")
            .expect("正文落盘");

        let loaded = crate::workspace::load_scripts_at(&file);
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, id);
        assert_eq!(
            crate::workspace::read_body_at(&root, &meta.path).expect("读正文"),
            "#!/bin/bash\necho hi"
        );
        // 原子写不留下临时文件
        let leftovers: Vec<_> = fs::read_dir(&root)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
            .collect();
        assert!(leftovers.is_empty(), "原子写不应残留 .tmp: {leftovers:?}");

        let _ = fs::remove_dir_all(&root);
    }

    // T-scr-3：正文删除幂等（重复删除不报错）
    #[test]
    fn delete_body_is_idempotent() {
        let root = temp_root("delete");
        let id = uuid::Uuid::new_v4().to_string();
        let meta = sample_meta(&id);
        crate::workspace::write_body_at(&root, &meta.path, "echo x").expect("落盘");
        crate::workspace::delete_body_at(&root, &meta.path).expect("首次删除");
        crate::workspace::delete_body_at(&root, &meta.path).expect("重复删除应幂等");
        assert!(crate::workspace::read_body_at(&root, &meta.path).is_err());
        let _ = fs::remove_dir_all(&root);
    }

    // T-scr-7：正文文件名穿越一律被拒（含 `..`、绝对路径、子目录、反斜杠）
    #[test]
    fn body_path_rejects_escape() {
        let root = temp_root("escape");
        for bad in [
            "../outside.sh",
            "/etc/passwd",
            "sub/dir.sh",
            "..\\win.sh",
            "",
            "..",
            "a\u{0}b.sh",
        ] {
            let got = crate::workspace::body_path_in(&root, bad);
            assert!(got.is_err(), "越界文件名必须被拒绝: {bad:?} -> {got:?}");
        }
        // 合法文件名放行
        let ok_name = format!("{}.sh", uuid::Uuid::new_v4());
        assert!(crate::workspace::body_path_in(&root, &ok_name).is_ok());
        let _ = fs::remove_dir_all(&root);
    }

    // T-scr-4：更新保持 id / created_at 不变，只改 updated_at
    #[test]
    fn update_preserves_identity() {
        let root = temp_root("update");
        let id = uuid::Uuid::new_v4().to_string();
        let mut meta = sample_meta(&id);
        let file = root.join("scripts.json");
        crate::workspace::save_scripts_at(&file, std::slice::from_ref(&meta)).expect("落盘");

        let created = meta.created_at;
        meta.name = "改名后".to_string();
        meta.updated_at = chrono::Utc::now();
        crate::workspace::save_scripts_at(&file, std::slice::from_ref(&meta)).expect("更新");

        let loaded = crate::workspace::load_scripts_at(&file);
        assert_eq!(loaded[0].id, id, "id 不得变化");
        assert_eq!(loaded[0].created_at, created, "created_at 不得变化");
        assert_eq!(loaded[0].name, "改名后");
        let _ = fs::remove_dir_all(&root);
    }

    // T-scr-16：历史 scripts.json 缺新增字段仍能加载（验证 #[serde(default)]）
    #[test]
    fn legacy_json_without_new_fields_loads() {
        let root = temp_root("legacy");
        let file = root.join("scripts.json");
        let id = uuid::Uuid::new_v4().to_string();
        // 只给必填字段：无 params / description / builtin / enabled / timeout_secs
        let legacy = format!(
            r#"[{{"id":"{id}","name":"老脚本","category":"文件","path":"{id}.sh","interpreter":"bash","created_at":"2026-01-01T00:00:00Z","updated_at":"2026-01-01T00:00:00Z"}}]"#
        );
        fs::write(&file, legacy).expect("写历史 JSON");

        let loaded = crate::workspace::load_scripts_at(&file);
        assert_eq!(loaded.len(), 1, "缺字段的历史数据不得被静默丢弃");
        assert!(loaded[0].params.is_empty());
        assert!(loaded[0].description.is_empty());
        assert!(!loaded[0].builtin);
        assert!(loaded[0].enabled, "enabled 缺省应为 true");
        assert_eq!(loaded[0].timeout_secs, 0);
        let _ = fs::remove_dir_all(&root);
    }

    // T-scr-5：内置脚本拒绝删除
    #[test]
    fn builtin_script_cannot_be_deleted() {
        let mut meta = sample_meta(&uuid::Uuid::new_v4().to_string());
        assert_eq!(can_delete(&meta), Ok(()));
        meta.builtin = true;
        assert!(can_delete(&meta).is_err());
    }
}
