//! Source preview uses the repository modal's fixed header/footer and scroll area.
use crate::app::{App, Message, message::SettingsMessage};
use crate::app::message::settings::skills::repo_modal::Event;
use crate::app::components::system_settings_common::*;
use iced::widget::{button, column, row, text};
use iced::Element;
fn event(e: Event) -> Message { Message::Settings(SettingsMessage::SkillRepositories(e)) }

pub(super) fn view(app: &App) -> Element<'_, Message> {
    let s = &app.skills_settings.repositories;
    let mut content = column![
        settings_field_label("技能来源链接"),
        settings_text_input("ccswitch://v1/import?resource=skill&repo=owner/name", &s.source_link)
            .on_input_maybe((!s.busy).then_some(|v| event(Event::SourceLink(v)))),
        button("预览来源").padding([8, 10]).style(rounded_action_btn_style)
            .on_press_maybe((!s.busy).then_some(event(Event::PreviewSource))),
    ].spacing(10);
    if let Some(request) = &s.source_preview {
        content = content.push(text(format!("来源：GitHub {}/{}\n分支：{}\n启用：{}\n项目：{}\n主机：{}",
            request.repo.owner, request.repo.name, request.repo.branch, request.repo.enabled,
            request.directory, request.host)))
            .push(text("链接来自外部，请核对以上来源。确认只保存仓库，不安装技能。已有同名仓库将更新配置。"))
            .push(row![
                button("确认添加仓库").padding([8, 10]).style(primary_action_btn_style)
                    .on_press_maybe((!s.busy && s.snapshot.is_some()).then_some(event(Event::ConfirmSource))),
                button("取消预览").padding([8, 10]).style(rounded_action_btn_style)
                    .on_press_maybe((!s.busy).then_some(event(Event::CancelSource))),
            ].spacing(8));
    }
    content.into()
}
