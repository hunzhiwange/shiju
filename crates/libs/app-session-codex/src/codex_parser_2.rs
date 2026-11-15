//! Codex rollout state transitions, migrated from the pinned source.
use super::*;

impl RolloutParser {
    pub(super) fn on_user_message(
        &mut self,
        contents: &Contents<'_>,
        id: Option<String>,
        ts: Option<i64>,
        span: JsonlSpan,
    ) {
        let mut blocks = Vec::new();
        // (注入文本, 在行内的 JSON Pointer)
        let mut injected: Vec<(String, String)> = Vec::new();
        let mut pending_text: Vec<String> = Vec::new();
        let flush_text = |pending: &mut Vec<String>, blocks: &mut Vec<SessionBlock>| {
            if !pending.is_empty() {
                blocks.push(SessionBlock::text(pending.join("\n")));
                pending.clear();
            }
        };
        let mut handle_text = |text: &str, pointer: String, pending: &mut Vec<String>| {
            let trimmed = text.trim();
            if trimmed.is_empty() || is_image_wrapper(trimmed) {
                return;
            }
            if INJECTED_USER_PREFIXES
                .iter()
                .any(|p| trimmed.starts_with(p))
                || is_wrapped_in_tag(trimmed)
            {
                injected.push((text.to_string(), pointer));
            } else if trimmed.starts_with(VSCODE_CONTEXT_PREFIX) {
                match extract_codex_prompt_from_ide_context(trimmed) {
                    Some(prompt) => pending.push(prompt),
                    None => injected.push((text.to_string(), pointer)),
                }
            } else {
                pending.push(text.to_string());
            }
        };
        match contents {
            Contents::Text(text) => handle_text(
                text.as_str(),
                "/payload/content".to_string(),
                &mut pending_text,
            ),
            Contents::Items(items) => {
                for (i, item) in items.iter().enumerate() {
                    if matches!(item.kind(), "input_image" | "image") {
                        let pointer = format!("/payload/content/{i}/image_url");
                        if let Some(image) = opt_str(&item.image_url)
                            .and_then(|url| image_from_url(url, span, pointer))
                        {
                            flush_text(&mut pending_text, &mut blocks);
                            blocks.push(SessionBlock::Image { image });
                        }
                    } else if let Some(text) = opt_str(&item.text) {
                        handle_text(
                            text,
                            format!("/payload/content/{i}/text"),
                            &mut pending_text,
                        );
                    }
                }
            }
        }
        flush_text(&mut pending_text, &mut blocks);

        if !injected.is_empty() {
            // 每段注入文本单独成块，超长的（AGENTS.md 之类）只放预览 + 引用
            let injected_blocks = injected
                .into_iter()
                .map(|(text, pointer)| large_text_block(text, || Some(span.content_ref(pointer))))
                .collect();
            self.push("user", ts, id.clone(), injected_blocks, true);
        }
        if !blocks.is_empty() {
            self.push("user", ts, id, blocks, false);
        }
    }

    pub(super) fn on_reasoning(&mut self, p: ReasoningPayload<'_>, ts: Option<i64>, span: JsonlSpan) {
        let summary = p
            .summary
            .as_ref()
            .map(|s| s.joined_text("\n\n"))
            .unwrap_or_default();
        let content = p
            .content
            .as_ref()
            .map(|c| c.joined_text("\n\n"))
            .unwrap_or_default();
        let redacted = summary.trim().is_empty() && content.trim().is_empty();
        if redacted && self.last_block_is_redacted_thinking() {
            return;
        }
        let text = preview_chars(&content, THINKING_PREVIEW_CHARS);
        let full = match p.content.as_ref() {
            Some(c) if text.truncated => Some(span.content_ref(c.text_pointer("/payload/content"))),
            _ => None,
        };
        let block = SessionBlock::Thinking {
            text: text.text,
            summary: (!summary.trim().is_empty()).then_some(summary),
            redacted,
            duration_ms: None,
            full,
        };
        self.push_assistant(ts, owned(&p.id), vec![block]);
    }

    pub(super) fn last_block_is_redacted_thinking(&self) -> bool {
        self.messages.last().is_some_and(|m| {
            m.role == "assistant"
                && m.turn_id == self.turn_id
                && matches!(
                    m.blocks.last(),
                    Some(SessionBlock::Thinking { redacted: true, .. })
                )
        })
    }

    pub(super) fn on_agent_message(&mut self, p: AgentMessagePayload<'_>, ts: Option<i64>) {
        let text = p
            .content
            .as_ref()
            .map(|c| c.joined_text("\n"))
            .unwrap_or_default();
        let text = text.trim();
        let (first, rest) = text.split_once('\n').unwrap_or((text, ""));
        let author = opt_str(&p.author).unwrap_or_default();
        let event = if author.is_empty() {
            first.trim().to_string()
        } else {
            format!("{author}: {}", first.trim())
        };
        let mut blocks = vec![SessionBlock::event(EventKind::SubAgent, Some(event), None)];
        if !rest.trim().is_empty() {
            blocks.push(SessionBlock::text(rest.trim().to_string()));
        }
        self.push("system", ts, owned(&p.id), blocks, false);
    }

    pub(super) fn on_web_search(&mut self, p: ActionPayload<'_>, ts: Option<i64>) {
        let id = owned(&p.id)
            .or_else(|| self.last_web_search_id.take())
            .unwrap_or_else(|| format!("ws_{}", self.messages.len()));
        self.last_web_search_id = None;
        let action = p.action.unwrap_or(Value::Null);
        let input = if action.is_null() {
            String::new()
        } else {
            action.to_string()
        };
        let status = match opt_str(&p.status).map(status_from_str) {
            Some(ToolStatus::Error) => ToolStatus::Error,
            Some(ToolStatus::Interrupted) => ToolStatus::Interrupted,
            _ => ToolStatus::Success,
        };
        let call = CallSpec::new(
            id.clone(),
            "web_search",
            ToolKind::Web,
            web_action_title(&action),
            &input,
        );
        let result = ResultSpec::empty(id.clone(), status);
        self.push_assistant(ts, Some(id), vec![call.into_block(), result.into_block()]);
    }

    // ─── 工具调用 ────────────────────────────────────────────────────

    pub(super) fn register_call(
        &mut self,
        ts: Option<i64>,
        id: Option<String>,
        call_id: String,
        block: SessionBlock,
        flavor: CallFlavor,
        cmds: Vec<String>,
    ) {
        let (msg_idx, block_idx) = self.push_assistant(ts, id, vec![block]);
        self.open.retain(|c| *c != call_id);
        self.open.push(call_id.clone());
        self.calls.insert(
            call_id,
            PendingCall {
                msg_idx,
                block_idx,
                flavor,
                cmds,
                items: Vec::new(),
            },
        );
    }

    pub(super) fn on_function_call(&mut self, p: FunctionCallPayload<'_>, ts: Option<i64>, span: JsonlSpan) {
        let name = opt_str(&p.name).unwrap_or("unknown");
        let namespace = opt_str(&p.namespace);
        let arguments = opt_str(&p.arguments).unwrap_or_default();
        let args: Value = serde_json::from_str(arguments).unwrap_or(Value::Null);
        let call_id = owned(&p.call_id)
            .or_else(|| owned(&p.id))
            .unwrap_or_default();

        let normalized = normalize_tool(ToolSource::Codex, name, namespace);
        let str_arg = |key: &str| {
            args.get(key)
                .and_then(Value::as_str)
                .filter(|s| !s.trim().is_empty())
        };
        let mut detail = None;
        let mut cmds = Vec::new();
        let mut flavor = CallFlavor::Other;
        let mut kind = normalized.kind;
        let title = match kind {
            ToolKind::Mcp => {
                flavor = CallFlavor::Mcp;
                detail = first_string_field(&args);
                title_mcp(normalized.server.as_deref().unwrap_or("mcp"), name)
            }
            ToolKind::Shell if name == "write_stdin" => match str_arg("chars") {
                Some(chars) => title_shell(chars),
                None => match args.get("session_id") {
                    Some(id) if !id.is_null() => format!("session {id}"),
                    _ => title_other(name),
                },
            },
            ToolKind::Shell => {
                flavor = CallFlavor::Shell;
                let cmd = args
                    .get("cmd")
                    .or_else(|| args.get("command"))
                    .map(command_from_value)
                    .unwrap_or_default();
                detail = str_arg("workdir").map(str::to_string);
                kind = refine_shell_kind(&cmd);
                let title = title_shell(&cmd);
                if !cmd.is_empty() {
                    cmds.push(cmd);
                }
                title
            }
            ToolKind::Ask => {
                let question = args
                    .get("questions")
                    .and_then(Value::as_array)
                    .and_then(|q| q.first())
                    .and_then(|q| {
                        ["title", "question", "header"]
                            .iter()
                            .find_map(|k| q.get(*k).and_then(Value::as_str))
                    })
                    .or_else(|| str_arg("question"));
                question.map(title_ask).unwrap_or_else(|| title_other(name))
            }
            ToolKind::Agent => {
                detail = str_arg("model").map(str::to_string);
                ["task_name", "description", "agent_type", "target", "name"]
                    .iter()
                    .find_map(|k| str_arg(k))
                    .map(title_agent)
                    .unwrap_or_else(|| title_other(name))
            }
            ToolKind::Todo => {
                let count = args
                    .get("plan")
                    .or_else(|| args.get("todos"))
                    .and_then(Value::as_array)
                    .map_or(0, Vec::len);
                title_todo(count)
            }
            ToolKind::Read | ToolKind::Write | ToolKind::Edit => {
                match str_arg("path").or_else(|| str_arg("file_path")) {
                    Some(path) if kind == ToolKind::Read => {
                        let num = |k: &str| args.get(k).and_then(Value::as_u64);
                        let (title, range) = title_read(path, num("offset"), num("limit"));
                        detail = range;
                        title
                    }
                    Some(path) => title_path(path),
                    None => title_other(name),
                }
            }
            ToolKind::Web => ["query", "url", "q"]
                .iter()
                .find_map(|k| str_arg(k))
                .map(|q| title_web(&[q]))
                .unwrap_or_else(|| title_other(name)),
            _ => {
                detail = first_string_field(&args);
                title_other(name)
            }
        };

        let block = CallSpec {
            detail,
            server: normalized.server,
            input_ref: Some(span.content_ref("/payload/arguments")),
            ..CallSpec::new(call_id.clone(), name, kind, title, arguments)
        }
        .into_block();
        self.register_call(ts, owned(&p.id), call_id, block, flavor, cmds);
    }

    pub(super) fn on_custom_call(&mut self, p: CustomCallPayload<'_>, ts: Option<i64>, span: JsonlSpan) {
        let name = opt_str(&p.name).unwrap_or("unknown");
        let input = opt_str(&p.input).unwrap_or_default();
        let call_id = owned(&p.call_id)
            .or_else(|| owned(&p.id))
            .unwrap_or_default();

        let mut spec = CallSpec {
            input_ref: Some(span.content_ref("/payload/input")),
            ..CallSpec::new(
                call_id.clone(),
                name,
                ToolKind::Other,
                title_other(name),
                input,
            )
        };
        let (flavor, cmds) = match name {
            "exec" => {
                let script = parse_exec_script(input);
                let (kind, title) = exec_title(&script, input);
                spec.kind = kind;
                spec.title = title;
                spec.detail = script.workdir.clone();
                spec.server = script.mcp.as_ref().map(|(server, _)| server.clone());
                (CallFlavor::Exec, script.cmds)
            }
            "apply_patch" => {
                // 先按补丁文本统计；匹配到 FileChange 时再用其 unified_diff 替换
                let diff = diff_summary(
                    parse_apply_patch(input),
                    Some(span.content_ref("/payload/input")),
                );
                spec.kind = diff_kind(&diff);
                spec.title = diff_title(&diff);
                spec.diff = Some(diff);
                (CallFlavor::ApplyPatch, Vec::new())
            }
            _ => {
                let normalized = normalize_tool(ToolSource::Codex, name, None);
                spec.kind = normalized.kind;
                spec.server = normalized.server;
                (CallFlavor::Other, Vec::new())
            }
        };
        let block = spec.into_block();
        self.register_call(ts, owned(&p.id), call_id, block, flavor, cmds);
    }

    pub(super) fn on_local_shell(&mut self, p: ActionPayload<'_>, ts: Option<i64>, span: JsonlSpan) {
        let action = p.action.unwrap_or(Value::Null);
        let cmd = action
            .get("command")
            .map(command_from_value)
            .unwrap_or_default();
        let input = action.to_string();
        let call_id = owned(&p.call_id)
            .or_else(|| owned(&p.id))
            .unwrap_or_default();
        let block = CallSpec {
            detail: action
                .get("working_directory")
                .and_then(Value::as_str)
                .map(str::to_string),
            input_ref: Some(span.content_ref("/payload/action")),
            ..CallSpec::new(
                call_id.clone(),
                "local_shell",
                refine_shell_kind(&cmd),
                title_shell(&cmd),
                &input,
            )
        }
        .into_block();
        let cmds = if cmd.is_empty() {
            Vec::new()
        } else {
            vec![cmd]
        };
        self.register_call(ts, owned(&p.id), call_id, block, CallFlavor::Shell, cmds);
    }

    // ─── 结构化项 ────────────────────────────────────────────────────

}
