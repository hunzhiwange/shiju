//! 解析嵌入助手 Markdown 的 Codex App 指令，同时保留可见正文。
//! Codex App directives embedded in assistant markdown.

#[cfg(any(test, target_arch = "wasm32"))]
use std::collections::HashMap;
#[cfg(any(test, target_arch = "wasm32"))]
use std::collections::HashSet;
#[cfg(any(test, target_arch = "wasm32"))]
use std::path::Path;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) enum GitActionDirective {
    /// 指令只携带界面执行动作所需的工作目录、分支和 PR 元数据。
    Stage {
        cwd: String,
    },
    Commit {
        cwd: String,
    },
    CreateBranch {
        cwd: String,
        branch: String,
    },
    Push {
        cwd: String,
        branch: String,
    },
    CreatePr {
        cwd: String,
        branch: String,
        url: Option<String>,
        is_draft: bool,
    },
}

impl GitActionDirective {
    #[cfg(test)]
    pub(crate) fn created_branch_cwd(&self) -> Option<&str> {
        match self {
            Self::CreateBranch { cwd, .. } => Some(cwd),
            _ => None,
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
#[cfg(test)]
pub(crate) struct ParsedAssistantMarkdown {
    /// 去除有效 Git 指令后的正文，以及按出现顺序去重的动作列表。
    pub(crate) visible_markdown: String,
    pub(crate) git_actions: Vec<GitActionDirective>,
}

#[cfg(test)]
impl ParsedAssistantMarkdown {
    #[cfg(test)]
    pub(crate) fn last_created_branch_cwd(&self) -> Option<&str> {
        self.git_actions.iter().rev().find_map(GitActionDirective::created_branch_cwd)
    }
}

#[cfg(test)]
pub(crate) fn parse_assistant_markdown(markdown: &str, cwd: &Path) -> ParsedAssistantMarkdown {
    // 按行改写代码评论并提取 Git 指令，避免隐藏标记生成空白展示行。
    let mut git_actions = Vec::new();
    let mut seen = HashSet::new();
    let mut visible_lines = Vec::new();

    for line in markdown.lines() {
        if let Some(rewritten) = rewrite_code_comment_line(line, cwd) {
            visible_lines.push(rewritten.trim_end().to_string());
            continue;
        }
        let (visible_line, line_actions) = strip_line_directives(line);
        for action in line_actions {
            // 相同动作只物化一次，同时保留不同动作第一次出现的顺序。
            if seen.insert(action.clone()) {
                git_actions.push(action);
            }
        }
        visible_lines.push(visible_line.trim_end().to_string());
    }

    while visible_lines.last().is_some_and(std::string::String::is_empty) {
        visible_lines.pop();
    }

    ParsedAssistantMarkdown { visible_markdown: visible_lines.join("\n"), git_actions }
}

#[cfg(test)]
fn rewrite_code_comment_line(line: &str, cwd: &Path) -> Option<String> {
    // 仅接受一到三个前导冒号及完整必填属性，格式异常时让原文继续可见。
    let content = line.trim_start_matches([' ', '\t']);
    let indent = &line[..line.len() - content.len()];
    let marker_length = content.bytes().take_while(|byte| *byte == b':').count();
    if !(1..=3).contains(&marker_length) {
        return None;
    }

    let directive = content[marker_length..].strip_prefix("code-comment{")?;
    let (attributes, suffix) = directive.rsplit_once('}')?;
    let attributes = parse_code_comment_attributes(attributes)?;
    let title = attributes.get("title")?;
    let body = attributes.get("body")?;
    let file = attributes.get("file")?;
    let title = title.trim();
    let body = body.trim();
    let file = file.trim();
    (!title.is_empty() && !body.is_empty() && !file.is_empty()).then_some(())?;

    let start = directive_integer(&attributes, "start").unwrap_or(1).max(1);
    let end = directive_integer(&attributes, "end").unwrap_or(start).max(start);
    let title = if title_has_priority(title) {
        title.to_string()
    } else if let Some(priority @ 0..=3) = directive_integer(&attributes, "priority") {
        format!("[P{priority}] {title}")
    } else {
        title.to_string()
    };
    // 仓库内路径尽量转成相对 cwd 的统一正斜杠形式，外部路径保持原样。
    let file_path = Path::new(file);
    let file =
        file_path.strip_prefix(cwd).unwrap_or(file_path).to_string_lossy().replace('\\', "/");
    let location =
        if start == end { format!("{file}:{start}") } else { format!("{file}:{start}-{end}") };

    Some(format!("{indent}- {title} — {location}\n{indent}  {body}{suffix}"))
}

pub(crate) fn strip_line_directives(line: &str) -> (String, Vec<GitActionDirective>) {
    // 已识别前缀但结构不完整时保留剩余文本；结构完整但动作无效时隐藏指令。
    let mut visible = String::new();
    let mut actions = Vec::new();
    let mut remaining = line;

    while let Some(start) = remaining.find("::git-") {
        visible.push_str(&remaining[..start]);
        let directive = &remaining[start + 2..];
        let Some(open_brace) = directive.find('{') else {
            visible.push_str(&remaining[start..]);
            return (visible, actions);
        };
        let Some(close_brace) = directive[open_brace + 1..].find('}') else {
            visible.push_str(&remaining[start..]);
            return (visible, actions);
        };
        let close_brace = open_brace + 1 + close_brace;
        let name = &directive[..open_brace];
        let attributes = &directive[open_brace + 1..close_brace];
        if let Some(action) = parse_git_action(name, attributes) {
            actions.push(action);
        }
        remaining = &directive[close_brace + 1..];
    }
    visible.push_str(remaining);
    (visible, actions)
}

#[cfg(test)]
fn directive_integer(attributes: &HashMap<String, String>, name: &str) -> Option<i64> {
    attributes.get(name)?.trim().trim_start_matches(['P', 'p']).parse().ok()
}

#[cfg(test)]
fn title_has_priority(title: &str) -> bool {
    let bytes = title.trim_start().as_bytes();
    bytes.len() >= 4
        && bytes[0] == b'['
        && matches!(bytes[1], b'P' | b'p')
        && bytes[2].is_ascii_digit()
        && bytes[3] == b']'
}

#[cfg(test)]
fn parse_code_comment_attributes(input: &str) -> Option<HashMap<String, String>> {
    let mut attributes = HashMap::new();
    let mut rest = input.trim();
    while !rest.is_empty() {
        let equals = rest.find('=')?;
        let name = rest[..equals].trim();
        if name.is_empty() {
            return None;
        }
        rest = rest[equals + 1..].trim_start();
        let (value, next) = if let Some(quoted) = rest.strip_prefix('"') {
            parse_quoted_value(quoted)?
        } else {
            let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
            (rest[..end].to_string(), &rest[end..])
        };
        attributes.insert(name.to_string(), value);
        rest = next.trim_start();
    }
    Some(attributes)
}

fn parse_git_action(name: &str, attributes: &str) -> Option<GitActionDirective> {
    let attrs = parse_attributes(attributes)?;
    let cwd = attrs.get("cwd")?.clone();
    match name {
        "git-stage" => Some(GitActionDirective::Stage { cwd }),
        "git-commit" => Some(GitActionDirective::Commit { cwd }),
        "git-create-branch" => {
            Some(GitActionDirective::CreateBranch { cwd, branch: attrs.get("branch")?.clone() })
        }
        "git-push" => Some(GitActionDirective::Push { cwd, branch: attrs.get("branch")?.clone() }),
        "git-create-pr" => Some(GitActionDirective::CreatePr {
            cwd,
            branch: attrs.get("branch")?.clone(),
            url: attrs.get("url").cloned(),
            is_draft: attrs.get("isDraft").is_some_and(|value| value == "true"),
        }),
        _ => None,
    }
}

fn parse_attributes(input: &str) -> Option<std::collections::HashMap<String, String>> {
    let mut attrs = std::collections::HashMap::new();
    let mut rest = input.trim();
    while !rest.is_empty() {
        let eq = rest.find('=')?;
        let key = rest[..eq].trim();
        if key.is_empty() {
            return None;
        }
        rest = rest[eq + 1..].trim_start();
        let (value, next) = if let Some(quoted) = rest.strip_prefix('"') {
            let end = quoted.find('"')?;
            (quoted[..end].to_string(), &quoted[end + 1..])
        } else {
            let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
            (rest[..end].to_string(), &rest[end..])
        };
        attrs.insert(key.to_string(), value);
        rest = next.trim_start();
    }
    Some(attrs)
}

#[cfg(test)]
fn parse_quoted_value(input: &str) -> Option<(String, &str)> {
    // 只对转义双引号做解码，其他反斜杠按路径内容原样保留。
    let mut value = String::new();
    let mut characters = input.char_indices().peekable();

    while let Some((index, character)) = characters.next() {
        if character == '"' {
            return Some((value, &input[index + 1..]));
        }
        match character {
            '\\' if characters.peek().is_some_and(|(_, next)| *next == '"') => {
                value.push('"');
                characters.next();
            }
            _ => value.push(character),
        }
    }

    None
}

#[cfg(test)]
#[path = "git_action_directives_tests.rs"]
mod tests;
