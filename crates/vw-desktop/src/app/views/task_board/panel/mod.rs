use crate::app::{App, Message};
mod executor_selector;
mod logs_modal;
mod model_selector;
mod reasoning_selector;
mod styles;
pub(crate) use executor_selector::build_bulk_executor_selector;
pub(crate) use model_selector::build_bulk_model_selector;
pub(crate) use reasoning_selector::build_bulk_reasoning_effort_selector;
fn input(app: &App) -> desktop_task_board::panel::PanelView<'_> {
 desktop_task_board::panel::PanelView {
 app_theme: &app.appearance.app_theme,
 desktop_locale: app.general_settings.desktop_locale,
 agents_settings: &app.agents_settings,
 project_path: &app.navigation_tabs.project_path,
 task_board_bulk_agent: &app.task_board.task_board_bulk_agent,
 task_board_bulk_task_split_mode: app.task_board.task_board_bulk_task_split_mode,
 task_board_detail_tab: app.task_board.task_board_detail_tab,
 task_board_draft: &app.task_board.task_board_draft,
 task_board_import_prompt_format: app.task_board.task_board_import_prompt_format,
 task_board_settings: &app.task_board.task_board_settings,
 task_board_viewing_logs: &app.task_board.task_board_viewing_logs,
 task_board_worktree_snapshot: &app.task_board.task_board_worktree_snapshot,
 task_board_logs_editor_id: &app.task_board.task_board_logs_editor_id,
 task_board_logs_context_menu_pos: app.task_board.task_board_logs_context_menu_pos,
 current_line_height: app.preview_state.current_line_height,
 task_board_logs_scroll_top_line: app.task_board.task_board_logs_scroll_top_line,
 task_board_new_subtask_content: &app.task_board.task_board_new_subtask_content,
 task_board_import_editor: &app.task_board.task_board_import_editor,
 task_board_prompt_editor: &app.task_board.task_board_prompt_editor,
 task_board_logs_editor: &app.task_board.task_board_logs_editor,
 task_board_clear_prompt_after_create: app.task_board.task_board_clear_prompt_after_create,
 task_board_close_after_create: app.task_board.task_board_close_after_create,
 task_board_close_after_edit: app.task_board.task_board_close_after_edit,
 task_board_create_submit_success: app.task_board.task_board_create_submit_success,
 task_board_edit_submit_success: app.task_board.task_board_edit_submit_success,
 task_board_import_prompt_collapsed: app.task_board.task_board_import_prompt_collapsed,
 task_board_is_import_mode: app.task_board.task_board_is_import_mode,
 task_board_logs_context_menu_open: app.task_board.task_board_logs_context_menu_open,
 }
}
fn map(message: desktop_task_board::panel::PanelMessage) -> Message {
 match message { desktop_task_board::panel::PanelMessage::TaskBoard(m) => Message::TaskBoard(m), desktop_task_board::panel::PanelMessage::Model(m) => model_selector::map(m),
 desktop_task_board::panel::PanelMessage::None => Message::None,
 desktop_task_board::panel::PanelMessage::Copy(s) => Message::CopyCode(s),
 desktop_task_board::panel::PanelMessage::Append(s) => crate::app::components::text_editor_context_menu::text_context_message(desktop_ui_controls::text_editor_context_menu::TextContextEffect::Append(s)),
 desktop_task_board::panel::PanelMessage::OpenUrl(s) => crate::app::components::text_editor_context_menu::text_context_message(desktop_ui_controls::text_editor_context_menu::TextContextEffect::OpenUrl(s)),
 desktop_task_board::panel::PanelMessage::Selection { session_id, message_idx, text_idx, text } => Message::ChatMarkdownSelectionChanged { session_id, message_idx, text_idx, text } }
}
pub fn build_task_panel(app: &App, edit: bool) -> iced::Element<'_, Message> {
 let panel = desktop_task_board::panel::build_with_controls(
 input(app), edit, model_selector::view(app), executor_selector::input(app),
 || reasoning_selector::domain_build(app, edit, false),
 ).map(map);
 if let Some(task) = app.task_board.task_board_viewing_logs.as_ref() {
     let labels = (edit && app.task_board.task_board_editing_task_id.as_ref() == Some(&task.id))
         .then(|| task_labels(app, task));
     let personal = app.navigation_tabs.project_path.as_ref()
         .and_then(|path| app.task_board.personal_tasks.get(path))
         .is_some_and(|ids| ids.contains(&task.id));
     let body = iced::widget::column![
         iced::widget::button(iced::widget::text(if personal { "移出我的任务" } else { "加入我的任务" }))
             .padding([8, 12])
             .style(crate::app::components::system_settings_common::rounded_action_btn_style)
             .on_press(Message::TaskBoard(crate::app::message::TaskBoardMessage::TogglePersonalTask(task.id.clone()))),
     ].spacing(8);
     body.extend(labels).push(panel).into()
 } else { panel }
}
fn task_labels<'a>(app: &'a App, task: &'a desktop_task_model::Task) -> iced::Element<'a, Message> {
    use crate::app::components::system_settings_common::*;
    use crate::app::message::TaskBoardMessage;
    use iced::widget::{button, checkbox, column, container, row, scrollable, text};

    let board = &app.task_board;
    let i18n = crate::app::i18n::I18n::new(app.general_settings.desktop_locale);
    let pending = board.task_label_save_in_progress.is_some();
    let ids = board.task_board_draft.label_ids.as_ref().unwrap_or(&task.label_ids);
    let choices = board.task_labels.iter().filter_map(|label| {
        let selected = ids.contains(&label.id);
        if label.archived && !selected {
            return None;
        }
        let id = label.id.clone();
        let label_text = if label.archived {
            format!("{} · {}", label.name, i18n.__("已归档"))
        } else {
            label.name.clone()
        };
        let checkbox = checkbox(selected).label(label_text).style(settings_checkbox_style);
        let checkbox = if pending {
            checkbox
        } else {
            checkbox.on_toggle(move |_| Message::TaskBoard(TaskBoardMessage::ToggleEditingTaskLabel(id.clone())))
        };
        Some(checkbox.into())
    }).collect::<Vec<iced::Element<'_, Message>>>();
    let clear = button(text(i18n.__("清空")).size(12))
        .padding([6, 10])
        .style(rounded_action_btn_style)
        .on_press_maybe((!pending && !ids.is_empty()).then_some(
            Message::TaskBoard(TaskBoardMessage::ClearEditingTaskLabels),
        ));
    let save = button(text(i18n.__("保存标签")).size(12))
        .padding([6, 10])
        .style(primary_action_btn_style)
        .on_press_maybe((!pending && board.task_board_draft.label_ids.is_some()).then_some(
            Message::TaskBoard(TaskBoardMessage::SaveEditingTaskLabels),
        ));
    let header = row![settings_field_label(i18n.__("标签")), clear, save]
        .spacing(8).align_y(iced::Alignment::Center);
    column![
        header,
        container(
            scrollable(container(row(choices).spacing(8).wrap()).padding([0, 7]))
                .height(iced::Length::Shrink)
                .direction(scrollable::Direction::Vertical(
                    scrollable::Scrollbar::new().width(4).scroller_width(4),
                ))
        ).max_height(88),
    ].spacing(4).into()
}

pub(crate) fn build_bulk_task_split_selector(app: &App) -> iced::Element<'_, Message> { desktop_task_board::panel::build_bulk_task_split_selector(input(app)).map(map) }
pub(crate) fn build_bulk_agent_selector(app: &App) -> iced::Element<'_, Message> { desktop_task_board::panel::build_bulk_agent_selector(input(app)).map(map) }

#[cfg(test)]
mod tests;
