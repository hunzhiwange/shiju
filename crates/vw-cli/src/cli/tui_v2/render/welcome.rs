use super::{
    Block, BorderType, Borders, Line, Modifier, Paragraph, Rect, Span, Style, Text, TuiRenderer,
    TuiState, Wrap,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TuiWelcomeLayout {
    Compact,
    Vertical,
    Horizontal,
}

impl TuiWelcomeLayout {
    fn for_width(width: u16) -> Self {
        match width {
            0..=89 => Self::Compact,
            90..=119 => Self::Vertical,
            _ => Self::Horizontal,
        }
    }
}

pub(super) fn should_show_welcome(state: &TuiState) -> bool {
    state.messages.is_empty() && state.session.session_id.is_none()
}

impl TuiRenderer {
    pub(super) fn render_welcome(
        &self,
        frame: &mut ratatui::Frame<'_>,
        area: Rect,
        state: &TuiState,
        badge_label: &str,
        endpoint_label: &str,
    ) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let layout = TuiWelcomeLayout::for_width(area.width);
        let (target_width, target_height) = match layout {
            TuiWelcomeLayout::Compact => (area.width, 10),
            TuiWelcomeLayout::Vertical => (area.width.saturating_sub(8).min(76), 12),
            TuiWelcomeLayout::Horizontal => (area.width.saturating_sub(8).min(116), 12),
        };
        let width = target_width.max(1).min(area.width);
        let height = target_height.min(area.height).max(1);
        let card = Rect::new(
            area.x.saturating_add(area.width.saturating_sub(width) / 2),
            area.y.saturating_add(area.height.saturating_sub(height) / 3),
            width,
            height,
        );
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(self.theme.accent_style())
            .title(Span::styled(
                format!(" VibeWindow v{} ", env!("CARGO_PKG_VERSION")),
                self.theme.accent_style().add_modifier(Modifier::BOLD),
            ))
            .style(Style::default().bg(self.theme.surface));
        let inner = block.inner(card);
        frame.render_widget(block, card);

        let model = state.status.model_name.as_deref().unwrap_or("未选择模型");
        let cwd = state
            .project
            .workspace_root
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "未绑定工作目录".to_string());
        let activity = state
            .session
            .preview
            .as_ref()
            .map(|preview| preview.title.as_str())
            .filter(|title| !title.trim().is_empty())
            .unwrap_or("尚无会话活动");
        let connected = if endpoint_label.trim().is_empty() {
            "连接状态: 未连接".to_string()
        } else {
            format!("连接状态: {badge_label} · {endpoint_label}")
        };

        let lines = match layout {
            TuiWelcomeLayout::Compact => compact_lines(model, &cwd, activity, &connected, self),
            TuiWelcomeLayout::Vertical => vertical_lines(model, &cwd, activity, &connected, self),
            TuiWelcomeLayout::Horizontal => {
                horizontal_lines(model, &cwd, activity, &connected, inner.width, self)
            }
        };
        frame.render_widget(
            Paragraph::new(Text::from(lines))
                .style(Style::default().bg(self.theme.surface))
                .wrap(Wrap { trim: true }),
            inner,
        );
    }
}

fn compact_lines(
    model: &str,
    cwd: &str,
    activity: &str,
    connected: &str,
    renderer: &TuiRenderer,
) -> Vec<Line<'static>> {
    vec![
        welcome_title(renderer),
        Line::raw(""),
        Line::styled("✦  V I B E W I N D O W  ✦", renderer.theme.accent_style()),
        Line::styled(format!("模型: {model}"), renderer.theme.muted_style()),
        Line::styled(format!("目录: {cwd}"), renderer.theme.muted_style()),
        Line::styled(connected.to_string(), renderer.theme.muted_style()),
        Line::styled(format!("最近活动: {activity}"), renderer.theme.muted_style()),
        Line::styled("输入消息开始，/help 查看命令", renderer.theme.body_style()),
    ]
}

fn vertical_lines(
    model: &str,
    cwd: &str,
    activity: &str,
    connected: &str,
    renderer: &TuiRenderer,
) -> Vec<Line<'static>> {
    vec![
        welcome_title(renderer),
        Line::styled("        ✦", renderer.theme.accent_style()),
        Line::styled("  V I B E W I N D O W", renderer.theme.accent_style()),
        Line::styled("        ✦", renderer.theme.accent_style()),
        Line::raw(""),
        Line::styled(format!("模型: {model}"), renderer.theme.muted_style()),
        Line::styled(format!("目录: {cwd}"), renderer.theme.muted_style()),
        Line::styled(connected.to_string(), renderer.theme.muted_style()),
        Line::styled(format!("最近活动: {activity}"), renderer.theme.muted_style()),
        Line::styled("输入消息开始，/help 查看命令", renderer.theme.body_style()),
    ]
}

fn horizontal_lines(
    model: &str,
    cwd: &str,
    activity: &str,
    connected: &str,
    width: u16,
    renderer: &TuiRenderer,
) -> Vec<Line<'static>> {
    let left_width = usize::from(width / 2).max(24);
    let joined = |left: &str, right: &str| {
        format!("{left:<left_width$}│ {right}", left_width = left_width.saturating_sub(2))
    };
    vec![
        welcome_title(renderer),
        Line::raw(""),
        Line::styled(joined("        ✦", "最近活动"), renderer.theme.accent_style()),
        Line::styled(joined("  V I B E W I N D O W", activity), renderer.theme.body_style()),
        Line::styled(joined("        ✦", ""), renderer.theme.accent_style()),
        Line::raw(""),
        Line::styled(joined(&format!("模型: {model}"), "常用入口"), renderer.theme.muted_style()),
        Line::styled(
            joined(&format!("目录: {cwd}"), "/resume  /help"),
            renderer.theme.muted_style(),
        ),
        Line::styled(joined(connected, "输入消息开始"), renderer.theme.muted_style()),
    ]
}

fn welcome_title(renderer: &TuiRenderer) -> Line<'static> {
    Line::from(vec![Span::styled(
        "欢迎使用 VibeWindow",
        renderer.theme.body_style().add_modifier(Modifier::BOLD),
    )])
}
