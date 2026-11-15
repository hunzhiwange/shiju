//! Telegram 文本格式转换模块。
//!
//! 本模块把应用内部常见的轻量 Markdown 输出转换为 Telegram 支持的 HTML 子集，
//! 并对用户可控内容做 HTML 转义。转换目标是提高可读性，同时避免未转义文本
//! 被 Telegram 当作 HTML 标签或链接属性解释。

use super::TelegramChannel;
use std::fmt::Write as _;

impl TelegramChannel {
    /// 将轻量 Markdown 文本转换为 Telegram HTML。
    ///
    /// # 参数
    /// - `text`: 待发送的原始文本。
    ///
    /// # 返回值
    /// 返回 Telegram `parse_mode = HTML` 可接受的字符串。
    ///
    /// # 处理范围
    /// 支持标题、粗体、斜体、行内代码、HTTP(S) 链接、删除线和 fenced code block。
    /// 不支持的 Markdown 语法会按普通文本转义输出。
    ///
    /// # 安全说明
    /// 所有写入 HTML 标签内容或属性的用户文本都会先转义，避免注入额外 HTML。
    pub(super) fn markdown_to_telegram_html(text: &str) -> String {
        let lines: Vec<&str> = text.split('\n').collect();
        let mut result_lines: Vec<String> = Vec::new();

        for line in &lines {
            let trimmed_line = line.trim_start();
            if trimmed_line.starts_with("```") {
                result_lines.push(trimmed_line.to_string());
                continue;
            }

            let mut line_out = String::new();

            let stripped = line.trim_start_matches('#');
            let header_level = line.len() - stripped.len();
            if header_level > 0 && line.starts_with('#') && stripped.starts_with(' ') {
                let title = Self::escape_html(stripped.trim());
                result_lines.push(format!("<b>{title}</b>"));
                continue;
            }

            let mut i = 0;
            let bytes = line.as_bytes();
            let len = bytes.len();
            while i < len {
                if i + 1 < len && bytes[i] == b'*' && bytes[i + 1] == b'*' {
                    if let Some(end) = line[i + 2..].find("**") {
                        let inner = Self::escape_html(&line[i + 2..i + 2 + end]);
                        write!(line_out, "<b>{inner}</b>").unwrap();
                        i += 4 + end;
                        continue;
                    }
                }
                if i + 1 < len && bytes[i] == b'_' && bytes[i + 1] == b'_' {
                    if let Some(end) = line[i + 2..].find("__") {
                        let inner = Self::escape_html(&line[i + 2..i + 2 + end]);
                        write!(line_out, "<b>{inner}</b>").unwrap();
                        i += 4 + end;
                        continue;
                    }
                }
                if bytes[i] == b'*' && (i == 0 || bytes[i - 1] != b'*') {
                    if let Some(end) = line[i + 1..].find('*') {
                        if end > 0 {
                            let inner = Self::escape_html(&line[i + 1..i + 1 + end]);
                            write!(line_out, "<i>{inner}</i>").unwrap();
                            i += 2 + end;
                            continue;
                        }
                    }
                }
                if bytes[i] == b'`' && (i == 0 || bytes[i - 1] != b'`') {
                    if let Some(end) = line[i + 1..].find('`') {
                        let inner = Self::escape_html(&line[i + 1..i + 1 + end]);
                        write!(line_out, "<code>{inner}</code>").unwrap();
                        i += 2 + end;
                        continue;
                    }
                }
                if bytes[i] == b'[' {
                    if let Some(bracket_end) = line[i + 1..].find(']') {
                        let text_part = &line[i + 1..i + 1 + bracket_end];
                        let after_bracket = i + 1 + bracket_end + 1;
                        if after_bracket < len && bytes[after_bracket] == b'(' {
                            if let Some(paren_end) = line[after_bracket + 1..].find(')') {
                                let url = &line[after_bracket + 1..after_bracket + 1 + paren_end];
                                if url.starts_with("http://") || url.starts_with("https://") {
                                    let text_html = Self::escape_html(text_part);
                                    let url_html = Self::escape_html(url);
                                    // Telegram HTML 只需要安全链接，限制协议可避免把任意 scheme 包进 href。
                                    write!(line_out, "<a href=\"{url_html}\">{text_html}</a>")
                                        .unwrap();
                                    i = after_bracket + 1 + paren_end + 1;
                                    continue;
                                }
                            }
                        }
                    }
                }
                if i + 1 < len && bytes[i] == b'~' && bytes[i + 1] == b'~' {
                    if let Some(end) = line[i + 2..].find("~~") {
                        let inner = Self::escape_html(&line[i + 2..i + 2 + end]);
                        write!(line_out, "<s>{inner}</s>").unwrap();
                        i += 4 + end;
                        continue;
                    }
                }
                let ch = line[i..].chars().next().unwrap();
                match ch {
                    '<' => line_out.push_str("&lt;"),
                    '>' => line_out.push_str("&gt;"),
                    '&' => line_out.push_str("&amp;"),
                    '"' => line_out.push_str("&quot;"),
                    '\'' => line_out.push_str("&#39;"),
                    _ => line_out.push(ch),
                }
                i += ch.len_utf8();
            }
            result_lines.push(line_out);
        }

        let joined = result_lines.join("\n");
        let mut final_out = String::with_capacity(joined.len());
        let mut in_code_block = false;
        let mut code_buf = String::new();

        for line in joined.split('\n') {
            let trimmed = line.trim();
            if trimmed.starts_with("```") {
                if in_code_block {
                    in_code_block = false;
                    let escaped = code_buf.trim_end_matches('\n');
                    writeln!(final_out, "<pre><code>{escaped}</code></pre>").unwrap();
                    code_buf.clear();
                } else {
                    in_code_block = true;
                    code_buf.clear();
                }
            } else if in_code_block {
                code_buf.push_str(line);
                code_buf.push('\n');
            } else {
                final_out.push_str(line);
                final_out.push('\n');
            }
        }
        if in_code_block && !code_buf.is_empty() {
            writeln!(final_out, "<pre><code>{}</code></pre>", code_buf.trim_end()).unwrap();
        }

        final_out.trim_end_matches('\n').to_string()
    }

    /// 转义 Telegram HTML 文本或属性中的特殊字符。
    ///
    /// # 参数
    /// - `s`: 原始字符串。
    ///
    /// # 返回值
    /// 返回替换了 `&`、`<`、`>`、`"` 和 `'` 的安全字符串。
    pub(super) fn escape_html(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#39;")
    }
}
