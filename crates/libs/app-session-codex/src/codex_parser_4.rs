//! Codex rollout state transitions, migrated from the pinned source.
use super::*;

impl RolloutParser {
    pub(super) fn finish_call(
        &mut self,
        call_id: String,
        call: PendingCall,
        out: Option<OutputInfo>,
        ts: Option<i64>,
        msg_id: Option<String>,
        aborted: bool,
    ) {
        self.finished.insert(call_id.clone());
        let PendingCall {
            msg_idx,
            block_idx,
            flavor,
            items,
            ..
        } = call;
        let is_shell = matches!(flavor, CallFlavor::Exec | CallFlavor::Shell);
        // 补丁没通过校验 / 被用户拒绝：什么都没改，按失败显示，也不报增删行数
        let patch_rejected = flavor == CallFlavor::ApplyPatch
            && out.as_ref().is_some_and(|o| apply_patch_rejected(&o.text));

        let mut commands = Vec::new();
        let mut file_change: Option<FileChangeInfo> = None;
        let mut mcp = None;
        let mut web = None;
        let mut views = Vec::new();
        for item in items {
            match item {
                ItemRecord::Command(info) => commands.push(info),
                ItemRecord::FileChange(info) => match &mut file_change {
                    Some(existing) => {
                        let mut files = std::mem::take(&mut existing.diff.files);
                        files.extend(info.diff.files);
                        existing.diff = diff_summary(files, None);
                    }
                    None => file_change = Some(info),
                },
                ItemRecord::Mcp(info) => mcp = Some(info),
                ItemRecord::Web(info) => web = Some(info),
                ItemRecord::ImageView { path, .. } => views.push(path),
            }
        }
        // 兜底：输出内容里包含池中某条命令输出的开头 → 认领
        if let (Some(out), true, true) = (&out, is_shell, commands.is_empty()) {
            if let Some(pos) = self
                .pool
                .iter()
                .position(|c| !c.output_head.is_empty() && out.text.contains(&c.output_head))
            {
                commands.push(self.pool.remove(pos));
            }
        }

        // 1. 回填调用块
        if let Some(SessionBlock::ToolCall {
            kind,
            title,
            detail,
            server,
            diff,
            ..
        }) = self.messages[msg_idx].blocks.get_mut(block_idx)
        {
            if is_shell {
                if let [command] = commands.as_slice() {
                    *kind = command.kind;
                    *title = title_shell(&command.command);
                    *detail = command.cwd.clone().or(detail.take());
                } else if commands.is_empty() {
                    if let Some(fc) = &file_change {
                        *kind = diff_kind(&fc.diff);
                        *title = diff_title(&fc.diff);
                        *diff = Some(fc.diff.clone());
                    } else if let Some(m) = &mcp {
                        *kind = ToolKind::Mcp;
                        *title = title_mcp(&m.server, &m.tool);
                        *server = Some(m.server.clone());
                        *detail = m.detail.clone();
                    } else if let Some(w) = web.as_ref().filter(|w| !w.title.is_empty()) {
                        *kind = ToolKind::Web;
                        *title = w.title.clone();
                    }
                }
            } else if let (CallFlavor::ApplyPatch, Some(fc)) = (flavor, &file_change) {
                let mut merged = fc.diff.clone();
                if merged.full.is_none() {
                    merged.full = diff.as_ref().and_then(|d| d.full.clone());
                }
                *kind = diff_kind(&merged);
                *diff = Some(merged);
            }
            if patch_rejected {
                if let Some(diff) = diff {
                    diff.added = 0;
                    diff.removed = 0;
                    for file in &mut diff.files {
                        file.added = 0;
                        file.removed = 0;
                    }
                }
            }
        }
        // 多条命令：拆成 `call_id#1`、`call_id#2`… 多个子调用
        let result_ids: Vec<String> = if is_shell && commands.len() > 1 {
            let template = self.messages[msg_idx].blocks[block_idx].clone();
            let subs: Vec<SessionBlock> = commands
                .iter()
                .enumerate()
                .map(|(k, command)| {
                    let mut block = template.clone();
                    if let SessionBlock::ToolCall {
                        id,
                        kind,
                        title,
                        detail,
                        ..
                    } = &mut block
                    {
                        *id = format!("{call_id}#{}", k + 1);
                        *kind = command.kind;
                        *title = title_shell(&command.command);
                        *detail = command.cwd.clone().or(detail.take());
                    }
                    block
                })
                .collect();
            self.messages[msg_idx]
                .blocks
                .splice(block_idx..=block_idx, subs);
            (1..=commands.len())
                .map(|k| format!("{call_id}#{k}"))
                .collect()
        } else {
            vec![call_id.clone()]
        };

        if out.is_none() && !aborted {
            return;
        }

        // 2. 结果：前面的子命令用 CommandExecution 自带的输出，最后一个用工具输出
        let mut results: Vec<SessionBlock> = Vec::new();
        if result_ids.len() > 1 {
            for (command, id) in commands.iter().zip(&result_ids) {
                if Some(id) == result_ids.last() {
                    break;
                }
                results.push(command.result_spec(id.clone()).into_block());
            }
        }
        let last_id = result_ids.last().cloned().unwrap_or(call_id);
        let last_command = if is_shell { commands.last() } else { None };
        let extra_status = file_change
            .as_ref()
            .map(|f| f.status)
            .or(mcp.as_ref().map(|m| m.status));
        let spec = match out {
            Some(out) => {
                let mut status = output_status(&out, last_command);
                if let Some(extra @ (ToolStatus::Error | ToolStatus::Interrupted)) = extra_status {
                    status = extra;
                }
                if patch_rejected {
                    status = ToolStatus::Error;
                }
                let images = if !out.images.is_empty() {
                    out.images
                } else {
                    let mut images = mcp.as_ref().map(|m| m.images.clone()).unwrap_or_default();
                    images.extend(views.iter().map(|p| local_image(p)));
                    images
                };
                ResultSpec {
                    output: preview(&out.text),
                    full: out.full,
                    exit_code: last_command.and_then(|c| c.exit_code).or(out.exit_code),
                    duration_ms: last_command
                        .and_then(|c| c.duration_ms)
                        .or(out.duration_ms)
                        .or(mcp.as_ref().and_then(|m| m.duration_ms)),
                    images,
                    ..ResultSpec::empty(last_id, status)
                }
            }
            None => match last_command {
                Some(command) => {
                    let mut spec = command.result_spec(last_id);
                    if matches!(spec.status, ToolStatus::Unknown | ToolStatus::Pending) {
                        spec.status = ToolStatus::Interrupted;
                    }
                    spec
                }
                None => ResultSpec::empty(last_id, ToolStatus::Interrupted),
            },
        };
        results.push(spec.into_block());
        self.push("tool", ts, msg_id, results, false);
    }

    // ─── 收尾 ────────────────────────────────────────────────────────

    pub(super) fn finish(mut self) -> Vec<SessionMessage> {
        // 文件末尾仍未完成的调用：只回填标题等，不伪造结果（前端显示进行中）
        for call_id in std::mem::take(&mut self.open) {
            if let Some(call) = self.calls.remove(&call_id) {
                self.finish_call(call_id, call, None, None, None, false);
            }
        }
        self.flush_pool(None);
        self.flush_model_event();

        // 每轮最后一条带正文的助手消息挂上模型与 token 用量
        let mut final_by_turn: HashMap<&str, usize> = HashMap::new();
        for (idx, msg) in self.messages.iter().enumerate() {
            let has_text = msg
                .blocks
                .iter()
                .any(|b| matches!(b, SessionBlock::Text { .. }));
            if let (true, true, Some(turn)) = (msg.role == "assistant", has_text, &msg.turn_id) {
                final_by_turn.insert(turn.as_str(), idx);
            }
        }
        let targets: Vec<(String, usize)> = final_by_turn
            .into_iter()
            .map(|(turn, idx)| (turn.to_string(), idx))
            .collect();
        for (turn, idx) in targets {
            let model = self.turn_models.get(&turn).cloned();
            let usage = self.turn_usage.get(&turn).copied();
            if model.is_none() && usage.is_none() {
                continue;
            }
            let meta = self.messages[idx]
                .meta
                .get_or_insert_with(MessageMeta::default);
            meta.model = model;
            if let Some(usage) = usage {
                let nonzero = |n: u64| (n > 0).then_some(n);
                meta.input_tokens = Some(usage.input_tokens);
                meta.output_tokens = Some(usage.output_tokens);
                meta.cache_read_tokens = nonzero(usage.cached_input_tokens);
                meta.cache_write_tokens = nonzero(usage.cache_write_input_tokens);
                meta.reasoning_tokens = nonzero(usage.reasoning_output_tokens);
            }
        }

        let phases = std::mem::take(&mut self.phases);
        let mut messages = self.messages;
        for msg in &mut messages {
            if msg.role == "assistant" {
                if let Some(phase) = msg.id.as_ref().and_then(|id| phases.get(id)) {
                    let meta = msg.meta.get_or_insert_with(MessageMeta::default);
                    if meta.stop_reason.is_none() {
                        meta.stop_reason = Some(phase.clone());
                    }
                }
            }
            msg.content = project_content(&msg.blocks);
        }
        messages.retain(|m| !m.is_empty());
        messages
    }
}
