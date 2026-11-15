//! 技能审计风险规则，负责识别命令串联和高风险提示片段。

use regex::Regex;
use std::sync::OnceLock;

/// 执行 contains_shell_chaining 操作，并返回调用方需要的结果。
pub(super) fn contains_shell_chaining(command: &str) -> bool {
    ["&&", "||", ";", "\n", "\r", "`", "$("].iter().any(|needle| command.contains(needle))
}

/// 执行 detect_high_risk_snippet 操作，并返回调用方需要的结果。
pub(super) fn detect_high_risk_snippet(content: &str) -> Option<&'static str> {
    static HIGH_RISK_PATTERNS: OnceLock<Vec<(Regex, &'static str)>> = OnceLock::new();
    let patterns = HIGH_RISK_PATTERNS.get_or_init(|| {
        vec![
            (
                Regex::new(
                    r"(?im)\b(?:ignore|disregard|override|bypass)\b[^\n]{0,140}\b(?:previous|earlier|system|safety|security)\s+instructions?\b",
                )
                .expect("regex"),
                "prompt-injection-override",
            ),
            (
                Regex::new(
                    r"(?im)\b(?:reveal|show|exfiltrate|leak)\b[^\n]{0,140}\b(?:system prompt|developer instructions|hidden prompt|secret instructions)\b",
                )
                .expect("regex"),
                "prompt-injection-exfiltration",
            ),
            (
                Regex::new(
                    r"(?im)\b(?:ask|request|collect|harvest|obtain)\b[^\n]{0,120}\b(?:password|api[_ -]?key|private[_ -]?key|seed phrase|recovery phrase|otp|2fa)\b",
                )
                .expect("regex"),
                "phishing-credential-harvest",
            ),
            (
                Regex::new(r"(?im)\bcurl\b[^\n|]{0,200}\|\s*(?:sh|bash|zsh)\b").expect("regex"),
                "curl-pipe-shell",
            ),
            (
                Regex::new(r"(?im)\bwget\b[^\n|]{0,200}\|\s*(?:sh|bash|zsh)\b").expect("regex"),
                "wget-pipe-shell",
            ),
            (
                Regex::new(r"(?im)\b(?:invoke-expression|iex)\b").expect("regex"),
                "powershell-iex",
            ),
            (
                Regex::new(r"(?im)\brm\s+-rf\s+/").expect("regex"),
                "destructive-rm-rf-root",
            ),
            (
                Regex::new(r"(?im)\bnc(?:at)?\b[^\n]{0,120}\s-e\b").expect("regex"),
                "netcat-remote-exec",
            ),
            (
                Regex::new(r"(?im)\bbase64\s+-d\b[^\n|]{0,220}\|\s*(?:sh|bash|zsh)\b")
                    .expect("regex"),
                "obfuscated-base64-exec",
            ),
            (
                Regex::new(r"(?im)\bdd\s+if=").expect("regex"),
                "disk-overwrite-dd",
            ),
            (
                Regex::new(r"(?im)\bmkfs(?:\.[a-z0-9]+)?\b").expect("regex"),
                "filesystem-format",
            ),
            (
                Regex::new(r"(?im):\(\)\s*\{\s*:\|:\&\s*\};:").expect("regex"),
                "fork-bomb",
            ),
        ]
    });

    patterns.iter().find_map(|(regex, label)| regex.is_match(content).then_some(*label))
}
#[cfg(test)]
#[path = "risk_tests.rs"]
mod risk_tests;
