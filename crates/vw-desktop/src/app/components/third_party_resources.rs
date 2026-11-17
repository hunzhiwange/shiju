//! Thin resource-page composition. Navigation and refresh dispatch remain with the host.
use crate::app::{App, Message};
use iced::{Element, Length, widget::{button, column, container, scrollable, text}};
use vw_api_app_identity::AppId;

use super::system_settings_common::{
    rounded_action_btn_style, settings_muted_text_style, settings_page_heading,
};

/// The catalog retains its own host, request generation, reader and maintenance overlays.
/// Session identities carry their application identity; this catalog spans applications.
pub(crate) fn sessions(app: &App) -> Element<'_, Message> {
    column![
        settings_page_heading("会话", "浏览当前执行主机授权范围内的第三方会话。"),
        super::external_sessions::view(&app.apps_catalog.external_sessions),
    ]
    .spacing(16)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

/// Keep account login, binding and confirmation messages with the authorization owner.
/// The host must also compose system_settings_auth::overlay for this page.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn authorization(app: &App) -> Element<'_, Message> {
    super::system_settings_auth::view(app)
}

/// Render only the selected application's providers and bound accounts. No totals are
/// synthesized: snapshots may represent different units, periods and query capabilities.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn usage(app: &App, selected: AppId) -> Element<'_, Message> {
    use super::apps_usage_cards;
    use crate::app::message::app_providers::list::{ListMessage, message};

    let provider_scope = apps_usage_cards::current_scope(app, false);
    let account_scope = apps_usage_cards::current_scope(app, true);
    let mut content = column![
        settings_page_heading("用量与额度", "按应用、供应商和授权账号查看真实查询结果。"),
        text(format!("应用：{selected:?} · 执行主机：{}", app.apps_catalog.host)),
        text("暂不支持跨应用或跨账号汇总；尚未读取的数据不计为零。")
            .style(settings_muted_text_style),
    ].spacing(12).width(Length::Fill);

    let providers_match = app.app_providers.scope.as_ref().is_some_and(|scope| {
        scope.app == selected && scope.host == app.apps_catalog.host
            && app.apps_catalog.context.as_ref().is_some_and(|context| {
                scope.context.gateway == context.gateway
                    && scope.context.tenant == context.tenant
                    && scope.context.project == context.project
            })
    });
    let open = || button(text("读取应用供应商"))
        .padding([8, 12]).style(rounded_action_btn_style)
        .on_press(message(ListMessage::Open(selected)));
    if !providers_match {
        content = content.push(text("尚未读取此应用在当前主机上的供应商。"))
            .push(open());
    } else {
        content = content.push(button(text("刷新供应商"))
            .padding([8, 12]).style(rounded_action_btn_style)
            .on_press_maybe((!app.app_providers.loading).then(|| message(ListMessage::Refresh))));
        if app.app_providers.loading {
            content = content.push(text("正在读取供应商…"));
        }
        if app.app_providers.error.is_some() {
            content = content.push(text("供应商读取失败，请刷新重试。"));
        }
        match &app.app_providers.snapshot {
            Some(snapshot) => {
                let mut count = 0;
                for entry in &snapshot.catalog.rows {
                    if entry.provider.app_id != selected { continue; }
                    count += 1;
                    content = content.push(column![
                        text(format!("供应商：{}", entry.provider.id)),
                        apps_usage_cards::provider_view(provider_scope.as_ref(),
                            &app.app_providers.usage, selected, &entry.provider.id),
                    ].spacing(6));
                }
                if count == 0 { content = content.push(text("此应用暂无供应商用量数据。")); }
            }
            None => { content = content.push(text("供应商数据尚未加载。")); }
        }
    }
    content = content.push(text("授权账号").size(18));
    // Resolve the authorization scope independently: a management connection can
    // target a different host and must never borrow the catalog's scope.
    if account_scope.as_ref().is_some_and(|scope| scope.host == app.apps_catalog.host) {
        let mut count = 0;
        for service in app.agent_apps_auth.rows.iter().flatten() {
            if apps_usage_cards::binding_app(service.service) != Some(selected) { continue; }
            match &service.accounts {
                Ok(accounts) => for account in accounts {
                    count += 1;
                    content = content.push(column![
                        text(format!("账号：{}", account.account_id)),
                        apps_usage_cards::account_view(account_scope.as_ref(),
                            &app.agent_apps_auth.usage, service.service, &account.account_id),
                    ].spacing(6));
                },
                Err(_) => { content = content.push(text("账号读取失败，请在授权页面刷新重试。")); }
            }
        }
        if count == 0 { content = content.push(text("暂无已加载的授权账号用量。")); }
    } else {
        content = content.push(text("请在授权页面刷新账号。"));
    }
    scrollable(container(content).padding([0, 7]).width(Length::Fill))
        .direction(scrollable::Direction::Vertical(
            scrollable::Scrollbar::new().width(4).scroller_width(4),
        ))
        .height(Length::Fill).into()
}
