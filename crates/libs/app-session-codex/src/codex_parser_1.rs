//! Codex rollout state transitions, migrated from the pinned source.
use super::*;

impl RolloutParser {
    pub(super) fn process_line(&mut self, line: &[u8], span: JsonlSpan) {
        let (top, sub): (Cow<str>, Option<Cow<str>>) = match sniff_types(line) {
            Some((top, Some(sub))) => (Cow::Borrowed(top), Some(Cow::Borrowed(sub))),
            Some((top, None)) if !matches!(top, "response_item" | "event_msg") => {
                (Cow::Borrowed(top), None)
            }
            _ => {
                let Ok(head) = serde_json::from_slice::<Head>(line) else {
                    return;
                };
                let Some(top) = head.ty else {
                    return;
                };
                (top.0, head.payload.and_then(|p| p.ty).map(|t| t.0))
            }
        };
        let sub = sub.as_deref().unwrap_or_default();
        match top.as_ref() {
            "response_item" => self.response_item(sub, line, span),
            "event_msg" => self.event_msg(sub, line, span),
            "turn_context" => {
                if let Some((_, p)) = parse_line::<TurnContextPayload>(line) {
                    if let Some(turn) = owned(&p.turn_id) {
                        self.start_turn(turn);
                    }
                    if let Some(model) = owned(&p.model) {
                        if let Some(turn) = &self.turn_id {
                            self.turn_models.insert(turn.clone(), model.clone());
                        }
                        self.last_model = Some(model);
                    }
                }
            }
            "token_usage_record" => {
                if let Some((_, p)) = parse_line::<UsageRecordPayload>(line) {
                    self.usage_from_records = true;
                    let turn = owned(&p.turn_id).or_else(|| self.turn_id.clone());
                    if let (Some(turn), Some(usage)) = (turn, p.turn_token_usage) {
                        self.turn_usage.insert(turn, usage);
                    }
                }
            }
            "compacted" => {
                if let Some((ts, p)) = parse_line::<CompactedPayload>(line) {
                    let summary = opt_str(&p.message)
                        .filter(|m| !m.trim().is_empty())
                        .map(|m| {
                            summary_event_block(EventKind::Compaction, m, || {
                                Some(span.content_ref("/payload/message"))
                            })
                        });
                    self.compaction_event(ts, summary);
                }
            }
            // session_meta / world_state / inter_agent_communication_metadata …：不产生消息
            _ => {}
        }
    }

    pub(super) fn response_item(&mut self, sub: &str, line: &[u8], span: JsonlSpan) {
        match sub {
            "message" => {
                if let Some((ts, p)) = parse_line::<MessagePayload>(line) {
                    self.on_message(p, ts, span);
                }
            }
            "reasoning" => {
                if let Some((ts, p)) = parse_line::<ReasoningPayload>(line) {
                    self.on_reasoning(p, ts, span);
                }
            }
            "function_call" => {
                if let Some((ts, p)) = parse_line::<FunctionCallPayload>(line) {
                    self.on_function_call(p, ts, span);
                }
            }
            "custom_tool_call" => {
                if let Some((ts, p)) = parse_line::<CustomCallPayload>(line) {
                    self.on_custom_call(p, ts, span);
                }
            }
            "function_call_output" | "custom_tool_call_output" => {
                if let Some((ts, p)) = parse_line::<OutputPayload>(line) {
                    self.on_output(p, ts, span);
                }
            }
            "local_shell_call" => {
                if let Some((ts, p)) = parse_line::<ActionPayload>(line) {
                    self.on_local_shell(p, ts, span);
                }
            }
            "web_search_call" => {
                if let Some((ts, p)) = parse_line::<ActionPayload>(line) {
                    self.on_web_search(p, ts);
                }
            }
            "compaction" => {
                if let Some((ts, _)) = parse_line::<serde::de::IgnoredAny>(line) {
                    self.compaction_event(ts, None);
                }
            }
            "agent_message" => {
                if let Some((ts, p)) = parse_line::<AgentMessagePayload>(line) {
                    self.on_agent_message(p, ts);
                }
            }
            _ => {}
        }
    }

    pub(super) fn event_msg(&mut self, sub: &str, line: &[u8], span: JsonlSpan) {
        if sub == "item_completed" {
            if let Some((ts, p)) = parse_line::<ItemCompletedPayload>(line) {
                self.on_item(p.item, ts, span);
            }
            return;
        }
        if !matches!(
            sub,
            "task_started"
                | "task_complete"
                | "turn_aborted"
                | "thread_settings_applied"
                | "token_count"
        ) {
            return;
        }
        let Some((ts, p)) = parse_line::<EventPayload>(line) else {
            return;
        };
        match sub {
            "task_started" => {
                if let Some(turn) = owned(&p.turn_id) {
                    self.start_turn(turn);
                }
            }
            "task_complete" => self.flush_pool(ts),
            "turn_aborted" => {
                let reason = match p.reason {
                    Some(Value::String(reason)) => reason,
                    Some(other) if !other.is_null() => other.to_string(),
                    _ => "interrupted".to_string(),
                };
                self.abort_turn(ts, reason);
            }
            "thread_settings_applied" => {
                if let Some(model) = p.thread_settings.as_ref().and_then(|s| owned(&s.model)) {
                    self.note_model(model, ts);
                }
            }
            "token_count" => {
                if self.usage_from_records {
                    return;
                }
                let usage = p.info.and_then(|i| i.last_token_usage);
                if let (Some(turn), Some(usage)) = (self.turn_id.clone(), usage) {
                    self.turn_usage.entry(turn).or_default().add(&usage);
                }
            }
            _ => {}
        }
    }

    // ─── 轮次 ────────────────────────────────────────────────────────

    pub(super) fn start_turn(&mut self, turn: String) {
        if self.turn_id.as_deref() != Some(turn.as_str()) {
            self.flush_pool(None);
            if let Some(model) = &self.last_model {
                self.turn_models
                    .entry(turn.clone())
                    .or_insert_with(|| model.clone());
            }
            self.turn_id = Some(turn);
        }
        self.flush_model_event();
    }

    // 模型与上一个不同才记一次 ModelChange（第一次只作为基准）
    pub(super) fn note_model(&mut self, model: String, ts: Option<i64>) {
        if self.last_model.as_ref().is_some_and(|last| *last != model) {
            self.pending_model_event = Some((model.clone(), ts));
        }
        self.last_model = Some(model);
    }

    pub(super) fn flush_model_event(&mut self) {
        if let Some((model, ts)) = self.pending_model_event.take() {
            self.push(
                "system",
                ts,
                None,
                vec![SessionBlock::event(
                    EventKind::ModelChange,
                    Some(model),
                    None,
                )],
                false,
            );
        }
    }

    pub(super) fn abort_turn(&mut self, ts: Option<i64>, reason: String) {
        for call_id in std::mem::take(&mut self.open) {
            if let Some(call) = self.calls.remove(&call_id) {
                self.finish_call(call_id, call, None, ts, None, true);
            }
        }
        self.flush_pool(ts);
        self.push(
            "system",
            ts,
            None,
            vec![SessionBlock::event(EventKind::Aborted, Some(reason), None)],
            false,
        );
    }

    // `summary` 为带摘要的 Compaction 事件块（`compacted` 记录）；其余来源只标记发生过压缩。
    pub(super) fn compaction_event(&mut self, ts: Option<i64>, summary: Option<SessionBlock>) {
        if let Some(idx) = self.last_compaction {
            // 同一次压缩的多种记录：补上缺的摘要即可
            if let (Some(first), Some(summary)) = (self.messages[idx].blocks.first_mut(), summary) {
                if matches!(first, SessionBlock::Event { text: None, .. }) {
                    *first = summary;
                }
            }
            return;
        }
        let block =
            summary.unwrap_or_else(|| SessionBlock::event(EventKind::Compaction, None, None));
        let idx = self.push("system", ts, None, vec![block], false);
        self.last_compaction = Some(idx);
    }

    // ─── 消息输出 ────────────────────────────────────────────────────

    pub(super) fn push(
        &mut self,
        role: &str,
        ts: Option<i64>,
        id: Option<String>,
        blocks: Vec<SessionBlock>,
        injected: bool,
    ) -> usize {
        self.flush_model_event();
        self.last_compaction = None;
        self.messages.push(SessionMessage {
            role: role.to_string(),
            ts,
            id,
            turn_id: self.turn_id.clone(),
            injected,
            blocks,
            ..SessionMessage::default()
        });
        self.messages.len() - 1
    }

    // 助手侧块：前一条是同一轮「只有思考」的助手消息时并入，否则新起一条。
    // 返回 `(消息下标, 首个新块下标)`。
    pub(super) fn push_assistant(
        &mut self,
        ts: Option<i64>,
        id: Option<String>,
        blocks: Vec<SessionBlock>,
    ) -> (usize, usize) {
        self.flush_model_event();
        let turn_id = self.turn_id.clone();
        if let Some(last) = self.messages.last_mut() {
            let thinking_only = !last.blocks.is_empty()
                && last
                    .blocks
                    .iter()
                    .all(|b| matches!(b, SessionBlock::Thinking { .. }));
            if last.role == "assistant" && last.turn_id == turn_id && thinking_only {
                let start = last.blocks.len();
                last.blocks.extend(blocks);
                last.ts = ts.or(last.ts);
                if id.is_some() {
                    last.id = id;
                }
                self.last_compaction = None;
                return (self.messages.len() - 1, start);
            }
        }
        (self.push("assistant", ts, id, blocks, false), 0)
    }

    pub(super) fn on_message(&mut self, p: MessagePayload<'_>, ts: Option<i64>, span: JsonlSpan) {
        let id = owned(&p.id);
        let Some(contents) = p.content.as_ref() else {
            return;
        };
        match opt_str(&p.role).unwrap_or_default() {
            "user" => self.on_user_message(contents, id, ts, span),
            "assistant" => {
                let text = contents.joined_text("\n");
                if text.trim().is_empty() {
                    return;
                }
                let (idx, _) = self.push_assistant(ts, id, vec![SessionBlock::text(text)]);
                if let Some(phase) = owned(&p.phase) {
                    self.messages[idx]
                        .meta
                        .get_or_insert_with(MessageMeta::default)
                        .stop_reason = Some(phase);
                }
            }
            // developer / system：系统注入，默认隐藏；超长时只放预览 + 引用
            _ => {
                let text = contents.joined_text("\n");
                if !text.trim().is_empty() {
                    let block = large_text_block(text, || {
                        Some(span.content_ref(contents.text_pointer("/payload/content")))
                    });
                    self.push("system", ts, id, vec![block], true);
                }
            }
        }
    }

}
