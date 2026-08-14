use super::{
    Clear, Line, Paragraph, Rect, Text, TuiPanelBorder, TuiPanelSurface, TuiPill, TuiRenderer,
    TuiState, TuiTone, UiOverlay, UiOverlayKind, UiQuestionSurfaceKind, Wrap, step_state_label,
    terminal_label, truncate_label,
};

/// modal compositor 输出的聚合结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TuiModalHost {
    pub(crate) title: String,
    pub(crate) chips: Vec<TuiPill>,
    pub(crate) body_lines: Vec<String>,
}

impl TuiRenderer {
    pub(super) fn render_modal_host(
        &self,
        frame: &mut ratatui::Frame<'_>,
        area: Rect,
        modal_host: &TuiModalHost,
    ) {
        let block = self.panel_block(
            modal_host.title.clone(),
            TuiPanelSurface::Raised,
            TuiPanelBorder::Focused,
        );
        let inner = block.inner(area);

        let mut lines = vec![self.render_pills_line(&modal_host.chips, None), Line::raw("")];
        lines.extend(modal_host.body_lines.iter().cloned().map(Line::from));

        frame.render_widget(Clear, area);
        frame.render_widget(block, area);
        frame.render_widget(
            Paragraph::new(Text::from(lines))
                .style(self.theme.panel_style(TuiPanelSurface::Raised))
                .wrap(Wrap { trim: false }),
            inner,
        );
    }
}

pub(crate) fn build_modal_host(state: &TuiState) -> Option<TuiModalHost> {
    let Some(overlay) = state.overlays.active() else {
        let actions = &state.message_actions;
        if !actions.active {
            return None;
        }
        return Some(TuiModalHost {
            title: "消息操作".to_string(),
            chips: vec![TuiPill::new(
                format!(
                    "模式 {}",
                    match actions.kind {
                        crate::cli::tui_v2::model::MessageActionKind::Browse => "浏览",
                        crate::cli::tui_v2::model::MessageActionKind::Rewind => "回退",
                    }
                ),
                TuiTone::Accent,
            )],
            body_lines: vec![
                format!(
                    "选中: {}",
                    actions.selected_id.as_ref().map(|id| id.as_str()).unwrap_or("-")
                ),
                format!("展开: {}", if actions.expanded { "是" } else { "否" }),
                String::new(),
                "↑/↓ 导航 · u/U 用户消息 · Space 展开".to_string(),
                "c 复制 · e 编辑 · f fork · r restore · s summarize · w rewind".to_string(),
                "Esc 取消；破坏性动作只生成待确认 intent。".to_string(),
            ],
        });
    };
    let (title, body_lines) = overlay_body_lines(overlay);

    Some(TuiModalHost {
        title,
        chips: vec![TuiPill::new(
            format!("类型 {}", overlay_kind_label(overlay.kind())),
            TuiTone::Accent,
        )],
        body_lines,
    })
}

pub(super) fn overlay_body_lines(overlay: &UiOverlay) -> (String, Vec<String>) {
    match overlay {
        UiOverlay::Confirm(overlay) => (
            if overlay.title.trim().is_empty() {
                "确认".to_string()
            } else {
                overlay.title.clone()
            },
            vec![
                overlay.body.clone(),
                String::new(),
                format!(
                    "Enter 确认 {}  Esc {}  危险操作={}",
                    overlay.confirm_label, overlay.cancel_label, overlay.destructive
                ),
            ],
        ),
        UiOverlay::Search(overlay) => ("搜索".to_string(), search_overlay_body_lines(overlay)),
        UiOverlay::Question(overlay) => {
            (overlay.modal_title().to_string(), question_overlay_body_lines(overlay))
        }
        UiOverlay::Todo(overlay) => ("待办".to_string(), todo_overlay_body_lines(overlay)),
        UiOverlay::Task(overlay) => ("任务面板".to_string(), task_overlay_body_lines(overlay)),
        UiOverlay::CommandPalette(overlay) => {
            ("命令面板".to_string(), command_palette_body_lines(overlay))
        }
        UiOverlay::Report(overlay) => {
            let mut lines = overlay
                .lines
                .iter()
                .skip(overlay.scroll_offset)
                .take(12)
                .cloned()
                .collect::<Vec<_>>();
            if lines.is_empty() {
                lines.push("没有可展示的数据。".to_string());
            }
            if let Some(notice) = overlay.capability_notice.as_deref() {
                lines.push(String::new());
                lines.push(format!("能力说明: {notice}"));
            }
            lines.push(String::new());
            lines.push("↑/↓ 或 PgUp/PgDn 滚动 · Esc 关闭".to_string());
            (overlay.title.clone(), lines)
        }
        UiOverlay::Error(overlay) => (
            overlay.title.clone(),
            vec![
                overlay.message.clone(),
                String::new(),
                if overlay.recoverable {
                    "Esc 关闭弹层并返回输入区。".to_string()
                } else {
                    "此弹层需要显式恢复后才能继续。".to_string()
                },
            ],
        ),
        UiOverlay::Mcp(overlay) => ("MCP".to_string(), {
            let mut lines = vec![
                format!("配置来源: {}", overlay.config_source),
                format!("服务器数: {}", overlay.servers.len()),
            ];
            if overlay.servers.is_empty() {
                lines.push(String::new());
                lines.push("当前没有可展示的 MCP 服务器。".to_string());
            } else {
                lines.push(String::new());
                lines.extend(overlay.servers.iter().take(8).map(|server| {
                    format!("{} [{}] {}", server.name, server.transport.label(), server.address)
                }));
            }
            lines
        }),
        UiOverlay::Memory(overlay) => ("记忆".to_string(), {
            let mut lines = vec![format!("条目数: {}", overlay.entries.len())];
            if overlay.entries.is_empty() {
                lines.push(String::new());
                lines.push("当前没有可展示的记忆文件。".to_string());
            } else {
                lines.push(String::new());
                lines.extend(
                    overlay
                        .entries
                        .iter()
                        .take(8)
                        .map(|entry| format!("{} [{}]", entry.filename, entry.scope)),
                );
            }
            lines
        }),
        UiOverlay::Settings(panel) if panel.model_picker.is_some() => ("模型".to_string(), {
            let picker = panel.model_picker.as_ref().expect("guarded model picker");
            let mut lines = vec!["选择本次 TUI 会话使用的模型：".to_string(), String::new()];
            lines.extend(panel.model_options.iter().enumerate().map(|(index, model)| {
                let marker = if index == picker.selected { "›" } else { " " };
                format!("{marker} {model}")
            }));
            lines.push(String::new());
            lines.push("↑/↓ 选择 · Enter 应用 · Esc 取消列表编辑".to_string());
            lines
        }),
        UiOverlay::Settings(panel) if panel.delegate_agent_picker.is_some() => {
            ("委托代理".to_string(), {
                let picker =
                    panel.delegate_agent_picker.as_ref().expect("guarded delegate agent picker");
                let mut lines = vec!["选择后续提交使用的会话级代理：".to_string(), String::new()];
                lines.extend(panel.delegate_agent_options.iter().enumerate().map(
                    |(index, agent)| {
                        let marker = if index == picker.selected { "›" } else { " " };
                        format!("{marker} {}", agent.label)
                    },
                ));
                lines.push(String::new());
                lines.push("↑/↓ 选择 · Enter 确认 · Esc 取消列表编辑".to_string());
                lines
            })
        }
        UiOverlay::Settings(panel) if panel.acp_backend_picker.is_some() => {
            ("ACP 后端".to_string(), {
                let picker = panel.acp_backend_picker.as_ref().expect("guarded ACP backend picker");
                let mut lines = vec!["选择后续提交使用的 ACP 后端：".to_string(), String::new()];
                lines.extend(panel.acp_backend_options.iter().enumerate().map(
                    |(index, backend)| {
                        let marker = if index == picker.selected { "›" } else { " " };
                        format!("{marker} {}", backend.label)
                    },
                ));
                lines.push(String::new());
                lines.push("↑/↓ 选择 · Enter 应用 · Esc 取消列表编辑".to_string());
                lines
            })
        }
        UiOverlay::Settings(panel) => ("设置".to_string(), {
            let rows = panel.visible_rows();
            let mut lines = vec![
                "当前设置仅作用于本次 TUI 会话；模型同时写入项目偏好。".to_string(),
                String::new(),
            ];
            for (index, row) in rows.iter().enumerate() {
                let marker = if index == panel.selected { "›" } else { " " };
                let unavailable = row
                    .unavailable_reason
                    .map(|reason| format!(" [不可用: {reason}]"))
                    .unwrap_or_default();
                lines.push(format!(
                    "{marker} {} = {}{unavailable}",
                    row.label,
                    panel.value_label(row.key)
                ));
            }
            lines.push(String::new());
            lines.push(
                "模型/委托代理/ACP 行 Enter 打开列表 · 模式行 ←/→/Tab 切换 · Enter 保存 · Esc 取消"
                    .to_string(),
            );
            lines
        }),
        UiOverlay::Permissions(overlay) => ("待处理权限".to_string(), {
            let mut lines = vec![
                format!("待处理请求: {}", overlay.queue.len()),
                "这里只展示 Gateway 的实时待审批请求，不伪装成全局权限规则编辑器。".to_string(),
                String::new(),
            ];
            if let Some(request) = overlay.queue.current() {
                lines.push(format!("请求: {}", request.id));
                lines.push(format!("会话: {}", request.session_id));
                lines.push(format!("权限: {}", request.permission));
                lines.push(format!("预览类型: {:?}", request.preview));
                if !request.patterns.is_empty() {
                    lines.push(format!("范围: {}", request.patterns.join(", ")));
                }
                if let Some(explanation) = request.explanation.as_deref() {
                    lines.push(format!("说明: {explanation}"));
                }
                lines.push(String::new());
                lines.push(if request.always_available() {
                    "1 仅本次 · 2 按网关模式长期允许 · R 拒绝 · Esc 关闭".to_string()
                } else {
                    "1 仅本次 · 2 不可用（无持久化模式）· R 拒绝 · Esc 关闭".to_string()
                });
            } else {
                lines.push("当前没有待处理权限请求。".to_string());
                lines.push("Esc 关闭".to_string());
            }
            if let Some(notice) = overlay.notice.as_deref() {
                lines.push(String::new());
                lines.push(format!("提示: {notice}"));
            }
            lines
        }),
        UiOverlay::Integration(panel) if panel.skill_provider_picker.is_some() => {
            let selected = panel
                .skill_provider_picker
                .as_ref()
                .map(|picker| picker.selected)
                .unwrap_or_default();
            let mut lines = vw_config_types::skills::SkillsDirectoryProvider::ALL
                .iter()
                .enumerate()
                .map(|(index, provider)| {
                    format!("{} {}", if index == selected { "›" } else { " " }, provider.label())
                })
                .collect::<Vec<_>>();
            lines.push(String::new());
            lines.push("↑/↓ 选择 · Enter 确认 · Esc 取消".to_string());
            ("技能平台".to_string(), lines)
        }
        UiOverlay::Integration(panel) => ("集成".to_string(), {
            let mut lines = panel.lines();
            if panel.kind == crate::cli::tui_v2::model::integration::IntegrationKind::Skill {
                lines.push(if panel.can_toggle {
                    "输入 `$` 选择技能；`/skills` 管理平台与启停。".to_string()
                } else {
                    "输入 `$` 选择技能；`/skills` 管理平台，当前平台只读。".to_string()
                });
            }
            if !panel.skill_platform_focused
                && let Some(item) = panel.items.get(panel.selected)
            {
                lines.push(String::new());
                lines.push(format!("当前: {}", item.title));
                for action in &item.actions {
                    let reason = action
                        .disabled_reason
                        .as_deref()
                        .map(|reason| format!(" ({reason})"))
                        .unwrap_or_default();
                    lines.push(format!(
                        "  {} [{}]{reason}",
                        action.label,
                        if action.enabled { "可用" } else { "不可用" }
                    ));
                }
            }
            lines.push(String::new());
            lines.push(
                if panel.kind == crate::cli::tui_v2::model::integration::IntegrationKind::Skill
                    && !panel.can_toggle
                {
                    "↑/↓ 选择 · Enter/p 平台 · r 刷新 · Esc 关闭".to_string()
                } else if panel.kind
                    == crate::cli::tui_v2::model::integration::IntegrationKind::Skill
                {
                    "↑/↓ 选择 · Enter 平台/启停 · p 平台 · Space/t 启停 · r 刷新 · Esc 关闭"
                        .to_string()
                } else {
                    "↑/↓ 选择 · Enter/Space/t 启停 · d 删除（需确认）· r 刷新 · Esc 关闭"
                        .to_string()
                },
            );
            lines
        }),
        UiOverlay::BackgroundTasks(panel) => ("后台任务".to_string(), {
            let mut lines = vec![
                format!("任务数: {}", panel.tasks.len()),
                "Scheduler 任务没有可靠的 session 归属时保持只读。".to_string(),
                String::new(),
            ];
            for (index, task) in panel.tasks.iter().take(10).enumerate() {
                let marker = if index == panel.selected { "›" } else { " " };
                lines.push(format!("{marker} {} [{:?}] {:?}", task.title, task.kind, task.status));
            }
            if let Some(task) = panel.tasks.get(panel.selected) {
                lines.push(String::new());
                lines.push(format!("ID: {}", task.id));
                lines.push(format!("详情: {}", task.detail));
                if !task.log.is_empty() {
                    lines.push(format!("日志: {}", task.log.join(" | ")));
                }
            }
            if let Some(error) = panel.error.as_deref() {
                lines.push(format!("加载错误: {error}"));
            }
            lines.push(String::new());
            lines.push("↑/↓ 选择 · Esc 关闭".to_string());
            lines
        }),
        UiOverlay::Agents(view) => ("Agents".to_string(), {
            let mut lines = vec![
                format!("Workers: {}", view.members.len()),
                "当前 Gateway 只提供 worker/team 读取；没有可证明作用域的消息或停止命令。"
                    .to_string(),
                String::new(),
            ];
            for member in view.members.iter().take(10) {
                let marker = if view.viewed_agent_id.as_deref() == Some(member.id.as_str()) {
                    "›"
                } else {
                    " "
                };
                lines.push(format!(
                    "{marker} {} ({}) [{:?}] team={}",
                    member.name,
                    member.id,
                    member.status,
                    member.team_id.as_deref().unwrap_or("-")
                ));
            }
            if let Some(error) = view.error.as_deref() {
                lines.push(format!("加载错误: {error}"));
            }
            lines.push(String::new());
            lines.push("↑/↓ 选择 · Esc 关闭".to_string());
            lines
        }),
        UiOverlay::Workflows(panel) => ("Workflows".to_string(), {
            let mut lines = vec![format!("Workflow 数: {}", panel.items.len()), String::new()];
            for (index, item) in panel.items.iter().take(10).enumerate() {
                let marker = if index == panel.selected { "›" } else { " " };
                lines.push(format!("{marker} {} ({})", item.name, item.uuid));
                if !item.description.trim().is_empty() {
                    lines.push(format!("    {}", item.description));
                }
            }
            if let Some(error) = panel.error.as_deref() {
                lines.push(format!("加载错误: {error}"));
            }
            lines.push(String::new());
            lines.push("↑/↓ 选择 · Enter 填入动态命令 · Esc 关闭".to_string());
            lines
        }),
        UiOverlay::Schedules(panel) => ("VibeWindow Cron".to_string(), {
            let mut lines = vec![format!("任务数: {}", panel.items.len()), String::new()];
            for (index, item) in panel.items.iter().take(10).enumerate() {
                let marker = if index == panel.selected { "›" } else { " " };
                lines.push(format!(
                    "{marker} {} [{}] {} next={}",
                    item.name,
                    if item.enabled { "enabled" } else { "disabled" },
                    item.schedule,
                    item.next_run
                ));
            }
            if let Some(item) = panel.items.get(panel.selected) {
                lines.push(String::new());
                lines.push(format!("ID: {}", item.id));
                lines.push(format!("最近状态: {}", item.last_status.as_deref().unwrap_or("-")));
                lines.extend(item.history.iter().map(|run| format!("  {run}")));
                if let Some(error) = item.history_error.as_deref() {
                    lines.push(format!("历史不可用: {error}"));
                }
            }
            if let Some(error) = panel.error.as_deref() {
                lines.push(format!("加载错误: {error}"));
            }
            lines.push(String::new());
            lines.push("使用 /cron enable|disable|delete <id> 变更 · Esc 关闭".to_string());
            lines
        }),
    }
}

fn command_palette_body_lines(
    overlay: &crate::cli::tui_v2::model::overlay::UiCommandPaletteOverlay,
) -> Vec<String> {
    let visible_rows = overlay.visible_rows.max(1);
    let first = overlay.first_visible_index.min(overlay.items.len());
    let end = first.saturating_add(visible_rows).min(overlay.items.len());
    let mut lines = vec![
        format!("查询: /{}", overlay.query),
        format!("命令: {} · 显示 {}-{}", overlay.items.len(), first, end),
        String::new(),
    ];

    if overlay.items.is_empty() {
        lines.push("没有匹配的命令。".to_string());
    } else {
        for (index, item) in overlay.items.iter().enumerate().take(end).skip(first) {
            let marker = if overlay.selected_index == Some(index) { "›" } else { " " };
            let argument =
                item.argument_hint.as_deref().map(|hint| format!(" {hint}")).unwrap_or_default();
            let disabled = if item.available { "" } else { " [不可用]" };
            lines.push(truncate_label(
                format!("{marker} {}{argument} [{}]{disabled}", item.label, item.source).as_str(),
                88,
            ));
            if let Some(detail) = item.detail.as_deref() {
                lines.push(format!("    {}", truncate_label(detail, 84)));
            }
            if let Some(reason) = item.unavailable_reason.as_deref() {
                lines.push(format!("    原因: {}", truncate_label(reason, 80)));
            }
        }
    }

    if let Some(notice) = overlay.notice.as_deref() {
        lines.push(String::new());
        lines.push(format!("不可用: {}", truncate_label(notice, 80)));
    }
    lines.push(String::new());
    lines.push("输入筛选 · ↑/↓/Tab 导航 · PgUp/PgDn 翻页 · Enter 选择 · Esc 关闭".to_string());
    lines
}

pub(super) fn search_overlay_body_lines(
    overlay: &super::super::model::UiSearchOverlay,
) -> Vec<String> {
    let mut lines = vec![
        format!(
            "查询: {}",
            if overlay.query.trim().is_empty() {
                "<空>".to_string()
            } else {
                overlay.query.clone()
            }
        ),
        format!("大小写: {}", if overlay.case_sensitive { "区分" } else { "不区分" }),
        format!("匹配数: {}", overlay.matches.len()),
        String::new(),
    ];

    if overlay.query.trim().is_empty() {
        lines.push("开始输入后会在当前会话中搜索。".to_string());
        lines.push("Enter 跳到当前结果，End 跳到末尾，u 跳到第一条未读。".to_string());
        lines.push("Tab / Shift+Tab 切换结果，Ctrl+S 切换大小写。".to_string());
        return lines;
    }

    let Some(selected_index) = overlay.selected_index else {
        lines.push("当前查询没有匹配结果。".to_string());
        lines.push("继续输入以缩小范围，或用 End / u 快速跳转。".to_string());
        return lines;
    };

    if let Some(selected) = overlay.matches.get(selected_index) {
        lines.push(format!("选中: {}/{}", selected_index.saturating_add(1), overlay.matches.len()));
        lines.push(selected.preview.clone());
    }
    lines.push(String::new());
    lines.push("Enter 跳到当前结果并关闭搜索弹层。".to_string());
    lines.push("End 跳到会话末尾，u 跳到第一条未读锚点。".to_string());
    lines
}

pub(super) fn question_overlay_body_lines(
    overlay: &super::super::model::UiQuestionOverlay,
) -> Vec<String> {
    let mut lines = vec![
        format!("请求: {}", overlay.request_id),
        format!("会话: {}", overlay.session_id),
        format!(
            "题目: {}/{}",
            overlay.selected_index.saturating_add(1),
            overlay.prompts.len().max(1)
        ),
    ];

    match overlay.surface_kind() {
        UiQuestionSurfaceKind::Question => {}
        UiQuestionSurfaceKind::ToolFallback => {
            if let Some(tool) = overlay.tool.as_ref() {
                lines.push(
                    "该提问来自工具回退通道，你可以直接在这里回答，无需离开 TUI。".to_string(),
                );
                lines.push(format!("工具调用: {}", tool.call_id));
                lines.push(format!("来源消息: {}", tool.message_id));
                lines.push(String::new());
            }
        }
        UiQuestionSurfaceKind::PermissionRequest => {
            if let Some(tool) = overlay.tool.as_ref() {
                lines.push(
                    "该工具调用正在等待你的授权。选择允许项后可继续，或按 Ctrl+R 明确拒绝。"
                        .to_string(),
                );
                lines.push(format!("工具调用: {}", tool.call_id));
                lines.push(format!("来源消息: {}", tool.message_id));
                lines.push(String::new());
            }
        }
    }

    let Some(prompt) = overlay.prompts.get(overlay.selected_index) else {
        lines.push("当前问题内容为空。".to_string());
        return lines;
    };

    if !prompt.header.trim().is_empty() {
        lines.push(format!("标题: {}", prompt.header));
    }
    lines.push(prompt.question.clone());

    let answers = overlay.answers.get(overlay.selected_index).cloned().unwrap_or_default();
    let selected_answers = answers
        .iter()
        .map(|answer| answer.strip_prefix("__custom__:").unwrap_or(answer.as_str()).to_string())
        .collect::<Vec<_>>();

    if !prompt.options.is_empty() {
        lines.push(String::new());
        lines.push("选项:".to_string());
        for (index, option) in prompt.options.iter().take(9).enumerate() {
            let selected = selected_answers.iter().any(|answer| answer == &option.label);
            let description = if option.description.trim().is_empty() {
                String::new()
            } else {
                format!(" - {}", option.description)
            };
            lines.push(format!(
                "  {}. [{}] {}{}",
                index + 1,
                if selected { "x" } else { " " },
                option.label,
                description
            ));
        }
    }

    lines.push(String::new());
    lines.push(format!(
        "当前回答: {}",
        if selected_answers.is_empty() { "-".to_string() } else { selected_answers.join(", ") }
    ));
    if !prompt.multiple
        && selected_answers.len() == 1
        && let Some(preview) = prompt
            .options
            .iter()
            .find(|option| option.label == selected_answers[0])
            .and_then(|option| option.preview.as_deref())
    {
        lines.push(String::new());
        lines.push("预览:".to_string());
        for line in preview.lines() {
            lines.push(format!("  {}", line));
        }
    }
    lines.push(match overlay.surface_kind() {
        UiQuestionSurfaceKind::PermissionRequest => {
            "按键: 1-9 选择授权  Tab/Shift+Tab 切题  Enter 提交  Ctrl+R 拒绝  Esc 关闭".to_string()
        }
        _ => "按键: 1-9 选择  Tab/Shift+Tab 切题  Enter 提交  Ctrl+R 拒绝  Esc 关闭".to_string(),
    });
    lines
}

pub(super) fn todo_overlay_body_lines(overlay: &super::super::model::UiTodoOverlay) -> Vec<String> {
    let completed_count =
        overlay.items.iter().filter(|item| item.status.eq_ignore_ascii_case("completed")).count();
    let pending_count = overlay.items.len().saturating_sub(completed_count);
    let mut lines = vec![
        format!("会话: {}", overlay.session_id.as_deref().unwrap_or("-")),
        format!("条目: {}", overlay.items.len()),
        format!("脏标记: {}", overlay.dirty),
        format!("状态汇总: 待处理={} 已完成={}", pending_count, completed_count),
    ];

    if overlay.items.is_empty() {
        lines.push(String::new());
        lines.push("当前会话还没有待办项。".to_string());
        lines.push("按 r 刷新，或按 Esc 关闭面板。".to_string());
        return lines;
    }

    for (index, item) in overlay.items.iter().take(8).enumerate() {
        let marker = if index == overlay.selected_index { ">" } else { " " };
        lines.push(format!(
            "{} [{}] {} ({})",
            marker,
            if item.status.eq_ignore_ascii_case("completed") { "x" } else { " " },
            truncate_label(item.content.as_str(), 44),
            item.priority
        ));
    }

    if let Some(item) = overlay.items.get(overlay.selected_index) {
        lines.push(String::new());
        lines.push(format!(
            "当前待办: {}/{}",
            overlay.selected_index.saturating_add(1),
            overlay.items.len()
        ));
        lines.push(format!("ID: {}", item.id));
        lines.push(format!("状态: {}", item.status));
        lines.push(format!("优先级: {}", item.priority));
        lines.push("内容:".to_string());
        for line in item.content.lines() {
            lines.push(format!("  {}", line));
        }
        if item.content.trim().is_empty() {
            lines.push("  -".to_string());
        }
    }

    lines.push(String::new());
    lines.push("按键: Up/Down 移动  Space 切换完成  s 保存  r 刷新  Esc 关闭".to_string());
    lines
}

pub(super) fn task_overlay_body_lines(overlay: &super::super::model::UiTaskOverlay) -> Vec<String> {
    let mut lines = vec![
        format!("会话: {}", overlay.session_id.as_deref().unwrap_or("-")),
        format!("终端: {}", terminal_label(&overlay.turn_terminal)),
        format!("待处理问题: {}", overlay.pending_questions),
        format!("待办: {}", overlay.todo_count),
    ];

    if let Some(sync_error) = overlay.sync_error.as_ref() {
        lines.push(format!("任务同步错误: {sync_error}"));
    }

    lines.push(String::new());

    if overlay.steps.is_empty() {
        lines.push("当前会话还没有步骤活动。".to_string());
        lines.push("该面板仍会持续显示问题、待办和任务同步状态。".to_string());
        lines.push("Esc 关闭面板。".to_string());
        return lines;
    }

    lines.push(format!("步骤数: {}", overlay.steps.len()));
    for (index, step) in overlay.steps.iter().enumerate() {
        let marker = if index == overlay.selected_index { ">" } else { " " };
        let finish_reason = step.finish_reason.as_deref().unwrap_or("-");
        lines.push(format!(
            "{marker} 步骤 {}  {}  结束={}",
            step.step_index,
            step_state_label(&step.state),
            finish_reason
        ));
    }

    if let Some(step) = overlay.steps.get(overlay.selected_index) {
        lines.push(String::new());
        lines.push(format!("当前步骤: {}", step.step_index));
        lines.push(format!("状态: {}", step_state_label(&step.state)));
        lines.push(step_timing_line(step));
        lines.push(format!(
            "令牌: 输入={} 输出={} 缓存={} 推理={}",
            step.usage.input_tokens,
            step.usage.output_tokens,
            step.usage.cached_tokens,
            step.usage.reasoning_tokens
        ));
        lines.push(format!("结束原因: {}", step.finish_reason.as_deref().unwrap_or("-")));
    }

    lines.push(String::new());
    lines.push("Up/Down 切换步骤，Enter 跳到选中步骤。".to_string());
    lines.push("Esc 关闭面板并返回输入区。".to_string());
    lines
}

pub(super) fn step_timing_line(step: &super::super::model::UiTaskStepItem) -> String {
    match step.finished_ms {
        Some(finished_ms) => format!(
            "耗时: 开始={} 结束={} 总计={}ms",
            step.started_ms,
            finished_ms,
            finished_ms.saturating_sub(step.started_ms)
        ),
        None => format!("耗时: 开始={} 结束=进行中", step.started_ms),
    }
}

pub(super) fn overlay_kind_label(kind: UiOverlayKind) -> &'static str {
    match kind {
        UiOverlayKind::Confirm => "确认",
        UiOverlayKind::Search => "搜索",
        UiOverlayKind::Question => "提问",
        UiOverlayKind::Todo => "待办",
        UiOverlayKind::Task => "任务",
        UiOverlayKind::CommandPalette => "命令面板",
        UiOverlayKind::Report => "报告",
        UiOverlayKind::Error => "错误",
        UiOverlayKind::Mcp => "MCP",
        UiOverlayKind::Memory => "记忆",
        UiOverlayKind::Settings => "设置",
        UiOverlayKind::Permissions => "权限",
        UiOverlayKind::Integration => "集成",
        UiOverlayKind::BackgroundTasks => "后台任务",
        UiOverlayKind::Agents => "Agents",
        UiOverlayKind::Workflows => "Workflows",
        UiOverlayKind::Schedules => "Cron",
    }
}
