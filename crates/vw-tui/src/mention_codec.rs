use std::collections::HashMap;
use std::collections::VecDeque;

use codex_utils_plugins::mention_syntax::PLUGIN_TEXT_MENTION_SIGIL;
use codex_utils_plugins::mention_syntax::TOOL_MENTION_SIGIL;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LinkedMention {
    /// 保留符号、可见名称及其绑定路径，支持同名提及按出现顺序匹配。
    pub(crate) sigil: char,
    pub(crate) mention: String,
    pub(crate) path: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DecodedHistoryText {
    pub(crate) text: String,
    pub(crate) mentions: Vec<LinkedMention>,
}

#[allow(dead_code)]
pub(crate) fn encode_history_mentions(text: &str, mentions: &[LinkedMention]) -> String {
    // 按“符号 + 名称”为每个绑定维护队列，重复提及依次消费对应路径。
    if mentions.is_empty() || text.is_empty() {
        return text.to_string();
    }

    let mut mentions_by_token: HashMap<(char, &str), VecDeque<&str>> = HashMap::new();
    for mention in mentions {
        if !matches!(
            mention.sigil,
            TOOL_MENTION_SIGIL | PLUGIN_TEXT_MENTION_SIGIL
        ) {
            continue;
        }
        mentions_by_token
            .entry((mention.sigil, mention.mention.as_str()))
            .or_default()
            .push_back(mention.path.as_str());
    }

    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut index = 0usize;

    while index < bytes.len() {
        if matches!(
            bytes[index],
            byte if byte == TOOL_MENTION_SIGIL as u8 || byte == PLUGIN_TEXT_MENTION_SIGIL as u8
        ) {
            let sigil = bytes[index] as char;
            if sigil == TOOL_MENTION_SIGIL || starts_plaintext_mention(text, index) {
                let name_start = index + 1;
                if let Some(first) = bytes.get(name_start)
                    && is_mention_name_char(*first)
                {
                    let mut name_end = name_start + 1;
                    while let Some(next) = bytes.get(name_end)
                        && is_mention_name_char(*next)
                    {
                        name_end += 1;
                    }

                    let name = &text[name_start..name_end];
                    if (sigil == TOOL_MENTION_SIGIL || ends_plaintext_mention(bytes, name_end))
                        && let Some(path) = mentions_by_token
                            .get_mut(&(sigil, name))
                            .and_then(VecDeque::pop_front)
                    {
                        out.push('[');
                        out.push(sigil);
                        out.push_str(name);
                        out.push_str("](");
                        out.push_str(path);
                        out.push(')');
                        index = name_end;
                        continue;
                    }
                }
            }
        }

        let Some(ch) = text[index..].chars().next() else {
            break;
        };
        out.push(ch);
        index += ch.len_utf8();
    }

    out
}

pub(crate) fn decode_history_mentions_with_at_mentions(
    text: &str,
    at_mentions_enabled: bool,
) -> DecodedHistoryText {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut mentions = Vec::new();
    let mut index = 0usize;

    // 只有完整且受支持的链接提及会被还原，普通 Markdown 链接保持原文。
    while index < bytes.len() {
        if bytes[index] == b'['
            && let Some((sigil, name, path, end_index)) =
                parse_history_linked_mention(text, bytes, index, at_mentions_enabled)
        {
            out.push(sigil);
            out.push_str(name);
            mentions.push(LinkedMention {
                sigil,
                mention: name.to_string(),
                path: path.to_string(),
            });
            index = end_index;
            continue;
        }

        let Some(ch) = text[index..].chars().next() else {
            break;
        };
        out.push(ch);
        index += ch.len_utf8();
    }

    DecodedHistoryText {
        text: out,
        mentions,
    }
}

fn parse_history_linked_mention<'a>(
    text: &'a str,
    text_bytes: &[u8],
    start: usize,
    at_mentions_enabled: bool,
) -> Option<(char, &'a str, &'a str, usize)> {
    // TUI 历史上写入 `$name`，但启用统一提及时，任何规范工具路径都应在历史往返中保留 `@`。
    // TUI historically wrote `$name`, but selected unified `@` mentions should preserve `@` on
    // history round-trip for any canonical tool path.
    if let Some((name, path, end_index)) =
        parse_linked_tool_mention(text, text_bytes, start, TOOL_MENTION_SIGIL)
        && !is_common_env_var(name)
        && is_tool_path(path)
    {
        return Some((TOOL_MENTION_SIGIL, name, path, end_index));
    }

    if at_mentions_enabled {
        if let Some((name, path, end_index)) =
            parse_linked_tool_mention(text, text_bytes, start, PLUGIN_TEXT_MENTION_SIGIL)
            && !is_common_env_var(name)
            && is_tool_path(path)
        {
            return Some((PLUGIN_TEXT_MENTION_SIGIL, name, path, end_index));
        }
    } else if let Some((name, path, end_index)) =
        parse_linked_tool_mention(text, text_bytes, start, PLUGIN_TEXT_MENTION_SIGIL)
        && !is_common_env_var(name)
        && path.starts_with("plugin://")
    {
        return Some((TOOL_MENTION_SIGIL, name, path, end_index));
    }

    None
}

fn parse_linked_tool_mention<'a>(
    text: &'a str,
    text_bytes: &[u8],
    start: usize,
    sigil: char,
) -> Option<(&'a str, &'a str, usize)> {
    // 解析器只接受受限 ASCII 名称和非空右括号路径，避免吞掉任意 Markdown。
    let sigil_index = start + 1;
    if text_bytes.get(sigil_index) != Some(&(sigil as u8)) {
        return None;
    }

    let name_start = sigil_index + 1;
    let first_name_byte = text_bytes.get(name_start)?;
    if !is_mention_name_char(*first_name_byte) {
        return None;
    }

    let mut name_end = name_start + 1;
    while let Some(next_byte) = text_bytes.get(name_end)
        && is_mention_name_char(*next_byte)
    {
        name_end += 1;
    }

    if text_bytes.get(name_end) != Some(&b']') {
        return None;
    }

    let mut path_start = name_end + 1;
    while let Some(next_byte) = text_bytes.get(path_start)
        && next_byte.is_ascii_whitespace()
    {
        path_start += 1;
    }
    if text_bytes.get(path_start) != Some(&b'(') {
        return None;
    }

    let mut path_end = path_start + 1;
    while let Some(next_byte) = text_bytes.get(path_end)
        && *next_byte != b')'
    {
        path_end += 1;
    }
    if text_bytes.get(path_end) != Some(&b')') {
        return None;
    }

    let path = text[path_start + 1..path_end].trim();
    if path.is_empty() {
        return None;
    }

    let name = &text[name_start..name_end];
    Some((name, path, path_end + 1))
}

fn is_mention_name_char(byte: u8) -> bool {
    matches!(byte, b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_' | b'-')
}

fn starts_plaintext_mention(text: &str, index: usize) -> bool {
    if index == 0 {
        return true;
    }

    text.get(..index)
        .and_then(|prefix| prefix.chars().next_back())
        .is_some_and(|ch| ch.is_whitespace() || !is_mention_name_char_char(ch))
}

fn ends_plaintext_mention(text_bytes: &[u8], index: usize) -> bool {
    // 路径样后缀继续属于普通文本；句末标点和空白则允许结束提及。
    text_bytes.get(index).is_none_or(|byte| {
        byte.is_ascii_whitespace()
            || *byte == b'.'
                && text_bytes.get(index + 1).is_none_or(|next| {
                    next.is_ascii_whitespace()
                        || !next.is_ascii_alphanumeric() && *next != b'_' && *next != b'-'
                })
            || !matches!(*byte, b'.' | b'/' | b'\\')
                && !byte.is_ascii_alphanumeric()
                && *byte != b'_'
                && *byte != b'-'
    })
}

fn is_mention_name_char_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-')
}

pub(crate) fn is_common_env_var(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    matches!(
        upper.as_str(),
        "PATH"
            | "HOME"
            | "USER"
            | "SHELL"
            | "PWD"
            | "TMPDIR"
            | "TEMP"
            | "TMP"
            | "LANG"
            | "TERM"
            | "XDG_CONFIG_HOME"
    )
}

fn is_tool_path(path: &str) -> bool {
    path.starts_with("app://")
        || path.starts_with("mcp://")
        || path.starts_with("plugin://")
        || path.starts_with("skill://")
        || path
            .rsplit(['/', '\\'])
            .next()
            .is_some_and(|name| name.eq_ignore_ascii_case("SKILL.md"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    // 支持的 app、plugin 和技能路径应恢复可见 `$` 提及并保留绑定。
    fn decode_history_mentions_restores_visible_tokens() {
        let decoded = decode_history_mentions_with_at_mentions(
            "Use [$figma](app://figma-1), [$sample](plugin://sample@test), and [$figma](/tmp/figma/SKILL.md).",
            /*at_mentions_enabled*/ true,
        );
        assert_eq!(decoded.text, "Use $figma, $sample, and $figma.");
        assert_eq!(
            decoded.mentions,
            vec![
                LinkedMention {
                    sigil: '$',
                    mention: "figma".to_string(),
                    path: "app://figma-1".to_string(),
                },
                LinkedMention {
                    sigil: '$',
                    mention: "sample".to_string(),
                    path: "plugin://sample@test".to_string(),
                },
                LinkedMention {
                    sigil: '$',
                    mention: "figma".to_string(),
                    path: "/tmp/figma/SKILL.md".to_string(),
                },
            ]
        );
    }

    #[test]
    // 启用统一提及时，插件链接的 `@` 符号必须原样恢复。
    fn decode_history_mentions_restores_plugin_links_with_at_sigil() {
        let decoded = decode_history_mentions_with_at_mentions(
            "Use [@sample](plugin://sample@test) and [$figma](app://figma-1).",
            /*at_mentions_enabled*/ true,
        );
        assert_eq!(decoded.text, "Use @sample and $figma.");
        assert_eq!(
            decoded.mentions,
            vec![
                LinkedMention {
                    sigil: '@',
                    mention: "sample".to_string(),
                    path: "plugin://sample@test".to_string(),
                },
                LinkedMention {
                    sigil: '$',
                    mention: "figma".to_string(),
                    path: "app://figma-1".to_string(),
                },
            ]
        );
    }

    #[test]
    // 关闭统一提及时，旧插件链接回退成 `$`，保持旧输入模型可用。
    fn decode_history_mentions_without_at_mentions_uses_legacy_plugin_fallback() {
        let decoded = decode_history_mentions_with_at_mentions(
            "Use [@sample](plugin://sample@test) and [$figma](app://figma-1).",
            /*at_mentions_enabled*/ false,
        );
        assert_eq!(decoded.text, "Use $sample and $figma.");
        assert_eq!(
            decoded.mentions,
            vec![
                LinkedMention {
                    sigil: '$',
                    mention: "sample".to_string(),
                    path: "plugin://sample@test".to_string(),
                },
                LinkedMention {
                    sigil: '$',
                    mention: "figma".to_string(),
                    path: "app://figma-1".to_string(),
                },
            ]
        );
    }

    #[test]
    // 关闭统一提及时，非插件 `@` 链接不能被误识别为旧工具提及。
    fn decode_history_mentions_without_at_mentions_ignores_at_non_plugin_paths() {
        let decoded = decode_history_mentions_with_at_mentions(
            "Use [@figma](app://figma-1).",
            /*at_mentions_enabled*/ false,
        );

        assert_eq!(decoded.text, "Use [@figma](app://figma-1).");
        assert_eq!(decoded.mentions, Vec::<LinkedMention>::new());
    }

    #[test]
    // 启用统一提及时，规范工具路径也应恢复为 `@` 符号。
    fn decode_history_mentions_restores_at_sigil_for_tool_paths() {
        let decoded = decode_history_mentions_with_at_mentions(
            "Use [@figma](app://figma-1).",
            /*at_mentions_enabled*/ true,
        );

        assert_eq!(decoded.text, "Use @figma.");
        assert_eq!(
            decoded.mentions,
            vec![LinkedMention {
                sigil: '@',
                mention: "figma".to_string(),
                path: "app://figma-1".to_string(),
            }]
        );
    }

    #[test]
    // 同名绑定按输入顺序消费，未绑定的后续提及保持纯文本。
    fn encode_history_mentions_links_bound_mentions_in_order() {
        let text = "$figma then $sample then $figma then $other";
        let encoded = encode_history_mentions(
            text,
            &[
                LinkedMention {
                    sigil: '$',
                    mention: "figma".to_string(),
                    path: "app://figma-app".to_string(),
                },
                LinkedMention {
                    sigil: '$',
                    mention: "sample".to_string(),
                    path: "plugin://sample@test".to_string(),
                },
                LinkedMention {
                    sigil: '$',
                    mention: "figma".to_string(),
                    path: "/tmp/figma/SKILL.md".to_string(),
                },
            ],
        );
        assert_eq!(
            encoded,
            "[$figma](app://figma-app) then [$sample](plugin://sample@test) then [$figma](/tmp/figma/SKILL.md) then $other"
        );
    }

    #[test]
    // 标点边界后的 `$` 提及仍应转换为链接。
    fn encode_history_mentions_links_dollar_mentions_after_punctuation() {
        let encoded = encode_history_mentions(
            "($figma)",
            &[LinkedMention {
                sigil: '$',
                mention: "figma".to_string(),
                path: "app://figma".to_string(),
            }],
        );
        assert_eq!(encoded, "([$figma](app://figma))");
    }

    #[test]
    // 路径样后缀属于链接后的正文，不能阻止已绑定 `$` 提及编码。
    fn encode_history_mentions_links_dollar_mentions_with_path_like_suffixes() {
        let mention = LinkedMention {
            sigil: '$',
            mention: "figma".to_string(),
            path: "app://figma".to_string(),
        };

        assert_eq!(
            encode_history_mentions("$figma/docs", std::slice::from_ref(&mention)),
            "[$figma](app://figma)/docs"
        );
        assert_eq!(
            encode_history_mentions("$figma.suffix", std::slice::from_ref(&mention)),
            "[$figma](app://figma).suffix"
        );
        assert_eq!(
            encode_history_mentions("$figma\\docs", &[mention]),
            "[$figma](app://figma)\\docs"
        );
    }

    #[test]
    // 编码统一提及时必须保留原有 `@`，不能改写成旧 `$`。
    fn encode_history_mentions_preserves_at_sigils() {
        let text = "@figma then @sample then $other";
        let encoded = encode_history_mentions(
            text,
            &[
                LinkedMention {
                    sigil: '@',
                    mention: "figma".to_string(),
                    path: "/tmp/figma/SKILL.md".to_string(),
                },
                LinkedMention {
                    sigil: '@',
                    mention: "sample".to_string(),
                    path: "plugin://sample@test".to_string(),
                },
            ],
        );
        assert_eq!(
            encoded,
            "[@figma](/tmp/figma/SKILL.md) then [@sample](plugin://sample@test) then $other"
        );
    }

    #[test]
    // 同一名称可分别绑定 `@` 与 `$`，两类队列不能互相消费。
    fn encode_history_mentions_links_both_sigils_for_same_name() {
        let text = "@figma then $figma";
        let encoded = encode_history_mentions(
            text,
            &[
                LinkedMention {
                    sigil: '@',
                    mention: "figma".to_string(),
                    path: "plugin://figma@test".to_string(),
                },
                LinkedMention {
                    sigil: '$',
                    mention: "figma".to_string(),
                    path: "app://figma".to_string(),
                },
            ],
        );
        assert_eq!(
            encoded,
            "[@figma](plugin://figma@test) then [$figma](app://figma)"
        );
    }

    #[test]
    // 未绑定的 `@` 不能抢占稍后同名 `$` 工具提及的路径。
    fn encode_history_mentions_does_not_let_at_token_steal_later_tool_binding() {
        let text = "@figma then $figma";
        let encoded = encode_history_mentions(
            text,
            &[LinkedMention {
                sigil: '$',
                mention: "figma".to_string(),
                path: "app://figma-app".to_string(),
            }],
        );
        assert_eq!(encoded, "@figma then [$figma](app://figma-app)");
    }

    #[test]
    // 全角空格仍是 `@` 链接可接受的纯文本边界。
    fn encode_history_mentions_links_at_mentions_after_unicode_whitespace() {
        // 修复覆盖：全角空格应继续作为 `@` 链接的有效纯文本边界。
        // Fix coverage: full-width space should remain a valid plaintext boundary for `@` links.
        let text = "foo　@sample";
        let encoded = encode_history_mentions(
            text,
            &[LinkedMention {
                sigil: '@',
                mention: "sample".to_string(),
                path: "plugin://sample@test".to_string(),
            }],
        );
        assert_eq!(encoded, "foo　[@sample](plugin://sample@test)");
    }

    #[test]
    // 句末句点应保留在链接之外。
    fn encode_history_mentions_links_sentence_ending_at_mentions() {
        let text = "Please ask @figma.";
        let encoded = encode_history_mentions(
            text,
            &[LinkedMention {
                sigil: '@',
                mention: "figma".to_string(),
                path: "/tmp/figma/SKILL.md".to_string(),
            }],
        );
        assert_eq!(encoded, "Please ask [@figma](/tmp/figma/SKILL.md).");
    }

    #[test]
    // 括号中的提及只替换内部标记，外围括号保持不变。
    fn encode_history_mentions_links_parenthesized_at_mentions() {
        let text = "Please ask (@figma)";
        let encoded = encode_history_mentions(
            text,
            &[LinkedMention {
                sigil: '@',
                mention: "figma".to_string(),
                path: "plugin://figma@test".to_string(),
            }],
        );
        assert_eq!(encoded, "Please ask ([@figma](plugin://figma@test))");
    }

    #[test]
    // 邮箱和包路径中的嵌入式 `@` 不应被编码，独立提及仍应命中。
    fn encode_history_mentions_skips_embedded_at_substrings() {
        let text = "foo@sample.com npx @sample/pkg then @sample";
        let encoded = encode_history_mentions(
            text,
            &[LinkedMention {
                sigil: '@',
                mention: "sample".to_string(),
                path: "plugin://sample@test".to_string(),
            }],
        );
        assert_eq!(
            encoded,
            "foo@sample.com npx @sample/pkg then [@sample](plugin://sample@test)"
        );
    }
}
//! 在历史存储用 Markdown 链接与输入框可见的工具/插件提及之间双向转换。
