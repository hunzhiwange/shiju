//! Management entry points; opening this view never submits a write operation.
use crate::app::{App, Message};
use crate::app::components::{
    app_routing, system_settings_common::rounded_action_btn_style,
    system_settings_data, system_settings_device_sync,
};
use iced::widget::{button, column, container, text};
use iced::{Element, Length};
use vw_api_app_identity::AppId;

/// The parent owns application selection. Never infer a target from a display name.
pub(crate) fn view(app: &App, selected: Option<AppId>) -> Element<'_, Message> {
    let catalog = &app.apps_catalog;
    let target = selected.filter(|id| catalog.rows.iter().any(|row| row.entry.id == *id));
    let mut content = column![
        text("管理工具").size(20),
        text(format!("执行主机：{}", catalog.host)),
        text("应用工具作用于所选智能体；备份恢复与设备同步使用各自窗口中的身份及资源范围。"),
    ].spacing(12);

    if let Some(id) = target {
        content = content
            .push(text(format!("当前应用：{id:?}")))
            .push(app_routing::config::timeout_button(id))
            .push(app_routing::outbound::entry(id))
            .push(app_routing::requests::entry(id))
            .push(button(text("高可用策略"))
                .padding([8, 12])
                .style(rounded_action_btn_style)
                .on_press(crate::app::message::agent_apps_high_availability_policy::message(
                    crate::app::message::agent_apps_high_availability_policy::Event::Open(id),
                )));
    } else {
        content = content.push(text("请先选择当前主机目录中的智能体，再管理应用路由与高可用策略。"));
    }

    content = content
        .push(text("主机路由服务").size(16))
        .push(app_routing::lifecycle::view(catalog))
        .push(text("备份与恢复").size(16))
        .push(text("打开窗口后选择身份和备份，恢复前预览并确认；此处不会自动覆盖数据。"))
        .push(system_settings_data::backup_list_ui::view(app))
        .push(system_settings_data::recovery_ui::view(app))
        .push(text("设备同步").size(16))
        .push(text("仅管理已有 WebDAV / S3 同步。请在窗口中选择资源、查看预览并确认。"))
        .push(system_settings_device_sync::entry_points());
    container(content).padding([0, 7]).width(Length::Fill).into()
}

/// Reuse the existing confirmation flows, including their original request targets.
/// The parent must call this once instead of mounting these overlays twice.
pub(crate) fn overlay<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    let base = app_routing::requests::overlay(&app.apps_catalog, base);
    let base = app_routing::config::overlay(&app.apps_catalog, base);
    let base = iced::widget::stack![base,
        desktop_apps_ui::app_routing::outbound::overlay(
            &app.apps_catalog.routing, iced::widget::Space::new().into(),
        ).map(|event| Message::Settings(crate::app::message::settings::SettingsMessage::AppsCatalog(
            crate::app::message::settings::AppsCatalogMessage::Routing(event),
        )))
    ].into();
    let state = &app.apps_catalog.high_availability;
    let base = if state.open {
        use desktop_apps_ui::agent_apps_high_availability::policy_modal as domain;
        use crate::app::message::agent_apps_high_availability_policy as policy;
        iced::widget::stack![base, domain::overlay(domain::State {
            open: state.open, busy: state.busy, loaded: state.loaded,
            fields: &state.fields, error: &state.error,
        }, iced::widget::Space::new().into()).map(|event| policy::message(match event {
            domain::Event::Close => policy::Event::Close,
            domain::Event::Edit(i, value) => policy::Event::Edit(i, value),
            domain::Event::Save => policy::Event::Save,
            domain::Event::Reload => policy::Event::Reload,
        }))].into()
    } else {
        base
    };
    let base = system_settings_data::backup_list_ui::overlay(app, base);
    let base = system_settings_data::recovery_ui::overlay(app, base);
    system_settings_device_sync::overlay(app, base)
}
