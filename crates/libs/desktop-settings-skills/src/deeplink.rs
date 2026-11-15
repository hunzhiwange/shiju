use desktop_ui_controls::system_settings_common::*;
// Source preview uses the repository modal's fixed header/footer and scroll area.
use iced::Element;
use iced::widget::{button, column, row, text};

pub fn view<'a, M: Clone + 'static>(s: ViewState<'a>, message: fn(Event) -> M) -> Element<'a, M> {
    let mut content = column![
        settings_field_label("技能来源链接"),
        settings_text_input("ccswitch://v1/import?resource=skill&repo=owner/name", s.source_link)
            .on_input_maybe((!s.busy).then_some(move |v| message(Event::SourceLink(v)))),
        button("预览来源")
            .padding([8, 10])
            .style(rounded_action_btn_style)
            .on_press_maybe((!s.busy).then_some(message(Event::PreviewSource))),
    ]
    .spacing(10);
    if let Some(request) = s.source_preview {
        content = content.push(text(format!("来源：GitHub {}/{}\n分支：{}\n启用：{}\n项目：{}\n主机：{}",
            request.repo.owner, request.repo.name, request.repo.branch, request.repo.enabled,
            request.directory, request.host)))
            .push(text("链接来自外部，请核对以上来源。确认只保存仓库，不安装技能。已有同名仓库将更新配置。"))
            .push(row![
                button("确认添加仓库").padding([8, 10]).style(primary_action_btn_style)
                    .on_press_maybe((!s.busy && s.snapshot.is_some()).then_some(message(Event::ConfirmSource))),
                button("取消预览").padding([8, 10]).style(rounded_action_btn_style)
                    .on_press_maybe((!s.busy).then_some(message(Event::CancelSource))),
            ].spacing(8));
    }
    content.into()
}
#[derive(Clone, Copy)]
pub struct ViewState<'a> {
    pub source_link: &'a String,
    pub busy: bool,
    pub source_preview: &'a Option<desktop_platform::skill_deeplink::Request>,
    pub snapshot: &'a Option<vw_api_app_skills::repos_api::SkillReposResponse>,
}

#[derive(Clone, Debug)]
pub enum Event {
    SourceLink(String),
    PreviewSource,
    CancelSource,
    ConfirmSource,
}
