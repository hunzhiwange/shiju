//! 定时任务的提示词编辑器及独立于当前会话的执行选项。
use super::*;
use crate::message::SettingsMessage as S;
use crate::state::CronJobDraft;
use desktop_ui_controls::system_settings_common::settings_toggle;
use iced::widget::{column, responsive, tooltip};

pub(super) fn view<'a>(
    app: &ViewData<'_>,
    draft: &'a CronJobDraft,
    editing: bool,
) -> Element<'a, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let workspace = project_field_builder(app, &draft.project_path, |path| {
        Message::Settings(S::CronJobEditProjectPathChanged(path))
    });
    let shell = draft.job_type == CronAddJobType::Shell;
    let mut content = column![].spacing(10).width(Length::Fill);
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
            .placeholder(i18n.__("要让 Agent 执行的任务"))
            .on_action(|action| Message::Settings(S::CronJobEditPromptEditorAction(action)))
    };
    content = content.push(editor.height(Length::Fixed(116.0)).style(|theme, status| {
        let mut style = settings_text_editor_style(theme, status);
        style.background = Background::Color(iced::Color::TRANSPARENT);
        if !matches!(status, text_editor::Status::Focused { .. }) {
            style.border.width = 0.0;
        }
        style
    }));

    let controls = responsive(move |size| {
        let mut controls = row![].spacing(8).align_y(Alignment::Center);
        let workspace = container(workspace()).width(Length::Fill);
        if !shell {
            controls = controls.push(access_toggle(i18n, draft));
        }
        if size.width >= 460.0 {
            row![workspace, controls].spacing(12).align_y(Alignment::Center).into()
        } else {
            column![workspace, controls.wrap()].spacing(8).into()
        }
    })
    .height(Length::Shrink);

    if !shell {
        content = content.push(super::composer_picker::controls(app, draft, editing));
    }
    let content = container(content).width(Length::Fill).padding(8).style(composer_style);
    container(column![content, container(controls).padding([0, 8])].spacing(8))
        .padding(4)
        .width(Length::Fill)
        .style(|theme| {
            let mut style = composer_style(theme);
            style.background = Some(theme.extended_palette().background.weak.color.into());
            style.border.width = 0.0;
            style
        })
        .into()
}

fn access_toggle(i18n: I18n, draft: &CronJobDraft) -> Element<'static, Message> {
    let label = if draft.full_access { i18n.__("完全访问") } else { i18n.__("受限") };
    let trigger = settings_toggle(draft.full_access)
        .label(label)
        .on_toggle(|enabled| Message::Settings(S::CronJobEditFullAccessToggled(enabled)));
    tooltip(trigger, text(i18n.__("获取完全访问权限")).size(12), tooltip::Position::Top)
        .style(settings_modal_card_style)
        .into()
}

fn composer_style(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(theme.palette().background.into()),
        border: iced::Border {
            color: theme.palette().text.scale_alpha(0.13),
            width: 1.0,
            radius: 14.0.into(),
        },
        ..Default::default()
    }
}
