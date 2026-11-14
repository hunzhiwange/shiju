const SYSTEM_REMINDER_OPEN: &str = "<system-reminder>";
const SYSTEM_REMINDER_CLOSE: &str = "</system-reminder>";

fn thinking_block(
    item: &RawItem<'_>,
    kind: &str,
    index: usize,
    duration_ms: Option<u64>,
    line: JsonlSpan,
) -> Option<SessionBlock> {
    let text = item.thinking.as_str().unwrap_or_default();
    let has_signature = item.signature.as_str().is_some_and(|s| !s.is_empty());
    let redacted = kind == "redacted_thinking" || (text.trim().is_empty() && has_signature);
    if text.trim().is_empty() && !redacted {
        return None;
    }
    let p = preview_chars(text, THINKING_PREVIEW_CHARS);
    Some(SessionBlock::Thinking {
        text: if redacted { String::new() } else { p.text },
        summary: None,
        redacted,
        duration_ms,
        full: (!redacted && p.truncated)
            .then(|| line.content_ref(format!("/message/content/{index}/thinking"))),
    })
}

fn tool_call_block(item: RawItem<'_>, index: usize, line: JsonlSpan) -> SessionBlock {
    let id = item.id.as_str().unwrap_or_default().to_string();
    let raw_name = item.name.as_str().unwrap_or("unknown").to_string();
    let input = item.input.unwrap_or(Value::Null);
    let input_json = if input.is_null() {
        String::new()
    } else {
        serde_json::to_string(&input).unwrap_or_default()
    };
    let p = preview_chars(&input_json, INPUT_PREVIEW_CHARS);
    let normalized = normalize_tool(ToolSource::Claude, &raw_name, None);
    let (kind, title, detail) = claude_tool_title(&raw_name, &normalized, &input);
    let diff = input_diff(&raw_name, &input);

    SessionBlock::ToolCall {
        id,
        raw_name,
        kind,
        title,
        detail,
        server: normalized.server,
        input_preview: p.text,
        input_total_len: p.total_len,
        // 指向参数对象（不是字符串），取全文时由命令层序列化
        input_full: p
            .truncated
            .then(|| line.content_ref(format!("/message/content/{index}/input"))),
        diff,
        by_user: false,
    }
}

/// 按 §3.3 提炼 Claude 工具的 kind（Bash 细化为 search）、标题与次要信息。
fn claude_tool_title(
    raw_name: &str,
    normalized: &NormalizedTool,
    input: &Value,
) -> (ToolKind, String, Option<String>) {
    let s = |key: &str| {
        input
            .get(key)
            .and_then(Value::as_str)
            .filter(|v| !v.trim().is_empty())
    };
    let kind = normalized.kind;

    if kind == ToolKind::Mcp {
        let (server, tool) = split_mcp_name(raw_name)
            .unwrap_or((normalized.server.as_deref().unwrap_or_default(), raw_name));
        return (kind, title_mcp(server, tool), first_string_field(input));
    }

    match raw_name {
        "Bash" => {
            let command = s("command").unwrap_or_default();
            (
                refine_shell_kind(command),
                title_shell(command),
                s("description").map(one_line_title),
            )
        }
        "Read" => {
            let u = |key: &str| input.get(key).and_then(Value::as_u64);
            let (title, detail) =
                title_read(s("file_path").unwrap_or_default(), u("offset"), u("limit"));
            (kind, title, detail)
        }
        "Grep" => (
            kind,
            title_search(
                s("pattern").unwrap_or_default(),
                s("path").or_else(|| s("glob")),
            ),
            s("output_mode").map(str::to_string),
        ),
        "Glob" => (
            kind,
            title_search(s("pattern").unwrap_or_default(), s("path")),
            None,
        ),
        "ToolSearch" => (
            kind,
            title_search(s("query").unwrap_or_default(), None),
            None,
        ),
        "Edit" | "MultiEdit" | "Write" => {
            (kind, title_path(s("file_path").unwrap_or_default()), None)
        }
        "NotebookEdit" => (
            kind,
            title_path(s("notebook_path").unwrap_or_default()),
            None,
        ),
        "WebFetch" => (kind, title_web(&[s("url").unwrap_or_default()]), None),
        "WebSearch" => (kind, title_web(&[s("query").unwrap_or_default()]), None),
        "Agent" | "Task" => (
            kind,
            title_agent(s("description").or_else(|| s("prompt")).unwrap_or_default()),
            s("model").map(str::to_string),
        ),
        "AskUserQuestion" => {
            let question = input
                .get("questions")
                .and_then(Value::as_array)
                .and_then(|qs| qs.first())
                .and_then(|q| q.get("question"))
                .and_then(Value::as_str)
                .unwrap_or_default();
            (kind, title_ask(question), None)
        }
        "TodoWrite" => {
            let count = input
                .get("todos")
                .and_then(Value::as_array)
                .map_or(0, Vec::len);
            (kind, title_todo(count), None)
        }
        _ if kind == ToolKind::Agent => (
            kind,
            first_string_field(input).unwrap_or_else(|| title_other(raw_name)),
            None,
        ),
        _ => (kind, title_other(raw_name), first_string_field(input)),
    }
}

/// 从工具参数估算的 diff；结果记录到来后用 `structuredPatch` 修正（见 `backfill_call`）。
fn input_diff(raw_name: &str, input: &Value) -> Option<DiffSummary> {
    fn s<'v>(value: &'v Value, key: &str) -> &'v str {
        value.get(key).and_then(Value::as_str).unwrap_or_default()
    }
    let path = s(input, "file_path");
    let (op, added, removed) = match raw_name {
        "Edit" => {
            let (added, removed) =
                line_change_counts(s(input, "old_string"), s(input, "new_string"));
            (DiffOp::Update, added, removed)
        }
        "MultiEdit" => {
            let edits = input.get("edits").and_then(Value::as_array)?;
            edits
                .iter()
                .fold((DiffOp::Update, 0, 0), |(op, a, r), edit| {
                    let (added, removed) =
                        line_change_counts(s(edit, "old_string"), s(edit, "new_string"));
                    (op, a + added, r + removed)
                })
        }
        "Write" => {
            let lines = u32::try_from(s(input, "content").lines().count()).unwrap_or(u32::MAX);
            (DiffOp::Add, lines, 0)
        }
        _ => return None,
    };
    if path.is_empty() {
        return None;
    }
    Some(DiffSummary {
        files: vec![DiffFile {
            path: path.to_string(),
            op,
            added,
            removed,
        }],
        added,
        removed,
        full: None,
    })
}

fn image_ref(source: Option<&RawSource<'_>>, pointer: String, line: JsonlSpan) -> Option<ImageRef> {
    let source = source?;
    if source.kind.as_str() != Some("base64") {
        return None;
    }
    let data = source.data.as_str()?;
    Some(ImageRef {
        source: ImageSource::Inline {
            content: line.content_ref(pointer),
        },
        media_type: source
            .media_type
            .as_str()
            .unwrap_or("application/octet-stream")
            .to_string(),
        size: estimate_base64_size(data.len()),
        alt: None,
    })
}

/// 用户 `!cmd`：`<bash-input>cmd</bash-input><bash-stdout>…</bash-stdout><bash-stderr>…</bash-stderr>`
fn user_bash_blocks(text: &str, uuid: Option<&str>) -> Vec<SessionBlock> {
    let command = tag_content(text, "bash-input").unwrap_or_default();
    let id = format!("user-bash-{}", uuid.unwrap_or_default());
    let input = preview_chars(command, INPUT_PREVIEW_CHARS);
    let mut blocks = vec![SessionBlock::ToolCall {
        id: id.clone(),
        raw_name: "Bash".to_string(),
        kind: refine_shell_kind(command),
        title: title_shell(command),
        detail: None,
        server: None,
        input_preview: input.text,
        input_total_len: input.total_len,
        input_full: None,
        diff: None,
        by_user: true,
    }];

    let stdout = tag_content(text, "bash-stdout");
    let stderr = tag_content(text, "bash-stderr");
    if stdout.is_some() || stderr.is_some() {
        let stdout = stdout.unwrap_or_default().trim_end();
        let stderr = stderr.unwrap_or_default().trim_end();
        let output = match (stdout.is_empty(), stderr.is_empty()) {
            (_, true) => stdout.to_string(),
            (true, false) => stderr.to_string(),
            (false, false) => format!("{stdout}\n{stderr}"),
        };
        let p = preview(&output);
        blocks.push(SessionBlock::ToolResult {
            call_id: id,
            // 只有 stderr 不代表失败，状态未知
            status: if stderr.is_empty() {
                ToolStatus::Success
            } else {
                ToolStatus::Unknown
            },
            preview: p.text,
            total_len: p.total_len,
            line_count: p.line_count,
            truncated: p.truncated,
            full: None,
            exit_code: None,
            duration_ms: None,
            images: Vec::new(),
            saved_path: None,
        });
    }
    blocks
}

fn message_meta(
    model: &LStr<'_>,
    usage: Option<&Value>,
    stop_reason: &LStr<'_>,
) -> Option<MessageMeta> {
    // 0 视为缺省，省掉序列化体积
    let tokens = |key: &str| {
        usage
            .and_then(|u| u.get(key))
            .and_then(Value::as_u64)
            .filter(|n| *n > 0)
    };
    let meta = MessageMeta {
        model: model
            .as_str()
            .filter(|m| !m.is_empty() && *m != "<synthetic>")
            .map(str::to_string),
        input_tokens: tokens("input_tokens"),
        output_tokens: tokens("output_tokens"),
        cache_read_tokens: tokens("cache_read_input_tokens"),
        cache_write_tokens: tokens("cache_creation_input_tokens"),
        stop_reason: stop_reason.as_str().map(str::to_string),
        ..MessageMeta::default()
    };
    (meta != MessageMeta::default()).then_some(meta)
}

/// 同一回复拆成的多条记录：后到的非空字段覆盖先前的（usage 以最后一块为准）。
fn merge_meta(target: &mut Option<MessageMeta>, next: Option<MessageMeta>) {
    let Some(next) = next else { return };
    let Some(current) = target.as_mut() else {
        *target = Some(next);
        return;
    };
    macro_rules! take {
        ($($field:ident),*) => {$(
            if next.$field.is_some() {
                current.$field = next.$field;
            }
        )*};
    }
    take!(
        model,
        input_tokens,
        output_tokens,
        cache_read_tokens,
        cache_write_tokens,
        stop_reason
    );
}

// ─── 文本规则 ────────────────────────────────────────────────────────────

/// 去掉所有 `<system-reminder>…</system-reminder>` 片段；没有闭合标签时去到末尾。
fn strip_system_reminders(text: &str) -> String {
    if !text.contains(SYSTEM_REMINDER_OPEN) {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find(SYSTEM_REMINDER_OPEN) {
        out.push_str(&rest[..start]);
        let after = &rest[start + SYSTEM_REMINDER_OPEN.len()..];
        rest = match after.find(SYSTEM_REMINDER_CLOSE) {
            Some(end) => &after[end + SYSTEM_REMINDER_CLOSE.len()..],
            None => "",
        };
    }
    out.push_str(rest);
    out.trim().to_string()
}

/// 系统/工具注入、不是用户亲手输入的文本。
fn is_injected_text(text: &str) -> bool {
    text.contains("<local-command-caveat>")
        || text.starts_with("<local-command-stdout>")
        || text.starts_with("<local-command-stderr>")
        || text.starts_with("<task-notification>")
}

/// `<command-name>/compact</command-name><command-args>…</command-args>` → `/compact …`
fn slash_command(text: &str) -> Option<String> {
    if !text.starts_with("<command-") {
        return None;
    }
    let name = tag_content(text, "command-name")?.trim();
    if name.is_empty() {
        return None;
    }
    let args = tag_content(text, "command-args").unwrap_or_default().trim();
    Some(if args.is_empty() {
        name.to_string()
    } else {
        format!("{name} {args}")
    })
}

/// 取 `<tag>…</tag>` 之间的内容（第一处）。
fn tag_content<'t>(text: &'t str, tag: &str) -> Option<&'t str> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = text.find(&open)? + open.len();
    let end = text[start..].find(&close).map_or(text.len(), |i| start + i);
    Some(&text[start..end])
}

/// `Exit code N` 开头的工具输出（Claude Bash 失败时的格式）
fn parse_exit_code(text: &str) -> Option<i32> {
    let first = text.trim_start().lines().next()?;
    let rest = first.strip_prefix("Error: ").unwrap_or(first);
    rest.strip_prefix("Exit code ")?.trim().parse().ok()
}

fn content_text(content: &RawContent<'_>) -> String {
    match content {
        RawContent::Text(text) => text.to_string(),
        RawContent::Items(items) => items
            .iter()
            .filter_map(|item| item.0.as_ref())
            .filter(|item| item.kind.as_str() == Some("text"))
            .filter_map(|item| item.text.as_str())
            .collect::<Vec<_>>()
            .join("\n"),
        RawContent::None => String::new(),
    }
}

fn is_true(value: &Option<Value>) -> bool {
    value.as_ref().and_then(Value::as_bool) == Some(true)
}

fn as_u64(value: &Value) -> Option<u64> {
    value
        .as_u64()
        .or_else(|| value.as_f64().filter(|n| *n >= 0.0).map(|n| n as u64))
}

// ─── 原始记录（只声明用到的字段，其余由 serde 跳过）──────────────────────
