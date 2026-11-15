//! 标志混淆 validator。
//!
//! 阻断通过 ANSI-C 字符串、空引号拼接或相邻引号隐藏 `-` 的参数标志，避免危险标志
//! 绕过 allowlist 或人工审阅。

use super::{SecurityCategory, SecurityValidator, block, raw};
use crate::shell::ast::ParsedCommand;
use regex::Regex;
use std::sync::LazyLock;

static ANSI_C_FLAG_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\$'([^']*\\x2d[^']*)+'").expect("valid ansi-c quoted flag regex")
});
static EMPTY_QUOTE_FLAG_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"['"]{2}\s*--?[A-Za-z]"#).expect("valid empty quote flag regex"));
static QUOTE_ADJACENT_FLAG_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"['"][^'"]*['"]-[-A-Za-z]"#).expect("valid adjacent quote flag regex")
});

pub(super) struct ObfuscatedFlagsValidator;

impl SecurityValidator for ObfuscatedFlagsValidator {
    fn name(&self) -> &str {
        "obfuscated_flags"
    }

    fn validate(&self, cmd: &ParsedCommand) -> Vec<crate::shell::security::SecurityFinding> {
        let command = raw(cmd);
        if ANSI_C_FLAG_RE.is_match(command)
            || EMPTY_QUOTE_FLAG_RE.is_match(command)
            || QUOTE_ADJACENT_FLAG_RE.is_match(command)
        {
            return vec![block(
                SecurityCategory::Obfuscation,
                "Command uses quoted or ANSI-C encoded flags to obscure its intent",
                Some("Pass flags as plain literal arguments"),
            )];
        }
        Vec::new()
    }
}
