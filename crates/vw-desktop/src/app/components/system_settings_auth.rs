//! Scoped authorization center and temporary management connection form.

use super::apps_usage_cards::{self, Scope};

mod account_action_modal;
mod binding_modal;
#[cfg(test)]
mod binding_modal_tests;
mod login_modal;
#[cfg(test)]
pub(in crate::app) mod login_modal_tests;

use super::system_settings_common::{
    icon_svg, primary_action_btn_style, provider_logo_svg, rounded_action_btn_style,
    settings_checkbox_style, settings_close_button, settings_divider, settings_error_banner,
    settings_field_label, settings_group_card, settings_icon_tile_style, settings_modal_header,
    settings_modal_layout, settings_modal_overlay_with_escape, settings_muted_text_style,
    settings_page_heading, settings_panel_style, settings_text_input, settings_value_badge,
};
use crate::app::{
    App, Message,
    assets::Icon,
    message::agent_apps_auth::{Event, actions::ActionEvent, message},
    state::agent_apps_auth::{AccountOperation, LoginTarget, ServiceAccounts, State},
};
use iced::widget::{button, checkbox, column, container, responsive, row, scrollable, svg, text};
use iced::{Alignment, Element, Length, Theme};
use vw_gateway_client::vw_api_types::agent_apps_auth::{
    AuthAccount, AuthCredentialState, AuthRegion, AuthService,
};

pub(crate) fn view(app: &App) -> Element<'_, Message> {
    let state = &app.agent_apps_auth;
    if let Some(binding) = &state.binding {
        return container(settings_page_heading(
            service_name(binding.service),
            "在授权中心为供应商选择当前执行主机上的授权账号。",
        ))
        .padding([0, 7])
        .width(Length::Fill)
        .height(Length::Fill)
        .into();
    }
    let usage_scope = apps_usage_cards::current_scope(app, true);
    let heading = settings_page_heading("授权中心", "查看当前公司在授权执行主机上的账号状态。");
    let content = column![
        heading,
        toolbar(state, usage_scope.as_ref()),
        login_preferences(state),
        services(state, usage_scope.as_ref()),
    ]
    .spacing(16)
    .width(Length::Fill);
    scrollable(container(content).padding(iced::Padding::new(7.0).top(0)).width(Length::Fill))
        .direction(scrollable::Direction::Vertical(
            scrollable::Scrollbar::new().width(4).scroller_width(4),
        ))
        .height(Length::Fill)
        .into()
}

fn toolbar(state: &State, usage_scope: Option<&Scope>) -> Element<'static, Message> {
    let configured = state.connection.is_some();
    let idle = state.pending.is_none() && state.account_action.is_none();
    let failed = state.connection_error.is_some()
        || state.rows.as_ref().is_some_and(|rows| rows.iter().all(|row| row.accounts.is_err()));
    let refresh_label = if failed { "重试" } else { "刷新" };
    let show_management = !cfg!(unix)
        || failed
        || state.connection.as_ref().is_some_and(|connection| connection.valid_management());
    let batch_pending = usage_scope.is_some_and(|scope| state.usage.batch_pending(scope));
    let actions = row![
        button(text(refresh_label).size(14))
            .style(primary_action_btn_style)
            .padding([8, 14])
            .on_press_maybe(
                (state.loopback && idle && (configured || cfg!(unix)))
                    .then(|| message(Event::Refresh)),
            ),
        button(text(if batch_pending { "正在刷新额度…" } else { "刷新全部额度" }).size(14))
            .style(rounded_action_btn_style)
            .padding([8, 14])
            .on_press_maybe(
                (state.loopback
                    && configured
                    && idle
                    && state.rows.is_some()
                    && usage_scope.is_some()
                    && !batch_pending)
                    .then(|| apps_usage_cards::message_event(apps_usage_cards::Event::RefreshAll))
            ),
    ];
    let actions = if state.loopback && show_management {
        actions.push(
            button(text("管理连接").size(14))
                .style(rounded_action_btn_style)
                .padding([8, 14])
                .on_press_maybe(idle.then(|| message(Event::Open))),
        )
    } else {
        actions
    };
    actions.spacing(8).align_y(Alignment::Center).wrap().into()
}

fn login_preferences(state: &State) -> Element<'_, Message> {
    let mut context = row![].spacing(12).align_y(Alignment::Center);
    if let Some(scope) = &state.scope {
        context = context.push(
            text(format!(
                "平台 {} · 公司 {}",
                scope.tenant.platform_id().as_ref(),
                scope.tenant.company_id().as_ref()
            ))
            .size(12)
            .wrapping(text::Wrapping::WordOrGlyph)
            .style(settings_muted_text_style),
        );
    }
    if let Some(connection) = &state.connection {
        let host = if connection.host == vw_shared::local_apps::HOST_ID {
            "本机"
        } else {
            connection.host.trim()
        };
        context = context.push(
            text(format!("执行主机 {host}"))
                .size(12)
                .wrapping(text::Wrapping::WordOrGlyph)
                .style(settings_muted_text_style),
        );
    }
    let preferences = column![
        text("登录偏好").size(16),
        checkbox(!state.copy_login_link)
            .label("登录时自动打开浏览器")
            .on_toggle(|open| message(Event::OpenBrowserPreference(open)))
            .style(settings_checkbox_style),
        text("关闭后复制登录链接，可在自己选择的浏览器中打开。")
            .size(13)
            .style(settings_muted_text_style),
    ]
    .spacing(8);
    let preferences = if state.scope.is_some() || state.connection.is_some() {
        preferences.push(settings_divider()).push(context.wrap())
    } else {
        preferences
    };
    settings_group_card(container(preferences).padding([14, 0])).into()
}

fn services<'a>(state: &'a State, usage_scope: Option<&Scope>) -> Element<'a, Message> {
    if !state.loopback {
        return settings_error_banner("授权中心需要本机回环管理连接。请在网关设置中选择本机连接。");
    }
    if state.pending.is_some() {
        return list_placeholder(Icon::Clock, "正在读取授权账号…");
    }
    if let Some(error) = &state.connection_error {
        return settings_error_banner(error.as_str());
    }
    let Some(rows) = &state.rows else {
        return if cfg!(unix) {
            list_placeholder(Icon::Clock, "正在准备本机授权…")
        } else {
            list_placeholder(Icon::ShieldLock, "请打开管理连接读取授权账号。")
        };
    };

    let heading =
        row![text("授权服务").size(17).width(Length::Fill), settings_value_badge(rows.len()),]
            .align_y(Alignment::Center);
    let mut list = column![heading].spacing(10).width(Length::Fill);
    let report = usage_scope.map_or_else(Vec::new, |scope| state.usage.batch_lines(scope));
    if !report.is_empty() {
        list =
            list.push(
                container(
                    column(report.into_iter().map(|line| {
                        text(line).size(13).wrapping(text::Wrapping::WordOrGlyph).into()
                    }))
                    .spacing(6),
                )
                .padding([12, 14])
                .width(Length::Fill)
                .style(settings_panel_style),
            );
    }
    if rows.is_empty() {
        return list.push(list_placeholder(Icon::ShieldLock, "暂无授权服务")).into();
    }
    for service in rows
        .iter()
        .filter(|row| Some(row.service) == state.focused_service)
        .chain(rows.iter().filter(|row| Some(row.service) != state.focused_service))
    {
        list = list.push(service_card(service, state, usage_scope));
    }
    list.into()
}

fn list_placeholder(icon: Icon, label: &'static str) -> Element<'static, Message> {
    container(
        column![
            container(
                icon_svg(icon, 20.0)
                    .style(|theme: &Theme, _| svg::Style { color: Some(theme.palette().text) })
            )
            .center_x(44)
            .center_y(44)
            .style(settings_icon_tile_style),
            text(label).size(14).style(settings_muted_text_style),
        ]
        .spacing(10)
        .align_x(Alignment::Center),
    )
    .center_x(Length::Fill)
    .padding([22, 12])
    .style(settings_panel_style)
    .into()
}

fn service_name(service: AuthService) -> &'static str {
    match service {
        AuthService::ClaudeOfficial => "Claude 官方账号",
        AuthService::CodexOauth => "Codex / ChatGPT",
        AuthService::GithubCopilot => "GitHub Copilot",
        AuthService::XaiOauth => "xAI",
        AuthService::KimiOauth => "Kimi Code",
        AuthService::AgyOauth => "Antigravity",
        AuthService::DshAccount => "DeepSeek Harness",
    }
}

fn service_card<'a>(
    service: &'a ServiceAccounts,
    state: &'a State,
    usage_scope: Option<&Scope>,
) -> Element<'a, Message> {
    let heading = if state.connection.is_some() {
        responsive(move |size| {
            let summary = service_summary(service);
            let actions = login_actions(service.service);
            if size.width >= 520.0 {
                row![summary, actions].spacing(12).align_y(Alignment::Center).into()
            } else {
                column![summary, actions].spacing(10).into()
            }
        })
        .height(Length::Shrink)
        .into()
    } else {
        service_summary(service)
    };

    let mut card = column![heading].spacing(12);
    match &service.accounts {
        Ok(accounts) => {
            for account in accounts {
                card = card.push(settings_divider()).push(account_row(
                    account,
                    service.service,
                    state,
                    usage_scope,
                ));
            }
        }
        Err(reason) => {
            card = card.push(settings_error_banner(*reason)).push(
                button(text("重试此服务").size(13))
                    .style(rounded_action_btn_style)
                    .padding([6, 12])
                    .on_press_maybe(
                        (state.connection.is_some()
                            && state.pending.is_none()
                            && state.login.is_none()
                            && state.account_action.is_none()
                            && state.form.is_none())
                            .then(|| message(Event::RetryService(service.service))),
                    ),
            );
        }
    }

    let focused = Some(service.service) == state.focused_service;
    container(card)
        .padding([12, 14])
        .width(Length::Fill)
        .style(move |theme| {
            let mut style = settings_panel_style(theme);
            if focused {
                style.border.color = theme.palette().primary;
            }
            style
        })
        .into()
}

fn service_summary(service: &ServiceAccounts) -> Element<'static, Message> {
    let logo = match service.service {
        AuthService::ClaudeOfficial => provider_logo_svg("anthropic", 21.0),
        AuthService::CodexOauth => provider_logo_svg("openai", 21.0),
        AuthService::GithubCopilot => provider_logo_svg("github-copilot", 21.0),
        AuthService::XaiOauth => provider_logo_svg("xai", 21.0),
        AuthService::KimiOauth => provider_logo_svg("kimi-for-coding", 21.0),
        AuthService::AgyOauth => icon_svg(Icon::AppAntigravity, 21.0),
        AuthService::DshAccount => provider_logo_svg("deepseek", 21.0),
    };
    let mut labels = column![text(service_name(service.service)).size(16)].spacing(3);
    if let Ok(accounts) = &service.accounts {
        let count = if accounts.is_empty() {
            "暂无授权账号".into()
        } else {
            format!("{} 个账号", accounts.len())
        };
        labels = labels.push(text(count).size(12).style(settings_muted_text_style));
    }
    row![
        container(logo).center_x(44).center_y(44).style(settings_icon_tile_style),
        labels.width(Length::Fill),
    ]
    .align_y(Alignment::Center)
    .spacing(12)
    .width(Length::Fill)
    .into()
}

fn account_row<'a>(
    account: &'a AuthAccount,
    service: AuthService,
    state: &'a State,
    usage_scope: Option<&Scope>,
) -> Element<'a, Message> {
    let validity = match account.credential_state {
        AuthCredentialState::Ready => "有效",
        AuthCredentialState::ReauthRequired => "需要重新授权",
        AuthCredentialState::SignedOut => "已登出",
    };
    let status = if account.is_default {
        format!("默认账号 · {validity}")
    } else {
        validity.to_owned()
    };
    let mut entry = column![
        row![
            text(&account.label).size(14).width(Length::Fill).wrapping(text::Wrapping::WordOrGlyph),
            settings_value_badge(status),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    ]
    .spacing(8);
    if state.connection.is_some() {
        entry = entry.push(account_actions(account, state.rows_generation()));
        if account.credential_state == AuthCredentialState::Ready
            && apps_usage_cards::binding_app(service).is_some()
        {
            entry = entry.push(apps_usage_cards::account_view(
                usage_scope,
                &state.usage,
                service,
                &account.account_id,
            ));
        }
    }
    entry.into()
}

fn login_actions(service: AuthService) -> Element<'static, Message> {
    let targets: &[(&str, LoginTarget)] = match service {
        AuthService::GithubCopilot => &[("设备码登录", LoginTarget::Copilot)],
        AuthService::CodexOauth => {
            &[("浏览器登录", LoginTarget::CodexBrowser), ("设备码登录", LoginTarget::Codex)]
        }
        AuthService::XaiOauth => &[("设备码登录", LoginTarget::Xai)],
        AuthService::KimiOauth => &[
            ("中国大陆登录", LoginTarget::Kimi(AuthRegion::MainlandCn)),
            ("国际登录", LoginTarget::Kimi(AuthRegion::Global)),
        ],
        AuthService::ClaudeOfficial => &[("浏览器登录", LoginTarget::Claude)],
        AuthService::AgyOauth => &[("浏览器登录", LoginTarget::Agy)],
        AuthService::DshAccount => &[("浏览器登录", LoginTarget::Dsh)],
    };
    iced::widget::row(targets.iter().map(|&(label, target)| {
        button(text(label).size(13))
            .style(rounded_action_btn_style)
            .padding([6, 12])
            .on_press(message(Event::StartLogin(target)))
            .into()
    }))
    .spacing(8)
    .wrap()
    .into()
}

fn account_actions(account: &AuthAccount, rows_generation: u64) -> Element<'_, Message> {
    use AccountOperation::*;
    let signed_out = account.credential_state == AuthCredentialState::SignedOut;
    let can_offer_default = account.credential_state == AuthCredentialState::Ready
        && !account.is_default
        && !matches!(account.service, AuthService::ClaudeOfficial | AuthService::DshAccount);
    iced::widget::row(
        [
            (SetDefault, can_offer_default),
            (Logout, !signed_out),
            (Revoke, !signed_out),
            (Delete, true),
        ]
        .into_iter()
        .filter(|(_, show)| *show)
        .map(|(operation, _)| {
            button(text(operation.label()).size(13))
                .style(rounded_action_btn_style)
                .padding([6, 10])
                .on_press(message(Event::AccountAction(ActionEvent::Open {
                    rows_generation,
                    operation,
                    service: account.service,
                    account_id: account.account_id.clone(),
                })))
                .into()
        }),
    )
    .spacing(6)
    .wrap()
    .into()
}

pub(crate) fn overlay<'a>(state: &'a State, base: Element<'a, Message>) -> Element<'a, Message> {
    if let Some(action) = &state.account_action {
        return account_action_modal::overlay(action, base);
    }
    if let Some(login) = &state.login {
        return login_modal::overlay(login, base);
    }
    let Some(form) = &state.form else {
        return if let Some(binding) = &state.binding {
            binding_modal::overlay(binding, base)
        } else {
            base
        };
    };
    let pinned = state.binding.is_some();
    let host = settings_text_input("网关登记的主机标识", &form.host)
        .on_input_maybe((!pinned).then_some(|value| message(Event::Host(value))));
    let token = settings_text_input("具备所需授权权限的主机主体令牌", &form.token)
        .secure(true)
        .on_input_maybe((!pinned).then_some(|value| message(Event::Token(value))));
    let close = message(Event::Close);
    let Some(scope) = &state.scope else { return base };
    let body = column![
        text(format!("{} · 公司 {}", scope.gateway, scope.tenant.company_id().as_ref()))
            .size(13)
            .style(settings_muted_text_style),
        text("凭据仅在当前网关和公司范围内保留于本次桌面会话。")
            .size(13)
            .style(settings_muted_text_style),
        settings_field_label("执行主机"),
        host,
        settings_field_label("管理用户名"),
        settings_text_input("管理用户名", &form.username)
            .on_input(|value| message(Event::Username(value))),
        settings_field_label("管理密码"),
        settings_text_input("管理密码", &form.password)
            .secure(true)
            .on_input(|value| message(Event::Password(value))),
        settings_field_label("主体令牌"),
        token,
    ]
    .spacing(8);
    let footer = row![
        button(text("取消"))
            .padding([8, 14])
            .style(rounded_action_btn_style)
            .on_press(close.clone()),
        button(text("读取账号"))
            .padding([8, 14])
            .style(primary_action_btn_style)
            .on_press_maybe(form.valid_management().then(|| message(Event::Connect))),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    let card = settings_modal_layout(
        settings_modal_header(text("管理连接").size(18), settings_close_button(close.clone())),
        body,
        Some(container(footer).align_right(Length::Fill).into()),
    )
    .max_width(500)
    .max_height(450)
    .width(Length::Fill)
    .height(Length::Fill);
    settings_modal_overlay_with_escape(base, close, card)
}
