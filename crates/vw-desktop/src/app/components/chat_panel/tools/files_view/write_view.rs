//! write 工具的文件视图布局。

use std::path::Path;

use iced::widget::{column, container, text};
use iced::{Alignment, Color, Element, Length, Theme};

use crate::app::components::chat_panel::utils::{
    chat_context_target_key, chat_secondary_muted_text_color,
};
use crate::app::components::widgets::RightClickArea;
use crate::app::{Message, message};

use super::super::{
    ToolPermissionState, selected_chat_text_for_target, tool_permission_state,
    tool_permission_target_summary,
};
use super::{FileListState, FilesViewContext};

pub(super) fn write_tool_summary(
    i18n: crate::app::i18n::I18n,
    render_state: &FileListState,
) -> Option<String> {
    let (display, _) = render_state.items_for_display.first()?;
    let file_name = Path::new(display)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(display.as_str())
        .trim();

    if file_name.is_empty() {
        return Some(
            i18n.format("%u 个文件", &[render_state.total_items.max(1).into()])
                .expect("valid file count template"),
        );
    }

    if render_state.total_items > 1 {
        Some(
            i18n.format("%s 等 %u 个文件", &[file_name.into(), render_state.total_items.into()])
                .expect("valid file summary template"),
        )
    } else {
        Some(file_name.to_string())
    }
}

pub(super) fn write_tool_title(
    i18n: crate::app::i18n::I18n,
    verb: &str,
    is_running: bool,
    permission_state: Option<ToolPermissionState>,
    is_error: bool,
) -> String {
    if is_running {
        i18n.format("%s中", &[i18n.__(verb).into()]).expect("valid file activity template")
    } else if let Some(permission_state) = permission_state {
        crate::app::components::chat_panel::tools::localized_tool_permission_title(
            i18n,
            verb,
            permission_state,
        )
    } else if is_error {
        i18n.format("%s失败", &[i18n.__(verb).into()]).expect("valid file failure template")
    } else {
        i18n.__(verb).to_string()
    }
}

pub(super) fn write_tool_detail_text(
    i18n: crate::app::i18n::I18n,
    verb: &str,
    is_running: bool,
    error_text: Option<&str>,
    fallback_summary: &str,
) -> String {
    if is_running {
        i18n.format("%s中…", &[i18n.__(verb).into()]).expect("valid file progress template")
    } else if let Some(error_text) = error_text.map(str::trim).filter(|text| !text.is_empty()) {
        error_text.to_string()
    } else {
        fallback_summary.to_string()
    }
}

pub(super) fn write_tool_detail_color(theme: &Theme, is_error: bool) -> Color {
    if is_error {
        theme.extended_palette().danger.base.color.scale_alpha(0.9)
    } else {
        chat_secondary_muted_text_color(theme)
    }
}

pub(super) fn write_tool_context_menu_message(
    target: u64,
    point: iced::Point,
    text: String,
) -> Message {
    Message::Chat(message::ChatMessage::OpenMessageContextMenu {
        target,
        x: point.x,
        y: point.y,
        text,
    })
}

pub(crate) fn build_write_tool_view<'a>(
    view_ctx: &FilesViewContext<'a>,
    render_state: &FileListState,
    list_column: iced::widget::Column<'a, Message>,
) -> Element<'a, Message> {
    let context_key = chat_context_target_key(view_ctx.msg_idx, Some(view_ctx.tool_idx));
    let selected_context_text = selected_chat_text_for_target(view_ctx.app, context_key);

    let tool_value = view_ctx
        .visible
        .split_once('\n')
        .and_then(|(_, rest)| serde_json::from_str::<serde_json::Value>(rest.trim()).ok());
    let permission_state =
        tool_value.as_ref().and_then(|value| tool_permission_state(&view_ctx.tool_name, value));
    let permission_target = tool_value
        .as_ref()
        .and_then(|value| tool_permission_target_summary(&view_ctx.tool_name, value));
    let title = write_tool_title(
        crate::app::i18n::I18n::new(view_ctx.app.desktop_locale),
        view_ctx.verb,
        view_ctx.is_running,
        permission_state,
        view_ctx.is_error,
    );
    let is_error = view_ctx.is_error;

    let summary = permission_target.clone().or_else(|| {
        write_tool_summary(crate::app::i18n::I18n::new(view_ctx.app.desktop_locale), render_state)
    });
    let fallback_summary = summary.clone().unwrap_or_else(|| title.clone());
    let body: Element<'a, Message> = if render_state.total_items > 0 {
        container(list_column).width(Length::Fill).into()
    } else {
        let detail_text = write_tool_detail_text(
            crate::app::i18n::I18n::new(view_ctx.app.desktop_locale),
            view_ctx.verb,
            view_ctx.is_running,
            view_ctx.error_text.as_deref(),
            &fallback_summary,
        );

        container(
            column![text(detail_text).size(14).style(move |theme: &Theme| {
                iced::widget::text::Style { color: Some(write_tool_detail_color(theme, is_error)) }
            })]
            .align_x(Alignment::Start),
        )
        .padding([2, 6])
        .width(Length::Fill)
        .into()
    };

    let fallback_context_text = selected_context_text.clone().unwrap_or(fallback_summary);
    let content: Element<'a, Message> = RightClickArea::new(
        body,
        Box::new(move |point| {
            write_tool_context_menu_message(context_key, point, fallback_context_text.clone())
        }),
    )
    .use_global_position()
    .preserve_on_right_click()
    .into();

    content
}
