//! 与 Codex 调用形态兼容的判定逻辑。

use std::path::Path;

fn basename_token(value: &str) -> String {
    let basename = Path::new(value)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(value)
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(value);

    basename
        .to_ascii_lowercase()
        .trim_end_matches(".cmd")
        .trim_end_matches(".exe")
        .trim_end_matches(".bat")
        .to_string()
}

fn is_word_char(ch: char) -> bool {
    ch == '_' || ch == '-' || ch.is_ascii_alphanumeric()
}

fn contains_codex_acp_token(value: &str) -> bool {
    let needle = "codex-acp";
    let mut search_from = 0;

    while let Some(found) = value[search_from..].find(needle) {
        let start = search_from + found;
        let end = start + needle.len();
        let before = value[..start].chars().next_back();
        let after = value[end..].chars().next();
        let before_ok = before.is_none_or(|ch| !is_word_char(ch));
        let after_ok = after.is_none_or(|ch| !is_word_char(ch));
        if before_ok && after_ok {
            return true;
        }
        search_from = end;
    }

    false
}

pub fn is_codex_acp_command(command: &str, args: &[String]) -> bool {
    if basename_token(command) == "codex-acp" {
        return true;
    }

    args.iter().any(|arg| arg.contains("codex-acp"))
}

pub fn is_codex_invocation(agent_name: &str, agent_command: &str) -> bool {
    if agent_name == "codex" {
        return true;
    }

    contains_codex_acp_token(agent_command)
}
