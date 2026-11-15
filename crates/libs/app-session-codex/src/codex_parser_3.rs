//! Codex rollout state transitions, migrated from the pinned source.
use super::*;

impl RolloutParser {
    pub(super) fn on_item(&mut self, item: RawItem<'_>, ts: Option<i64>, span: JsonlSpan) {
        match item.ty.as_str() {
            "AgentMessage" => {
                if let (Some(id), Some(phase)) = (owned(&item.id), owned(&item.phase)) {
                    self.phases.insert(id, phase);
                }
                return;
            }
            "ContextCompaction" => {
                self.compaction_event(ts, None);
                return;
            }
            "WebSearch" => {
                // 紧跟的 web_search_call 已带 action，这里只借 id
                self.last_web_search_id = owned(&item.id);
                return;
            }
            _ => {}
        }
        let item_id = owned(&item.id).unwrap_or_default();
        let Some(record) = item.into_record(span) else {
            return;
        };

        // 1. id 直接命中调用：exec_command 的 CommandExecution、McpToolCall、sleep 的 Extension
        if let Some(call) = self.calls.get_mut(&item_id) {
            call.items.push(record);
            return;
        }
        // 1b. 调用已经随输出结束（输出先于 item_completed 到达）：只回填，不再成步骤
        if !item_id.is_empty() && self.finished.contains(&item_id) {
            if let ItemRecord::Command(info) = &record {
                self.backfill_finished_result(&item_id, info);
            }
            return;
        }
        // 2. 按类型挂到最近的未完成调用
        let claim = match &record {
            ItemRecord::Command(info) => self.claim_command(info),
            ItemRecord::FileChange(_) => Claim::from(
                self.latest_open(&[CallFlavor::ApplyPatch])
                    .or_else(|| self.latest_open(&[CallFlavor::Exec])),
            ),
            ItemRecord::Mcp(_) | ItemRecord::Web(_) => {
                Claim::from(self.latest_open(&[CallFlavor::Exec]))
            }
            ItemRecord::ImageView { .. } => Claim::from(self.open.last().cloned()),
        };
        match (claim, record) {
            (Claim::Pool, ItemRecord::Command(info)) => self.pool.push(info),
            (Claim::Call(call_id), record) => match self.calls.get_mut(&call_id) {
                Some(call) => call.items.push(record),
                None => self.emit_standalone(record, ts),
            },
            (_, record) => self.emit_standalone(record, ts),
        }
    }

    pub(super) fn latest_open(&self, flavors: &[CallFlavor]) -> Option<String> {
        self.open
            .iter()
            .rev()
            .find(|id| {
                self.calls
                    .get(*id)
                    .is_some_and(|c| flavors.contains(&c.flavor))
            })
            .cloned()
    }

    // CommandExecution 归属：命令文本匹配的调用 → 唯一一个看不出命令的调用 →
    // 多个看不出命令的调用（先放池子）→ 都不是（后台进程结束等，单独成块）
    pub(super) fn claim_command(&self, info: &CommandInfo) -> Claim {
        let candidates: Vec<(&String, &PendingCall)> = self
            .open
            .iter()
            .rev()
            .filter_map(|id| self.calls.get(id).map(|c| (id, c)))
            .filter(|(_, c)| matches!(c.flavor, CallFlavor::Exec | CallFlavor::Shell))
            .collect();
        if let Some((id, _)) = candidates
            .iter()
            .find(|(_, c)| c.cmds.iter().any(|cmd| same_command(cmd, &info.command)))
        {
            return Claim::Call((*id).clone());
        }
        let unknown: Vec<&String> = candidates
            .iter()
            .filter(|(_, c)| c.cmds.is_empty())
            .map(|(id, _)| *id)
            .collect();
        match unknown.as_slice() {
            [] => Claim::None,
            [id] => Claim::Call((*id).clone()),
            _ => Claim::Pool,
        }
    }

    // 迟到的 CommandExecution：把退出码、耗时补进已输出的结果（只补缺的字段）
    pub(super) fn backfill_finished_result(&mut self, call_id: &str, info: &CommandInfo) {
        for message in self.messages.iter_mut().rev() {
            for block in message.blocks.iter_mut() {
                if let SessionBlock::ToolResult {
                    call_id: id,
                    status,
                    exit_code,
                    duration_ms,
                    ..
                } = block
                {
                    if id == call_id {
                        if exit_code.is_none() {
                            *exit_code = info.exit_code;
                        }
                        if duration_ms.is_none() {
                            *duration_ms = info.duration_ms;
                        }
                        // 输出先到时只能判为成功，失败 / 中断以迟到的结构化项为准
                        if *status == ToolStatus::Success {
                            if matches!(info.status, ToolStatus::Error | ToolStatus::Interrupted) {
                                *status = info.status;
                            } else if exit_code.is_some_and(|c| c != 0) {
                                *status = ToolStatus::Error;
                            }
                        }
                        return;
                    }
                }
            }
        }
    }

    // 找不到调用的结构化项：自己成一条助手消息（调用 + 结果）
    pub(super) fn emit_standalone(&mut self, record: ItemRecord, ts: Option<i64>) {
        let (id, blocks) = match record {
            ItemRecord::Command(info) => {
                let blocks = vec![
                    info.call_spec().into_block(),
                    info.result_spec(info.id.clone()).into_block(),
                ];
                (info.id, blocks)
            }
            ItemRecord::FileChange(info) => {
                let call = CallSpec {
                    diff: Some(info.diff.clone()),
                    ..CallSpec::new(
                        info.id.clone(),
                        "FileChange",
                        diff_kind(&info.diff),
                        diff_title(&info.diff),
                        "",
                    )
                };
                let result = ResultSpec::empty(info.id.clone(), info.status);
                (info.id, vec![call.into_block(), result.into_block()])
            }
            ItemRecord::Mcp(info) => {
                let call = CallSpec {
                    detail: info.detail.clone(),
                    server: Some(info.server.clone()),
                    ..CallSpec::new(
                        info.id.clone(),
                        info.tool.clone(),
                        ToolKind::Mcp,
                        title_mcp(&info.server, &info.tool),
                        &info.input,
                    )
                };
                let result = ResultSpec {
                    output: info.output.clone(),
                    full: info.output_ref.clone(),
                    duration_ms: info.duration_ms,
                    images: info.images.clone(),
                    ..ResultSpec::empty(info.id.clone(), info.status)
                };
                let blocks = vec![call.into_block(), result.into_block()];
                (info.id, blocks)
            }
            ItemRecord::ImageView { id, path } => (
                id,
                vec![SessionBlock::Image {
                    image: local_image(&path),
                }],
            ),
            ItemRecord::Web(info) => {
                let call = CallSpec::new(
                    info.id.clone(),
                    "web_search",
                    ToolKind::Web,
                    info.title.clone(),
                    &info.input,
                );
                let result = ResultSpec::empty(info.id.clone(), ToolStatus::Success);
                let blocks = vec![call.into_block(), result.into_block()];
                (info.id, blocks)
            }
        };
        let id = (!id.is_empty()).then_some(id);
        self.push_assistant(ts, id, blocks);
    }

    pub(super) fn flush_pool(&mut self, ts: Option<i64>) {
        for info in std::mem::take(&mut self.pool) {
            self.emit_standalone(ItemRecord::Command(info), ts);
        }
    }

    // ─── 输出与回填 ──────────────────────────────────────────────────

    pub(super) fn on_output(&mut self, p: OutputPayload<'_>, ts: Option<i64>, span: JsonlSpan) {
        let call_id = owned(&p.call_id).unwrap_or_default();
        let out = OutputInfo::from_contents(p.output.as_ref(), span);
        let id = owned(&p.id);
        match self.calls.remove(&call_id) {
            Some(call) => {
                self.open.retain(|c| *c != call_id);
                self.finish_call(call_id, call, Some(out), ts, id, false);
            }
            None => {
                let status = output_status(&out, None);
                let result = ResultSpec {
                    output: preview(&out.text),
                    full: out.full,
                    exit_code: out.exit_code,
                    duration_ms: out.duration_ms,
                    images: out.images,
                    ..ResultSpec::empty(call_id, status)
                };
                self.push("tool", ts, id, vec![result.into_block()], false);
            }
        }
    }

    // 用挂上的结构化项回填调用块，并输出结果。
    // `out = None` 时：`aborted` 为真输出中断结果，否则（文件末尾仍未完成）只回填调用块。
}
