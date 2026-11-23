//! Launch links remain queued until handed to the scoped repository confirmation.
use crate::app::message::settings::skills::repo_modal;
use crate::app::{
    App, Message, deep_links,
    message::settings::{GeneralSettingsMessage, SettingsMessage},
};
use iced::{Element, Task};

use desktop_platform::skill_link_preview::{parse, pending};

pub(crate) fn handoff(app: &mut App, id: u64) -> Task<Message> {
    let Some(link) = pending().filter(|link| link.id == id) else { return Task::none() };
    if app.skills_settings.repositories.open {
        return app.show_localized_toast(
            crate::app::state::ToastKind::Error,
            "请先关闭仓库管理弹窗",
            vec![],
        );
    }
    let Some(directory) = app.navigation_tabs.project_path.clone() else {
        return app.show_localized_toast(
            crate::app::state::ToastKind::Error,
            "请先打开项目，再预览技能链接",
            vec![],
        );
    };
    let request = match parse(&link.url, directory, app.apps_catalog.host.clone()) {
        Ok(request) => request,
        Err(error) => {
            return app.show_error_toast(
                desktop_i18n::I18n::new(app.general_settings.desktop_locale).__(&error),
            );
        }
    };
    let navigation = crate::app::message::settings::update(
        app,
        SettingsMessage::SystemSearchTargetSelected(
            desktop_settings_shell::search::SystemSettingsSearchTarget::ThirdParty(
                desktop_ui_controls::settings_navigation::ThirdPartyPage::Skills,
            ),
        ),
    );
    if !crate::app::components::third_party_settings::is_visible(app)
        || app.apps_catalog.third_party_page != desktop_ui_controls::settings_navigation::ThirdPartyPage::Skills
    {
        return navigation;
    }
    let task = Task::batch([navigation, repo_modal::update(app, repo_modal::Event::Open)]);
    if !app.skills_settings.repositories.open {
        return task;
    }
    // Open captures gateway/project/host and starts a read only SDK list request.
    // The existing ConfirmSource action alone may persist this repository.
    app.skills_settings.repositories.source_link = link.url;
    app.skills_settings.repositories.source_preview = Some(request);
    let _ = deep_links::acknowledge(id);
    task
}

pub(crate) fn overlay<'a>(app: &App, base: Element<'a, Message>) -> Element<'a, Message> {
    let Some(link) = pending() else { return base };
    let close = Message::Settings(SettingsMessage::General(
        GeneralSettingsMessage::SkillLinkRejected(link.id),
    ));
    let accept = Message::Settings(SettingsMessage::General(
        GeneralSettingsMessage::SkillLinkPreview(link.id),
    ));
    desktop_navigation::view_layout::link_preview::skill(
        base,
        &link.url,
        desktop_i18n::I18n::new(app.general_settings.desktop_locale),
        close,
        &app.apps_catalog.host,
        app.navigation_tabs.project_path.as_deref(),
        app.skills_settings.repositories.open,
        accept,
    )
}
