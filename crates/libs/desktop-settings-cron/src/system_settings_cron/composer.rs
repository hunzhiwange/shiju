//! 定时任务的提示词编辑器及独立于当前会话的执行选项。
use super::*;
use crate::message::SettingsMessage as S;
use crate::state::{CronEditorPicker as Picker, CronEditorPickerAction as Action, CronJobDraft};
use desktop_chat_input::input_panel::{
    quick_messages::{WorkspacePicker, workspace_picker},
    send_controls::permission_mode,
};
use iced::widget::column;

pub(super) fn view<'a>(
    app: &ViewData<'a>,
    draft: &'a CronJobDraft,
    editing: bool,
) -> Element<'a, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let action = move |action| Message::Settings(S::CronEditorPickerChanged { editing, action });
    let close = action(Action::Close);
    let project = draft.project_path.trim();
    let project_open = draft.editor_picker == Some(Picker::Project);
    let workspace = workspace_picker(
        WorkspacePicker {
            i18n,
            projects: app.recent_projects,
            project_labels: app.recent_project_labels,
            selected: (!project.is_empty()).then_some(project),
            current: app.project_path,
            query: &draft.editor_picker_query,
            open: project_open,
            within_window: true,
        },
        if project_open { close.clone() } else { action(Action::Open(Picker::Project)) },
        close.clone(),
        move |query| action(Action::Search(query)),
        |path| action(Action::ProjectSelected(path)),
        (!project.is_empty()).then(|| action(Action::ProjectSelected(String::new()))),
        (!cfg!(target_arch = "wasm32")).then(|| action(Action::BrowseProject)),
    );
    let shell = draft.job_type == CronAddJobType::Shell;
    let mut content = column![].width(Length::Fill);
    if !shell && !draft.skills.is_empty() {
        let chips = row(draft.skills.iter().map(|id| {
            let label = app
                .skills
                .iter()
                .find(|option| &option.id == id)
                .map_or(id, |option| &option.label);
            secondary_button(
                format!("{label} ×"),
                Message::Settings(S::CronEditorSkillToggled { editing, id: id.clone() }),
            )
        }))
        .spacing(6)
        .wrap();
        content = content.push(chips);
    }
    let editor = if shell {
        settings_text_editor(&draft.command_editor)
            .placeholder(i18n.__("Shell 命令"))
            .on_action(|action| Message::Settings(S::CronJobEditCommandEditorAction(action)))
    } else {
        settings_text_editor(&draft.prompt_editor)
            .placeholder(i18n.__("添加提示词"))
            .on_action(|action| Message::Settings(S::CronJobEditPromptEditorAction(action)))
    };
    content = content.push(editor.height(Length::Fixed(120.0)));

    if !shell {
        content = content.push(super::composer_picker::controls(app, draft, editing));
    }

    let mut controls = row![workspace].spacing(10).align_y(Alignment::Center);
    if !shell {
        let open = draft.editor_picker == Some(Picker::Permission);
        controls = controls.push(permission_mode(
            i18n,
            !draft.submitting,
            draft.full_access,
            open,
            if open { close.clone() } else { action(Action::Open(Picker::Permission)) },
            close,
            action(Action::PermissionSelected(!draft.full_access)),
        ));
    }

    content.push(controls.wrap().vertical_spacing(4)).spacing(8).into()
}
