//! Scoped authorization center and temporary management connection form.

mod login_modal;
#[cfg(test)]
pub(in crate::app) mod login_modal_tests;

use super::system_settings_common::{
    primary_action_btn_style, rounded_action_btn_style, settings_checkbox_style,
    settings_close_button, settings_error_banner, settings_field_label, settings_group_card,
    settings_modal_header, settings_modal_layout, settings_modal_overlay_with_escape,
    settings_muted_text_style, settings_page_heading, settings_text_input,
};
use crate::app::{
    Message,
    message::agent_apps_auth::{Event, message},
    state::agent_apps_auth::{LoginTarget, ServiceAccounts, State},
};
use iced::widget::{button, checkbox, column, container, row, scrollable, text};
use iced::{Alignment, Element, Length};
use vw_gateway_client::vw_api_types::agent_apps_auth::{
    AuthCredentialState, AuthRegion, AuthService,
};

pub(crate) fn view(state: &State) -> Element<'_, Message> {
    let configured = state.connection.is_some();
    let refresh_label =
        if state.rows.as_ref().is_some_and(|rows| rows.iter().any(|row| row.accounts.is_err())) {
            "重试"
        } else {
            "刷新"
        };
    let actions = row![
        button(text("管理连接")).style(rounded_action_btn_style).padding([6, 12]).on_press_maybe(
            (state.loopback && state.pending.is_none()).then(|| message(Event::Open))
        ),
        button(text(refresh_label))
            .style(rounded_action_btn_style)
            .padding([6, 12])
            .on_press_maybe(
                (configured && state.pending.is_none()).then(|| message(Event::Refresh))
            ),
    ]
    .spacing(8);

    let mut heading = column![
        settings_page_heading("授权中心", "查看当前公司在授权执行主机上的账号状态。"),
        checkbox(!state.copy_login_link)
            .label("登录时自动打开浏览器")
            .on_toggle(|open| message(Event::OpenBrowserPreference(open)))
            .style(settings_checkbox_style),
        text("关闭后复制登录链接，可在自己选择的浏览器中打开。")
            .size(13)
            .style(settings_muted_text_style),
        actions,
    ]
    .spacing(10);
    if let Some(scope) = &state.scope {
        heading = heading.push(
            text(format!(
                "平台 {} · 公司 {}",
                scope.tenant.platform_id().as_ref(),
                scope.tenant.company_id().as_ref()
            ))
            .size(12)
            .style(settings_muted_text_style),
        );
    }
    if let Some(connection) = &state.connection {
        heading = heading.push(
            text(format!("执行主机 {}", connection.host.trim()))
                .size(12)
                .style(settings_muted_text_style),
        );
    }

    let body: Element<'_, Message> = if !state.loopback {
        settings_error_banner("授权中心需要本机回环管理连接。请在网关设置中选择本机连接。")
    } else if state.pending.is_some() {
        text("正在读取授权账号…").style(settings_muted_text_style).into()
    } else if let Some(rows) = &state.rows {
        scrollable(
            container(column(rows.iter().map(|row| service_card(row, configured))).spacing(10))
                .padding([0, 7])
                .width(Length::Fill),
        )
        .direction(scrollable::Direction::Vertical(
            scrollable::Scrollbar::new().width(4).scroller_width(4),
        ))
        .height(Length::Fill)
        .into()
    } else {
        text("请配置管理连接，填写执行主机及其账号读取令牌。")
            .style(settings_muted_text_style)
            .into()
    };

    column![container(heading).padding([0, 7]), body]
        .width(Length::Fill)
        .height(Length::Fill)
        .spacing(12)
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

fn service_card(row: &ServiceAccounts, configured: bool) -> Element<'_, Message> {
    let mut card = column![text(service_name(row.service)).size(17)].spacing(6);
    match &row.accounts {
        Ok(accounts) if accounts.is_empty() => {
            card = card.push(text("暂无授权账号").size(13).style(settings_muted_text_style));
        }
        Ok(accounts) => {
            for account in accounts {
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
                card = card.push(
                    column![
                        text(&account.label).size(14),
                        text(status).size(12).style(settings_muted_text_style),
                    ]
                    .spacing(3),
                );
            }
        }
        Err(reason) => {
            card = card.push(text(*reason).size(13).style(settings_muted_text_style));
        }
    }
    if configured {
        let targets: &[(&str, LoginTarget)] = match row.service {
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
        if !targets.is_empty() {
            card = card.push(
                iced::widget::row(targets.iter().map(|&(label, target)| {
                    button(text(label))
                        .style(rounded_action_btn_style)
                        .padding([6, 12])
                        .on_press(message(Event::StartLogin(target)))
                        .into()
                }))
                .spacing(8)
                .wrap(),
            );
        }
    }
    settings_group_card(container(card).padding([10, 0])).into()
}

pub(crate) fn overlay<'a>(state: &'a State, base: Element<'a, Message>) -> Element<'a, Message> {
    if let Some(login) = &state.login {
        return login_modal::overlay(login, base);
    }
    let Some(form) = &state.form else { return base };
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
        settings_text_input("网关登记的主机标识", &form.host)
            .on_input(|value| message(Event::Host(value))),
        settings_field_label("管理用户名"),
        settings_text_input("管理用户名", &form.username)
            .on_input(|value| message(Event::Username(value))),
        settings_field_label("管理密码"),
        settings_text_input("管理密码", &form.password)
            .secure(true)
            .on_input(|value| message(Event::Password(value))),
        settings_field_label("主体令牌"),
        settings_text_input("具备所需授权权限的主机主体令牌", &form.token)
            .secure(true)
            .on_input(|value| message(Event::Token(value))),
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
            .on_press_maybe(form.valid().then(|| message(Event::Connect))),
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
