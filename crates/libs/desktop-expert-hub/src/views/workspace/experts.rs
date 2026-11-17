//! 桌面组合尚属市场子域的内容，专家布局由领域库拥有。
use super::{public_skill_detail, public_skills};
use crate::message::skill_hub::SkillHubMessage;
use crate::views::workspace::{Message, ViewInput as WorkspaceInput};
use desktop_skill_hub::skill_hub::LoadState;
use iced::Element;
pub(super) fn view<'a>(app: &WorkspaceInput<'a>) -> Element<'a, Message> {
    let state = &app.expert_hub.public_market;
    if state.detail.is_some() {
        return public_skill_detail::view(app);
    }
    let content = match &state.sources {
        LoadState::Idle | LoadState::Loading { .. } => public_skills::loading_view(),
        LoadState::Error(error) => public_skills::error_view(
            desktop_i18n::I18n::new(app.desktop_locale),
            &error.render(desktop_i18n::I18n::new(app.desktop_locale)),
        ),
        LoadState::NeedsLogin => public_skills::error_view(
            desktop_i18n::I18n::new(app.desktop_locale),
            desktop_i18n::I18n::new(app.desktop_locale).__("公开市场不需要登录，请重试"),
        ),
        LoadState::Loaded(_) => {
            public_skills::suites_content(desktop_i18n::I18n::new(app.desktop_locale), state)
        }
    };
    crate::views::experts::view(
        app.desktop_locale,
        &state.query.keyword_draft,
        content,
        public_skills::source_selector(desktop_i18n::I18n::new(app.desktop_locale), state),
        |message| Message::SkillHub(SkillHubMessage::Public(message)),
    )
}

#[cfg(test)]
use crate::message::skill_hub::PublicSkillMessage;
