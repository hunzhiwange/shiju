use super::*;

impl RawItem<'_> {
    /// 转成 [`ItemRecord`]；Reasoning / AgentMessage / UserMessage 等由 response_item
    /// 承担的类型返回 `None`。
    pub fn into_record(self, span: JsonlSpan) -> Option<ItemRecord> {
        let id = opt_str(&self.id).unwrap_or_default().to_string();
        match self.ty.as_str() {
            "CommandExecution" => Some(ItemRecord::Command(self.command_info(id, span))),
            "FileChange" => self.file_change_info(id, span).map(ItemRecord::FileChange),
            "McpToolCall" => Some(ItemRecord::Mcp(self.mcp_info(id, span))),
            "ImageView" => {
                let path = opt_str(&self.path)?;
                Some(ItemRecord::ImageView {
                    id,
                    path: strip_file_url(path).to_string(),
                })
            }
            "WebSearch" => Some(ItemRecord::Web(self.web_info(id))),
            "Extension" if opt_str(&self.kind).is_some_and(|k| k.starts_with("web.")) => {
                Some(ItemRecord::Web(self.web_info(id)))
            }
            _ => None,
        }
    }

    fn command_info(&self, id: String, span: JsonlSpan) -> CommandInfo {
        let command = self
            .command
            .as_ref()
            .map(command_from_value)
            .unwrap_or_default();
        let parsed: Vec<&str> = self
            .parsed_cmd
            .iter()
            .flatten()
            .map(|p| opt_str(&p.ty).unwrap_or("unknown"))
            .collect();
        let kind = kind_from_parsed_cmds(&parsed).unwrap_or_else(|| refine_shell_kind(&command));

        let exit_code = self.exit_code.and_then(|c| i32::try_from(c).ok());
        let mut status = opt_str(&self.status)
            .map(status_from_str)
            .unwrap_or(ToolStatus::Unknown);
        if exit_code.is_some_and(|c| c != 0) && status != ToolStatus::Interrupted {
            status = ToolStatus::Error;
        } else if status == ToolStatus::Unknown && exit_code == Some(0) {
            status = ToolStatus::Success;
        }

        let output_text = opt_str(&self.aggregated_output).unwrap_or_default();
        let output = preview(output_text);
        let output_ref = output
            .truncated
            .then(|| span.content_ref("/payload/item/aggregated_output"));
        CommandInfo {
            id,
            command,
            cwd: opt_str(&self.cwd).map(|c| strip_file_url(c).to_string()),
            kind,
            status,
            exit_code,
            duration_ms: self.duration.as_ref().map(ItemDuration::as_ms),
            output,
            output_ref,
            output_head: output_text.trim().chars().take(200).collect(),
            by_user: opt_str(&self.source).is_some_and(|s| s.contains("user")),
        }
    }

    fn file_change_info(&self, id: String, span: JsonlSpan) -> Option<FileChangeInfo> {
        let changes = self.changes.as_ref()?;
        let mut files = Vec::new();
        let mut single_diff_pointer = None;
        for (path, change) in changes {
            let op = match change.get("type").and_then(Value::as_str) {
                Some("add") => DiffOp::Add,
                Some("delete") => DiffOp::Delete,
                _ if change
                    .get("move_path")
                    .and_then(Value::as_str)
                    .is_some_and(|p| !p.is_empty()) =>
                {
                    DiffOp::Rename
                }
                _ => DiffOp::Update,
            };
            let (added, removed) =
                if let Some(diff) = change.get("unified_diff").and_then(Value::as_str) {
                    single_diff_pointer = Some(format!(
                        "/payload/item/changes/{}/unified_diff",
                        escape_pointer_token(path)
                    ));
                    count_diff_lines(diff)
                } else {
                    let lines = change
                        .get("content")
                        .and_then(Value::as_str)
                        .map(|c| saturating_u32(c.lines().count()))
                        .unwrap_or(0);
                    match op {
                        DiffOp::Delete => (0, lines),
                        _ => (lines, 0),
                    }
                };
            files.push(DiffFile {
                path: path.clone(),
                op,
                added,
                removed,
            });
        }
        if files.is_empty() {
            return None;
        }
        // 只有一个文件且是 unified diff 时才能直接指向它
        let full = (files.len() == 1)
            .then_some(single_diff_pointer)
            .flatten()
            .map(|p| span.content_ref(p));
        let mut status = opt_str(&self.status)
            .map(status_from_str)
            .unwrap_or(ToolStatus::Success);
        if status == ToolStatus::Unknown {
            status = ToolStatus::Success;
        }
        Some(FileChangeInfo {
            id,
            diff: diff_summary(files, full),
            status,
        })
    }

    fn mcp_info(&self, id: String, span: JsonlSpan) -> McpInfo {
        let server = opt_str(&self.server).unwrap_or("mcp").to_string();
        let tool = opt_str(&self.tool).unwrap_or_default().to_string();
        let detail = self
            .arguments
            .as_ref()
            .and_then(app_session_core::blocks::first_string_field);
        let input = self
            .arguments
            .as_ref()
            .map(|a| a.to_string())
            .unwrap_or_default();

        let result = self.result.as_ref();
        let mut status = opt_str(&self.status)
            .map(status_from_str)
            .unwrap_or(ToolStatus::Unknown);
        if result.and_then(|r| r.is_error) == Some(true) {
            status = ToolStatus::Error;
        } else if status == ToolStatus::Unknown && result.is_some() {
            status = ToolStatus::Success;
        }

        let contents = result.and_then(|r| r.content.as_ref());
        let text = contents.map(|c| c.joined_text("\n")).unwrap_or_default();
        let output = preview(&text);
        let output_ref = match contents {
            Some(c) if output.truncated => {
                Some(span.content_ref(c.text_pointer("/payload/item/result/content")))
            }
            _ => None,
        };
        let images = match contents {
            Some(Contents::Items(items)) => items
                .iter()
                .enumerate()
                .filter(|(_, item)| item.kind() == "image")
                .filter_map(|(i, item)| {
                    let data = opt_str(&item.data)?;
                    Some(ImageRef {
                        source: ImageSource::Inline {
                            content: span
                                .content_ref(format!("/payload/item/result/content/{i}/data")),
                        },
                        media_type: opt_str(&item.mime_type).unwrap_or("image/png").to_string(),
                        size: app_session_core::blocks::estimate_base64_size(data.len()),
                        alt: None,
                    })
                })
                .collect(),
            _ => Vec::new(),
        };

        McpInfo {
            id,
            server,
            tool,
            detail,
            input,
            status,
            duration_ms: self.duration.as_ref().map(ItemDuration::as_ms),
            output,
            output_ref,
            images,
        }
    }

    fn web_info(&self, id: String) -> WebInfo {
        let title = self
            .action
            .as_ref()
            .map(web_action_title)
            .filter(|t| !t.is_empty())
            .or_else(|| opt_str(&self.query).map(one_line_title))
            .unwrap_or_default();
        WebInfo {
            id,
            title,
            input: self
                .action
                .as_ref()
                .map(|a| a.to_string())
                .unwrap_or_default(),
        }
    }
}
