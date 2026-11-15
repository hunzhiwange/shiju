//! 桌面组合尚属市场子域的内容，专家布局由领域库拥有。
use super::{public_skill_detail, public_skills};
use crate::app::{App, Message};
use crate::app::skill_hub::LoadState;
use crate::app::message::skill_hub::{PublicSkillMessage, SkillHubMessage};
use iced::Element;
pub(super) fn view(app: &App) -> Element<'_, Message> {
    let state = &app.expert_hub.public_market;
    if state.detail.is_some() { return public_skill_detail::view(app); }
    let content = match &state.sources {
        LoadState::Idle | LoadState::Loading { .. } => public_skills::loading_view(),
        LoadState::Error(error) => public_skills::error_view(
            crate::app::i18n::I18n::new(app.desktop_locale),
            &error.render(crate::app::i18n::I18n::new(app.desktop_locale)),
        ),
        LoadState::NeedsLogin => public_skills::error_view(
            crate::app::i18n::I18n::new(app.desktop_locale),
            crate::app::i18n::I18n::new(app.desktop_locale).__("公开市场不需要登录，请重试"),
        ),
        LoadState::Loaded(_) => {
            public_skills::suites_content(crate::app::i18n::I18n::new(app.desktop_locale), state)
        }
    };
    desktop_expert_hub::views::experts::view(app.desktop_locale, &state.query.keyword_draft, content,
        public_skills::source_selector(crate::app::i18n::I18n::new(app.desktop_locale), state),
        |message| Message::SkillHub(SkillHubMessage::Public(message)))
}
