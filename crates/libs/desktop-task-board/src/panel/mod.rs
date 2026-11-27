//! 任务看板的新建弹窗、详情面板和日志视图。
//!
//! 本模块只负责视图组合与样式适配，不持有业务状态，也不扩大外部能力边界。

use std::fmt;

use iced::widget::{
    Space, button, checkbox, column, container, row, scrollable, text, text_editor, text_input,
};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme};

use desktop_ui_controls::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, primary_action_btn_style,
    rounded_action_btn_style, settings_checkbox_style, settings_close_button, settings_divider,
    settings_modal_card, settings_modal_header, settings_modal_layout, settings_panel,
    settings_segment_button_style, settings_select,
};
use desktop_i18n::I18n;
use crate::message::TaskBoardMessage;
use desktop_settings_agents::state::MAIN_AGENT_KEY;
use crate::message::TaskBoardDetailTab;
use desktop_task_model::Task;
use self::PanelMessage as Message;
use config_agent::is_retired_builtin_agent_key;

use super::common::button_style_success;

pub mod executor_selector;
pub mod logs_modal;
pub mod reasoning_selector;
pub mod styles;
mod task_logs;
mod forms;
use forms::*;
pub use forms::build_bulk_agent_selector;
use styles::{editor_style, input_label, input_style, panel_container_style};
use task_logs::build_task_logs;

const TASK_PANEL_SCROLLBAR_WIDTH: f32 = 4.0;
const TASK_PANEL_SCROLLBAR_GUTTER: f32 = 12.0;
pub const CREATE_TITLE_INPUT_ID: &str = "task-board-create-title";

#[derive(Debug, Clone, PartialEq, Eq)]
struct TaskBoardAgentOption {
    key: String,
    label: String,
}

impl fmt::Display for TaskBoardAgentOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.label)
    }
}

fn task_status_tag_colors(status: desktop_task_model::TaskStatus) -> (Color, Color) {
    match status {
        desktop_task_model::TaskStatus::Pool => {
            (Color::from_rgb8(107, 114, 128), Color::from_rgb8(243, 244, 246))
        }
        desktop_task_model::TaskStatus::Pending => {
            (Color::from_rgb8(37, 99, 235), Color::from_rgb8(219, 234, 254))
        }
        desktop_task_model::TaskStatus::Running => {
            (Color::from_rgb8(147, 51, 234), Color::from_rgb8(243, 232, 255))
        }
        desktop_task_model::TaskStatus::Failed => {
            (Color::from_rgb8(220, 38, 38), Color::from_rgb8(254, 226, 226))
        }
        desktop_task_model::TaskStatus::Paused => {
            (Color::from_rgb8(202, 138, 4), Color::from_rgb8(254, 249, 195))
        }
        desktop_task_model::TaskStatus::CodeComplete => {
            (Color::from_rgb8(5, 150, 105), Color::from_rgb8(209, 250, 229))
        }
        desktop_task_model::TaskStatus::PrSubmitted => {
            (Color::from_rgb8(8, 145, 178), Color::from_rgb8(207, 250, 254))
        }
        desktop_task_model::TaskStatus::Completed => {
            (Color::from_rgb8(22, 163, 74), Color::from_rgb8(220, 252, 231))
        }
        desktop_task_model::TaskStatus::Archived => {
            (Color::from_rgb8(100, 116, 139), Color::from_rgb8(241, 245, 249))
        }
        // 自定义状态暂用中性灰，待工作流迁移任务接管自定义列样式。
        desktop_task_model::TaskStatus::Custom(_) => {
            (Color::from_rgb8(107, 114, 128), Color::from_rgb8(243, 244, 246))
        }
    }
}

fn now_ms() -> u64 {
    runtime_time::now_ms()
}

fn format_duration_ms(duration_ms: u64) -> String {
    let secs = (duration_ms / 1000) as i64;
    let value = display_format::format_duration(secs);
    if value.is_empty() { "0s".to_string() } else { value }
}

fn running_dots(now_ms: u64) -> &'static str {
    match ((now_ms / 1000) % 3) as u8 {
        0 => "·",
        1 => "··",
        _ => "···",
    }
}

fn segmented_button_style(
    theme: &Theme,
    active: bool,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let mut style = settings_segment_button_style(theme, status, active);
    style.text_color = theme.palette().text;
    style
}

fn section_block<'a>(
    app: PanelView<'a>,
    title: &'a str,
    description: &'a str,
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let title = i18n.__(title);
    let label = text(title)
        .size(13)
        .font(iced::Font { weight: iced::font::Weight::Bold, ..Default::default() });
    let heading =
        settings_hover_help_label(i18n, app.app_theme, title, label, i18n.__(description));
    column![heading, settings_panel(content.into())].spacing(8).into()
}

/// 构建对应界面片段。
///
/// # 参数
/// - `app`: 当前视图构建所需的状态、配置或消息。
/// - `is_edit_mode`: 当前视图构建所需的状态、配置或消息。
///
/// # 返回
/// 返回可直接嵌入父级视图的 Iced 控件或样式闭包。
///
/// # 错误
/// 此函数不返回 `Result`；不可用状态会通过空视图、禁用控件或回退文案表达。
pub fn build_task_panel<'a>(app: PanelView<'a>, is_edit_mode: bool, slots: PanelSlots<'a>) -> Element<'a, Message> {
    build_task_panel_with_content(app, is_edit_mode, slots, |message| message, None)
}

fn build_task_panel_with_content<'a, M: Clone + 'a>(
    app: PanelView<'a>,
    is_edit_mode: bool,
    slots: PanelSlots<'a>,
    map: impl Fn(Message) -> M + Copy + 'a,
    detail: Option<Element<'a, M>>,
) -> Element<'a, M> {
    if !is_edit_mode {
        return build_create_modal(app, slots).map(map);
    }

    let current_task = app.task_board_viewing_logs.as_ref();
    let task_id_text = Some(current_task.map(|task| task.id.as_str()).unwrap_or(""));
    let i18n = I18n::new(app.desktop_locale);
    let title = i18n.__("任务详情");
    let title_label = text(title)
        .size(16)
        .font(iced::Font { weight: iced::font::Weight::Bold, ..Default::default() });
    let title = settings_hover_help_label(
        i18n,
        app.app_theme,
        title,
        title_label,
        i18n.__("查看运行状态、日志和执行参数。"),
    );
    let header_row = build_header_row(
        i18n,
        title,
        task_id_text,
        current_task,
        build_close_button(TaskBoardMessage::CloseTaskLogs),
    );
    let header = column![header_row, Space::new().height(18.0), build_detail_tabs(app)]
        .width(Length::Fill);
    let mut main_col = column![].width(Length::Fill);

    let showing_logs = app.task_board_detail_tab == TaskBoardDetailTab::Logs;
    if showing_logs {
        main_col = main_col.push(build_task_logs(app));
    } else {
        if let Some(reason_row) = current_task.and_then(|task| {
            build_failed_reason_row(desktop_i18n::I18n::new(app.desktop_locale), task)
        }) {
            main_col = main_col.push(reason_row).push(Space::new().height(12.0));
        }
        if let Some(task) = current_task
            && let Some(worktree_row) = build_selected_worktree_row(
                desktop_i18n::I18n::new(app.desktop_locale),
                task,
            )
        {
            main_col = main_col.push(worktree_row).push(Space::new().height(12.0));
        }
        if let Some(task) = current_task
            && let Some(merge_lock_row) = build_merge_lock_row(app, task)
        {
            main_col = main_col.push(merge_lock_row).push(Space::new().height(12.0));
        }

        let control = |label, field| {
            column![
                input_label(i18n.__(label)),
                container(field).center_y(42).width(Length::Fill),
            ]
            .spacing(6)
            .width(Length::FillPortion(1))
        };
        let priority_field = control("优先级", build_priority_field(app, true));
        let model_field = control("模型", slots.model.expect("visible model selector"));
        let agent_field = control("代理", build_agent_field(app));
        let executor_field = control("执行器", slots.executor.expect("visible executor selector"));
        let reasoning_effort_field = slots.reasoning.map(|field| control("推理强度", field));
        let prompt_field = build_prompt_field(app);

        main_col = main_col.push(section_block(
            app,
            "任务配置",
            "编辑提示词、模型、代理、执行器和优先级。",
            column![
                prompt_field,
                settings_divider(),
                row![agent_field, executor_field]
                    .extend(reasoning_effort_field.map(Into::into))
                    .spacing(12)
                    .align_y(Alignment::End),
                row![model_field, priority_field]
                    .spacing(12)
                    .align_y(Alignment::End),
            ]
            .spacing(14),
        ));

        if current_task.is_some() {
            main_col = main_col.push(Space::new().height(16.0)).push(build_edit_options_row(app));
        }
    }

    let panel_body: Element<'_, Message> = main_col.into();
    let panel_header: Element<'_, Message> = header.into();
    let main_col = column![].extend(detail).push(panel_body.map(map)).spacing(12).width(Length::Fill);
    let content: Element<'_, M> = if showing_logs {
        main_col.height(Length::Fill).into()
    } else {
        scrollable(container(main_col).width(Length::Fill).padding(iced::Padding {
            left: 7.0,
            right: TASK_PANEL_SCROLLBAR_GUTTER,
            ..iced::Padding::ZERO
        }))
        .direction(iced::widget::scrollable::Direction::Vertical(
            iced::widget::scrollable::Scrollbar::new()
                .width(TASK_PANEL_SCROLLBAR_WIDTH)
                .scroller_width(TASK_PANEL_SCROLLBAR_WIDTH),
        ))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    };
    let mut content = column![panel_header.map(map), content].spacing(12).height(Length::Fill);
    if !showing_logs && current_task.is_some() {
        content = content.push(build_action_buttons(app, true).map(map));
    }

    let panel = container(content)
        .padding(20)
        .style(panel_container_style)
        .width(560)
        .height(Length::Fill);
    // Mark the whole sidebar surface as interactive so the stack shields the board.
    iced::widget::mouse_area(panel).interaction(iced::mouse::Interaction::Idle).into()
}

fn build_create_modal<'a>(app: PanelView<'a>, slots: PanelSlots<'a>) -> Element<'a, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let header = settings_modal_header(
        text(i18n.__("新建任务"))
            .size(18)
            .font(iced::Font { weight: iced::font::Weight::Bold, ..Default::default() }),
        row![
            build_mode_toggle(app),
            build_close_button(TaskBoardMessage::CreateTaskCancelled),
        ]
        .spacing(12)
        .align_y(Alignment::Center),
    );
    if app.task_board_is_import_mode {
        settings_modal_layout(
            header,
            build_import_field(app),
            Some(build_action_buttons(app, false)),
        )
        .width(760)
        .height(660)
        .into()
    } else {
        settings_modal_card(
            column![
                header,
                settings_divider(),
                build_create_form(app, slots),
                settings_divider(),
                build_create_footer(app),
            ]
                .spacing(16)
                .height(Length::Fill),
        )
        .width(700)
        .height(510)
        .into()
    }
}

fn build_close_button(close_msg: TaskBoardMessage) -> iced::Element<'static, Message> {
    settings_close_button(Message::TaskBoard(close_msg))
}

fn build_detail_tabs(app: PanelView<'_>) -> Element<'_, Message> {
    row![
        build_detail_tab_button(
            desktop_i18n::I18n::new(app.desktop_locale).__("基本"),
            TaskBoardDetailTab::Basic,
            app.task_board_detail_tab
        ),
        build_detail_tab_button(
            desktop_i18n::I18n::new(app.desktop_locale).__("日志"),
            TaskBoardDetailTab::Logs,
            app.task_board_detail_tab
        ),
        Space::new().width(Length::Fill),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

fn build_detail_tab_button(
    label: &str,
    tab: TaskBoardDetailTab,
    active_tab: TaskBoardDetailTab,
) -> iced::widget::Button<'_, Message> {
    button(text(label).size(13))
        .on_press(Message::TaskBoard(TaskBoardMessage::SelectDetailTab(tab)))
        .padding([6, 14])
        .style(move |theme: &Theme, status| {
            segmented_button_style(theme, active_tab == tab, status)
        })
}

fn build_header_row<'a>(
    i18n: desktop_i18n::I18n,
    title: impl Into<Element<'a, Message>>,
    task_id_text: Option<&'a str>,
    task: Option<&'a Task>,
    close_btn: iced::Element<'a, Message>,
) -> iced::widget::Row<'a, Message> {
    let mut title_row = row![].spacing(8).align_y(Alignment::Center).width(Length::Shrink);
    let now = now_ms();

    if let Some(id) = task_id_text {
        title_row = title_row.push(text(id).size(11).style(|theme: &Theme| {
            iced::widget::text::Style { color: Some(theme.extended_palette().background.base.text) }
        }));
    }

    title_row = title_row.push(title.into());

    if let Some(status) = task.map(|value| value.status) {
        let (status_color, status_bg_color) = task_status_tag_colors(status);
        let status_text = if status == desktop_task_model::TaskStatus::Running {
            format!("{} {}", desktop_task_model::i18n::status_label(i18n, status), running_dots(now))
        } else {
            desktop_task_model::i18n::status_label(i18n, status)
        };
        let status_tag = container(text(status_text).size(10))
            .height(Length::Fixed(20.0))
            .align_y(iced::alignment::Vertical::Center)
            .padding([0, 8])
            .style(move |_theme: &Theme| iced::widget::container::Style {
                background: Some(Background::Color(status_bg_color)),
                border: Border { radius: 999.0.into(), ..Default::default() },
                text_color: Some(status_color),
                ..Default::default()
            });
        title_row = title_row.push(status_tag);
    }

    if let Some(duration_ms) = task.and_then(|value| value.display_execution_duration_ms(now)) {
        let label =
            if task.is_some_and(|value| value.status == desktop_task_model::TaskStatus::Running) {
                i18n.format("执行中 %s", &[(format_duration_ms(duration_ms)).into()])
                    .expect("valid task detail template")
            } else {
                i18n.format("已执行 %s", &[(format_duration_ms(duration_ms)).into()])
                    .expect("valid task detail template")
            };
        title_row =
            title_row.push(text(label).size(11).style(|theme: &Theme| iced::widget::text::Style {
                color: Some(theme.extended_palette().background.base.text.scale_alpha(0.85)),
            }));
    }

    row![title_row, Space::new().width(Length::Fill), close_btn,]
        .spacing(8)
        .align_y(Alignment::Center)
        .width(Length::Fill)
}

fn build_failed_reason_row<'a>(
    i18n: desktop_i18n::I18n,
    task: &'a desktop_task_model::Task,
) -> Option<Element<'a, Message>> {
    if task.status == desktop_task_model::TaskStatus::Failed {
        let reason = task.last_error.as_deref().unwrap_or(i18n.__("未知错误")).trim();
        if reason.is_empty() {
            return None;
        }
        let reason_text = text(
            i18n.format("执行失败: %s", &[(reason).into()]).expect("valid task detail template"),
        )
        .size(12)
        .wrapping(iced::widget::text::Wrapping::WordOrGlyph);
        let retry_button = button(text(i18n.__("手工重试")).size(12))
            .padding([6, 10])
            .style(rounded_action_btn_style)
            .on_press(Message::TaskBoard(TaskBoardMessage::ExecuteTask {
                task_id: task.id.clone(),
            }));
        return Some(
            container(column![reason_text, retry_button].spacing(10).align_x(Alignment::Start))
                .padding([8, 10])
                .width(Length::Fill)
                .style(|theme: &Theme| {
                    let p = theme.extended_palette();
                    iced::widget::container::Style {
                        background: Some(Background::Color(p.danger.weak.color.scale_alpha(0.45))),
                        border: Border {
                            color: p.danger.base.color.scale_alpha(0.6),
                            width: 1.0,
                            radius: 6.0.into(),
                        },
                        ..Default::default()
                    }
                })
                .into(),
        );
    }
    if task.status == desktop_task_model::TaskStatus::Paused {
        let reason = task.pause_reason.as_deref().unwrap_or(i18n.__("人工暂停")).trim();
        if reason.is_empty() {
            return None;
        }
        let reason_text = text(
            i18n.format("暂停原因: %s", &[(reason).into()]).expect("valid task detail template"),
        )
        .size(12)
        .wrapping(iced::widget::text::Wrapping::WordOrGlyph);
        return Some(
            container(reason_text)
                .padding([8, 10])
                .width(Length::Fill)
                .style(|theme: &Theme| {
                    let p = theme.extended_palette();
                    iced::widget::container::Style {
                        background: Some(Background::Color(p.warning.weak.color.scale_alpha(0.45))),
                        border: Border {
                            color: p.warning.base.color.scale_alpha(0.6),
                            width: 1.0,
                            radius: 6.0.into(),
                        },
                        ..Default::default()
                    }
                })
                .into(),
        );
    }
    None
}

fn build_selected_worktree_row<'a>(
    i18n: desktop_i18n::I18n,
    task: &'a desktop_task_model::Task,
) -> Option<Element<'a, Message>> {
    let path = task.selected_worktree_path.as_deref()?.trim();
    if path.is_empty() {
        return None;
    }
    Some(
        container(
            column![
                text(i18n.__("当前工作区")).size(12).style(|theme: &Theme| {
                    iced::widget::text::Style {
                        color: Some(
                            theme.extended_palette().background.base.text.scale_alpha(0.78),
                        ),
                    }
                }),
                text(path).size(12).wrapping(iced::widget::text::Wrapping::WordOrGlyph),
            ]
            .spacing(4),
        )
        .padding([8, 10])
        .width(Length::Fill)
        .style(|theme: &Theme| {
            let p = theme.extended_palette();
            iced::widget::container::Style {
                background: Some(Background::Color(p.primary.weak.color.scale_alpha(0.22))),
                border: Border {
                    color: p.primary.base.color.scale_alpha(0.32),
                    width: 1.0,
                    radius: 6.0.into(),
                },
                ..Default::default()
            }
        })
        .into(),
    )
}

fn build_merge_lock_row<'a>(
    app: PanelView<'a>,
    task: &'a desktop_task_model::Task,
) -> Option<Element<'a, Message>> {
    let _project_path = app.project_path.as_deref()?;
    let target_branch = task.merge_target_branch.as_deref()?.trim();
    if target_branch.is_empty() {
        return None;
    }
    let lock_holder = app.task_board_worktree_snapshot.as_ref().and_then(|snapshot| {
        snapshot
            .merge_target_locks
            .iter()
            .find(|(target, _)| target == target_branch)
            .map(|(_, task_id)| task_id.clone())
    });
    let holder_text = match lock_holder.as_deref() {
        Some(task_id) if task_id == task.id => {
            desktop_i18n::I18n::new(app.desktop_locale).__("当前任务持有").to_string()
        }
        Some(task_id) => desktop_i18n::I18n::new(app.desktop_locale)
            .format("占用者: %s", &[(task_id).into()])
            .expect("valid merge lock template"),
        None => desktop_i18n::I18n::new(app.desktop_locale).__("当前无占用者").to_string(),
    };
    Some(
        container(
            column![
                text(desktop_i18n::I18n::new(app.desktop_locale).__("Merge 锁状态"))
                    .size(12)
                    .style(|theme: &Theme| iced::widget::text::Style {
                        color: Some(
                            theme.extended_palette().background.base.text.scale_alpha(0.78)
                        ),
                    }),
                text(
                    desktop_i18n::I18n::new(app.desktop_locale)
                        .format("目标分支: %s", &[(target_branch).into()])
                        .expect("valid merge lock template")
                )
                .size(12),
                text(holder_text).size(12),
            ]
            .spacing(4),
        )
        .padding([8, 10])
        .width(Length::Fill)
        .style(|theme: &Theme| {
            let p = theme.extended_palette();
            iced::widget::container::Style {
                background: Some(Background::Color(p.secondary.weak.color.scale_alpha(0.26))),
                border: Border {
                    color: p.secondary.base.color.scale_alpha(0.32),
                    width: 1.0,
                    radius: 6.0.into(),
                },
                ..Default::default()
            }
        })
        .into(),
    )
}


#[cfg(test)]
mod tests;

#[derive(Clone, Copy)]
pub struct PanelView<'a> {
    pub app_theme: &'a iced::Theme,
    pub desktop_locale: config_ui::DesktopLocale,
    pub agents_settings: &'a desktop_settings_agents::agents_state::AgentsSettingsState,
    pub project_path: &'a Option<String>,
    pub task_board_bulk_agent: &'a String,
    pub task_board_detail_tab: crate::message::TaskBoardDetailTab,
    pub task_board_draft: &'a desktop_task_model::TaskDraft,
    pub task_board_viewing_logs: &'a Option<desktop_task_model::Task>,
    pub task_board_worktree_snapshot: &'a Option<desktop_task_executor::executor::state::WorktreePoolSnapshot>,
    pub task_board_logs_editor_id: &'a iced::widget::Id,
    pub task_board_logs_context_menu_pos: Option<(f32, f32)>,
    pub current_line_height: f32,
    pub task_board_logs_scroll_top_line: f32,
    pub task_board_import_editor: &'a iced::widget::text_editor::Content,
    pub task_board_prompt_editor: &'a iced::widget::text_editor::Content,
    pub task_board_logs_editor: &'a iced::widget::text_editor::Content,
    pub task_board_clear_prompt_after_create: bool,
    pub task_board_close_after_create: bool,
    pub task_board_close_after_edit: bool,
    pub task_board_create_submit_success: bool,
    pub task_board_edit_submit_success: bool,
    pub task_board_is_import_mode: bool,
    pub task_board_logs_context_menu_open: bool,
}
#[derive(Debug, Clone)]
pub enum PanelMessage { TaskBoard(crate::message::TaskBoardMessage), Model(crate::model_selector::SelectorMessage), None, Copy(String), Append(String), OpenUrl(String), Selection { session_id: String, message_idx: usize, text_idx: usize, text: Option<String> } }
impl desktop_chat_view::message_view::markdown_selection::SelectionMessage for PanelMessage {
 fn none() -> Self { Self::None }
 fn copy(s: String) -> Self { Self::Copy(s) }
 fn context(e: desktop_ui_controls::text_editor_context_menu::TextContextEffect) -> Self {
 use desktop_ui_controls::text_editor_context_menu::TextContextEffect;
 match e { TextContextEffect::Copy(s) => Self::Copy(s), TextContextEffect::Append(s) => Self::Append(s), TextContextEffect::OpenUrl(s) => Self::OpenUrl(s) }
 }
 fn selection(session_id: String, message_idx: usize, text_idx: usize, text: Option<String>) -> Self { Self::Selection { session_id, message_idx, text_idx, text } }
}
fn settings_hover_help_label<'a>(i18n: I18n, theme: &Theme, title: &'a str, label: iced::widget::Text<'a>, description: impl Into<std::borrow::Cow<'a, str>>) -> iced::widget::Row<'a, Message> {
 desktop_ui_controls::system_settings_common::help::settings_hover_help_label(label, desktop_chat_view::message_view::markdown_selection::selectable_titled_text(title, description, Some(theme.palette().text), i18n))
}
pub struct PanelSlots<'a> {
 pub model: Option<iced::Element<'a, PanelMessage>>,
 pub executor: Option<iced::Element<'a, PanelMessage>>,
 pub reasoning: Option<iced::Element<'a, PanelMessage>>,
}

#[cfg(test)]
mod test_support;

/// Assemble the domain-owned selectors with the task panel. The host supplies only
/// catalogue projections and reasoning choices shared with chat.
pub fn build_with_controls<'a>(
    input: PanelView<'a>,
    edit: bool,
    model: crate::model_selector::SelectorView<'a>,
    executor: executor_selector::ExecutorView<'a>,
    reasoning: impl FnOnce() -> Option<Element<'a, TaskBoardMessage>>,
) -> Element<'a, PanelMessage> {
    build_with_controls_and_content(input, edit, model, executor, reasoning, |message| message, None)
}

pub fn build_with_controls_and_content<'a, M: Clone + 'a>(
    input: PanelView<'a>,
    edit: bool,
    model: crate::model_selector::SelectorView<'a>,
    executor: executor_selector::ExecutorView<'a>,
    reasoning: impl FnOnce() -> Option<Element<'a, TaskBoardMessage>>,
    map: impl Fn(PanelMessage) -> M + Copy + 'a,
    detail: Option<Element<'a, M>>,
) -> Element<'a, M> {
    let controls = if edit {
        input.task_board_detail_tab != TaskBoardDetailTab::Logs
    } else {
        !input.task_board_is_import_mode
    };
    let slots = PanelSlots {
        model: controls.then(|| {
            crate::model_selector::build_model_selector(model, edit).map(PanelMessage::Model)
        }),
        executor: controls.then(|| {
            if edit {
                executor_selector::build_edit_executor_selector(executor)
            } else {
                executor_selector::build_draft_agent_selector(executor, &input.agents_settings.entries)
            }
            .map(PanelMessage::TaskBoard)
        }),
        reasoning: controls.then(reasoning).flatten().map(|view| view.map(PanelMessage::TaskBoard)),
    };
    build_task_panel_with_content(input, edit, slots, map, detail)
}
