/// parsed_cmd 全是 read → Read；全是 read/search/list（探索类）→ Search；含 unknown → `None`
use super::*;

pub(crate) fn kind_from_parsed_cmds(types: &[&str]) -> Option<ToolKind> {
    if types.is_empty() {
        return None;
    }
    let kinds: Option<Vec<ToolKind>> = types
        .iter()
        .map(|t| kind_from_codex_parsed_cmd(t))
        .collect();
    let kinds = kinds?;
    if kinds.iter().all(|k| *k == ToolKind::Read) {
        Some(ToolKind::Read)
    } else {
        Some(ToolKind::Search)
    }
}

/// web_search_call / WebSearch / Extension 的 `action` → 标题（query、queries、url）
pub(crate) fn web_action_title(action: &Value) -> String {
    let str_field = |key: &str| {
        action
            .get(key)
            .and_then(Value::as_str)
            .filter(|s| !s.trim().is_empty())
    };
    if let Some(queries) = action.get("queries").and_then(Value::as_array) {
        let items: Vec<&str> = queries
            .iter()
            .filter_map(Value::as_str)
            .filter(|s| !s.trim().is_empty())
            .collect();
        if !items.is_empty() {
            return title_web(&items);
        }
    }
    if let Some(query) = str_field("query") {
        return title_web(&[query]);
    }
    match (str_field("pattern"), str_field("url")) {
        (Some(pattern), Some(url)) => title_web(&[&format!("{pattern} in {url}")]),
        (None, Some(url)) => title_web(&[url]),
        _ => String::new(),
    }
}

// ─── 命令文本 ─────────────────────────────────────────────────────────────

/// 去掉 `file://` 前缀
pub(crate) fn strip_file_url(path: &str) -> &str {
    path.strip_prefix("file://").unwrap_or(path)
}

/// CommandExecution.command / exec_command.cmd：数组形式的 `[zsh, -lc, script]`
/// 取 script，其余以空格拼接；字符串形式去掉 shell 包装前缀。
pub(crate) fn command_from_value(value: &Value) -> String {
    match value {
        Value::String(cmd) => unwrap_shell_string(cmd).to_string(),
        Value::Array(parts) => {
            let parts: Vec<&str> = parts.iter().filter_map(Value::as_str).collect();
            if let [program, flag, script] = parts.as_slice() {
                if is_shell_program(program) && matches!(*flag, "-lc" | "-c" | "-l -c") {
                    return script.to_string();
                }
            }
            parts.join(" ")
        }
        _ => String::new(),
    }
}

fn is_shell_program(program: &str) -> bool {
    let name = program.rsplit('/').next().unwrap_or(program);
    matches!(name, "zsh" | "bash" | "sh" | "fish" | "dash")
}

fn unwrap_shell_string(cmd: &str) -> &str {
    let trimmed = cmd.trim_start();
    let mut parts = trimmed.splitn(3, ' ');
    if let (Some(program), Some(flag), Some(rest)) = (parts.next(), parts.next(), parts.next()) {
        if is_shell_program(program) && matches!(flag, "-lc" | "-c") {
            let rest = rest.trim();
            // 去掉整体包裹的引号
            for quote in ['\'', '"'] {
                if let Some(inner) = rest.strip_prefix(quote).and_then(|r| r.strip_suffix(quote)) {
                    return inner;
                }
            }
            return rest;
        }
    }
    cmd
}

/// exec JS 里 `exec_command({cmd:"…"})` 的命令（D12：先正则取 cmd）
static EXEC_CMD_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"exec_command\(\s*\{\s*"?cmd"?\s*:\s*("(?:[^"\\]|\\.)*")"#).unwrap()
});
static EXEC_WORKDIR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#""?workdir"?\s*:\s*("(?:[^"\\]|\\.)*")"#).unwrap());
/// exec JS 里直接调用的 MCP 工具：`tools.mcp__server__tool(`
static EXEC_MCP_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"tools\.(mcp__[A-Za-z0-9_\-]+)\s*\(").unwrap());

/// JS 字串字面量（双引号）→ 文本；JSON 解不了时按 JS 的 `\'` 再试，最后退回去引号原文。
fn decode_js_string(literal: &str) -> String {
    if let Ok(text) = serde_json::from_str::<String>(literal) {
        return text;
    }
    if let Ok(text) = serde_json::from_str::<String>(&literal.replace("\\'", "'")) {
        return text;
    }
    literal.trim_matches('"').to_string()
}

/// exec JS 源里能静态识别出的信息
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct ExecScript {
    pub cmds: Vec<String>,
    pub workdir: Option<String>,
    /// `(server, tool)`
    pub mcp: Option<(String, String)>,
}

pub(crate) fn parse_exec_script(input: &str) -> ExecScript {
    let cmds = EXEC_CMD_RE
        .captures_iter(input)
        .map(|c| decode_js_string(&c[1]))
        .filter(|c| !c.trim().is_empty())
        .collect();
    let workdir = EXEC_WORKDIR_RE
        .captures(input)
        .map(|c| decode_js_string(&c[1]));
    let mcp = EXEC_MCP_RE.captures(input).and_then(|c| {
        app_session_core::blocks::split_mcp_name(&c[1]).map(|(s, t)| (s.to_string(), t.to_string()))
    });
    ExecScript { cmds, workdir, mcp }
}

/// 两段命令是否是同一条（空白归一后相等，或较长者以较短者开头）
pub(crate) fn same_command(a: &str, b: &str) -> bool {
    let norm = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
    let (a, b) = (norm(a), norm(b));
    if a.is_empty() || b.is_empty() {
        return false;
    }
    a == b || a.starts_with(&b) || b.starts_with(&a)
}

/// exec 调用的标题：一条命令取其首行；多条命令首条加 `…`；都没有时退回 JS 源首行（D12）。
pub(crate) fn exec_title(script: &ExecScript, input: &str) -> (ToolKind, String) {
    match script.cmds.as_slice() {
        [] => match &script.mcp {
            Some((server, tool)) => (ToolKind::Mcp, title_mcp(server, tool)),
            None => (ToolKind::Shell, title_shell(input)),
        },
        [cmd] => (refine_shell_kind(cmd), title_shell(cmd)),
        [first, ..] => {
            let mut title = title_shell(first);
            if !title.ends_with('…') {
                title.push('…');
            }
            (refine_shell_kind(first), title)
        }
    }
}

// ─── 工具输出头 ───────────────────────────────────────────────────────────

/// 输出头里提取的信息与正文
#[derive(Debug, Default, PartialEq)]
pub(crate) struct OutputHeader<'t> {
    pub body: &'t str,
    pub exit_code: Option<i32>,
    pub duration_ms: Option<u64>,
    /// `Script failed` 之类
    pub failed: bool,
}

/// 去掉 `Script completed\nWall time X seconds\nOutput:\n`、
/// `Chunk ID…\nWall time: X seconds\nProcess exited with code N\n…Output:\n`、
/// `Exit code: N\nWall time: X seconds\nOutput:\n` 这类输出头。
/// 只有前几行全部是已知头字段、且以 `Output:` 行结束时才剥离。
pub(crate) fn strip_output_header(text: &str) -> OutputHeader<'_> {
    let mut header = OutputHeader {
        body: text,
        ..OutputHeader::default()
    };
    let mut rest = text;
    for _ in 0..8 {
        let (line, after) = match rest.split_once('\n') {
            Some((line, after)) => (line, after),
            None => (rest, ""),
        };
        let line = line.trim_end_matches('\r');
        if line == "Output:" {
            header.body = after;
            return header;
        }
        if let Some(status) = line.strip_prefix("Script ") {
            if status.contains("fail") || status.contains("error") {
                header.failed = true;
            }
        } else if let Some(secs) = line
            .strip_prefix("Wall time")
            .map(|s| s.trim_start_matches(':').trim())
            .and_then(|s| s.strip_suffix("seconds"))
        {
            header.duration_ms = secs
                .trim()
                .parse::<f64>()
                .ok()
                .map(|s| (s * 1000.0).round() as u64);
        } else if let Some(code) = line
            .strip_prefix("Process exited with code ")
            .or_else(|| line.strip_prefix("Exit code: "))
        {
            header.exit_code = code.trim().parse().ok();
        } else if !(line.starts_with("Chunk ID:")
            || line.starts_with("Original token count:")
            || line.starts_with("Process running with session ID"))
        {
            break;
        }
        if after.is_empty() {
            break;
        }
        rest = after;
    }
    OutputHeader {
        body: text,
        ..OutputHeader::default()
    }
}

/// 旧版 shell 输出：`{"output":"…","metadata":{"exit_code":0,"duration_seconds":0.1}}`
pub(crate) fn parse_legacy_shell_output(text: &str) -> Option<(String, Option<i32>, Option<u64>)> {
    if !text.trim_start().starts_with("{\"output\"") {
        return None;
    }
    #[derive(Deserialize)]
    struct Legacy {
        output: String,
        #[serde(default)]
        metadata: Option<LegacyMeta>,
    }
    #[derive(Deserialize)]
    struct LegacyMeta {
        exit_code: Option<i32>,
        duration_seconds: Option<f64>,
    }
    let legacy: Legacy = serde_json::from_str(text).ok()?;
    let meta = legacy.metadata;
    Some((
        legacy.output,
        meta.as_ref().and_then(|m| m.exit_code),
        meta.and_then(|m| m.duration_seconds)
            .map(|s| (s * 1000.0).round() as u64),
    ))
}

// ─── diff ────────────────────────────────────────────────────────────────
