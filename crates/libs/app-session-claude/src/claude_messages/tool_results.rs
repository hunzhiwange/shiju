use super::*;

impl TranscriptBuilder {
    pub(super) fn tool_result_block(
        &mut self,
        item: RawItem<'_>,
        index: usize,
        tool_use_result: Option<&ToolUseResult>,
        ts: Option<i64>,
        line: JsonlSpan,
    ) -> SessionBlock {
        let call_id = item.tool_use_id.as_str().unwrap_or_default().to_string();
        let base = format!("/message/content/{index}/content");
        let mut images = Vec::new();
        let (text, pointer): (Cow<'_, str>, Option<String>) = match item.content {
            RawContent::Text(text) => (text, Some(base)),
            RawContent::Items(parts) => {
                let mut texts: Vec<(usize, Cow<'_, str>)> = Vec::new();
                for (j, part) in parts.into_iter().enumerate() {
                    let Some(part) = part.0 else { continue };
                    match part.kind.as_str().unwrap_or_default() {
                        "text" => {
                            if let Some(text) = part.text.0 {
                                texts.push((j, text));
                            }
                        }
                        "image" => {
                            if let Some(image) = image_ref(
                                part.source.0.as_ref(),
                                format!("{base}/{j}/source/data"),
                                line,
                            ) {
                                images.push(image);
                            }
                        }
                        _ => {}
                    }
                }
                match texts.len() {
                    0 => (Cow::Borrowed(""), None),
                    // 只有一段文本时直接指向字符串，P2 不用再拼接
                    1 => {
                        let (j, text) = texts.pop().expect("len checked");
                        (text, Some(format!("{base}/{j}/text")))
                    }
                    _ => {
                        let joined = texts
                            .iter()
                            .map(|(_, t)| t.as_ref())
                            .collect::<Vec<_>>()
                            .join("\n");
                        (Cow::Owned(joined), Some(base))
                    }
                }
            }
            RawContent::None => (Cow::Borrowed(""), None),
        };
        let text = if text.contains(SYSTEM_REMINDER_OPEN) {
            Cow::Owned(strip_system_reminders(&text))
        } else {
            text
        };
        let p = preview(&text);

        let is_error = item.is_error.as_ref().and_then(Value::as_bool) == Some(true);
        let interrupted = tool_use_result
            .and_then(|r| r.interrupted.as_ref())
            .and_then(Value::as_bool)
            == Some(true);
        let status = if interrupted {
            ToolStatus::Interrupted
        } else if is_error {
            ToolStatus::Error
        } else {
            ToolStatus::Success
        };

        let slot = self.calls.get(&call_id);
        let is_shell = slot.is_some_and(|s| s.name == "Bash");
        // 退出码与耗时只对 Bash 有意义（其余工具的「结果时间 - 调用时间」多是排队与思考时间）
        let exit_code = parse_exit_code(&text).or_else(|| {
            let background = tool_use_result.is_some_and(|r| r.background_task_id.is_some());
            (is_shell && status == ToolStatus::Success && !background).then_some(0)
        });
        let duration_ms = match (is_shell, slot.and_then(|s| s.ts), ts) {
            (true, Some(start), Some(end)) if end >= start => Some((end - start) as u64),
            _ => None,
        };

        let saved_path = tool_use_result
            .and_then(|r| r.persisted_output_path.as_ref())
            .and_then(Value::as_str)
            .filter(|p| !p.is_empty())
            .map(str::to_string);
        let full = if p.truncated {
            saved_path
                .as_deref()
                .and_then(|path| self.sidecar_rel_path(path))
                .map(|rel_path| ContentRef::Sidecar { rel_path })
                .or_else(|| pointer.map(|ptr| line.content_ref(ptr)))
        } else {
            None
        };

        self.backfill_call(&call_id, status, tool_use_result);

        SessionBlock::ToolResult {
            call_id,
            status,
            preview: p.text,
            total_len: p.total_len,
            line_count: p.line_count,
            truncated: p.truncated,
            full,
            exit_code,
            duration_ms,
            images,
            saved_path,
        }
    }

    /// 结果到来时回填对应 ToolCall：Edit/Write 用 `structuredPatch` 修正 diff，
    /// 失败的改动去掉 diff；Agent 用实际模型补 detail。
    pub(super) fn backfill_call(&mut self, call_id: &str, status: ToolStatus, result: Option<&ToolUseResult>) {
        let Some(slot) = self.calls.get(call_id) else {
            return;
        };
        let Some(SessionBlock::ToolCall {
            raw_name,
            detail,
            diff,
            ..
        }) = self
            .drafts
            .get_mut(slot.draft)
            .and_then(|d| d.blocks.get_mut(slot.block))
        else {
            return;
        };

        match raw_name.as_str() {
            "Edit" | "MultiEdit" | "Write" => {
                if status != ToolStatus::Success {
                    *diff = None;
                    return;
                }
                let Some(result) = result else { return };
                let op = match result.kind.as_ref().and_then(Value::as_str) {
                    Some("create") => Some(DiffOp::Add),
                    Some("update") => Some(DiffOp::Update),
                    _ => None,
                };
                let path = result
                    .file_path
                    .as_ref()
                    .and_then(Value::as_str)
                    .map(str::to_string);
                let patch = result.structured_patch.filter(|p| p.added + p.removed > 0);
                if op.is_none() && patch.is_none() {
                    return;
                }
                let summary = diff.get_or_insert_with(|| DiffSummary {
                    files: vec![DiffFile {
                        path: String::new(),
                        op: DiffOp::Update,
                        added: 0,
                        removed: 0,
                    }],
                    added: 0,
                    removed: 0,
                    full: None,
                });
                if let Some(file) = summary.files.first_mut() {
                    if let Some(op) = op {
                        file.op = op;
                    }
                    if let Some(path) = path {
                        file.path = path;
                    }
                    if let Some(patch) = patch {
                        file.added = patch.added;
                        file.removed = patch.removed;
                    }
                    summary.added = file.added;
                    summary.removed = file.removed;
                }
            }
            "Agent" | "Task" if detail.is_none() => {
                *detail = result
                    .and_then(|r| r.resolved_model.as_ref())
                    .and_then(Value::as_str)
                    .filter(|m| !m.is_empty())
                    .map(str::to_string);
            }
            _ => {}
        }
    }

    /// `persistedOutputPath` 在会话附属目录内时，转成相对路径（`ContentRef::Sidecar`）。
    pub(super) fn sidecar_rel_path(&self, saved: &str) -> Option<String> {
        Path::new(saved)
            .strip_prefix(&self.sidecar_dir)
            .ok()
            .filter(|rel| rel.components().all(|part| matches!(part, std::path::Component::Normal(_))))
            .map(|rel| rel.to_string_lossy().replace('\\', "/"))
            .filter(|rel| !rel.is_empty())
    }
}
