//! Scoped authorization center and temporary management connection form.

mod account_action_modal;
mod binding_modal;

mod login_modal;

use crate::{ActionEvent, Event, State, Usage, types::*};
use desktop_assets::assets::Icon;
use desktop_ui_controls::system_settings_common::*;

fn message<U>(event: Event<U>) -> Event<U> {
    event
}
use iced::widget::{button, column, container, responsive, row, scrollable, svg, text};
use iced::{Alignment, Element, Font, Length, Theme, font};
use vw_api_app_auth::{AuthAccount, AuthCredentialState, AuthRegion, AuthService};

pub fn view<'a, U: Clone + 'static>(
    state: &State<'a>,
    usage: Usage<'a, U>,
) -> Element<'a, Event<U>> {
    if let Some(binding) = state.binding {
        return container(settings_page_heading(
            service_name(binding.service),
            "在授权中心为供应商选择当前执行主机上的授权账号。",
        ))
        .padding([0, 7])
        .width(Length::Fill)
        .height(Length::Fill)
        .into();
    }
    let failed = state.connection_error.is_some()
        || state.rows.as_ref().is_some_and(|rows| rows.iter().all(|row| row.accounts.is_err()));
    let idle = state.pending.is_none() && state.account_action.is_none();
    let batch_pending = usage.batch_pending;
    let can_refresh_usage = idle
        && state.connection.is_some()
        && state.rows.is_some()
        && usage.available
        && !batch_pending;
    let header = responsive(move |size| {
        let heading = row![
            container(action_icon(Icon::ShieldLock, 20.0))
                .center_x(40)
                .center_y(40)
                .style(settings_icon_tile_style),
            column![
                title("授权中心", 24),
                text("管理当前公司的登录账号").size(13).style(settings_muted_text_style),
            ]
            .spacing(3),
        ]
        .spacing(12)
        .align_y(Alignment::Center);
        let actions = toolbar(failed, idle, can_refresh_usage, batch_pending);
        if size.width >= 530.0 {
            row![heading.width(Length::Fill), actions].spacing(12).align_y(Alignment::Center).into()
        } else {
            column![heading, actions].spacing(10).into()
        }
    })
    .height(Length::Shrink);
    let content =
        column![login_preferences(state), services(state, usage)].spacing(16).width(Length::Fill);
    let body = scrollable(
        container(content).padding(iced::Padding::new(7.0).top(2).bottom(10)).width(Length::Fill),
    )
    .direction(scrollable::Direction::Vertical(
        scrollable::Scrollbar::new().width(4).scroller_width(4),
    ))
    .height(Length::Fill);
    column![container(header).padding([0, 7]).width(Length::Fill), settings_divider(), body,]
        .spacing(12)
        .height(Length::Fill)
        .into()
}

fn title(label: impl Into<String>, size: u32) -> iced::widget::Text<'static> {
    text(label.into()).size(size).font(Font { weight: font::Weight::Semibold, ..Font::DEFAULT })
}

fn action_icon(icon: Icon, size: f32) -> iced::widget::Svg<'static> {
    icon_svg(icon, size).style(icons::icon_btn_svg_style)
}

fn toolbar<U: Clone + 'static>(
    failed: bool,
    idle: bool,
    can_refresh_usage: bool,
    batch_pending: bool,
) -> Element<'static, Event<U>> {
    let refresh_label = if failed { "重试读取账号" } else { "刷新账号" };
    row![
        button(
            row![action_icon(Icon::ArrowClockwise, 14.0), text(refresh_label).size(13)]
                .spacing(6)
                .align_y(Alignment::Center)
        )
        .style(rounded_action_btn_style)
        .padding([8, 12])
        .on_press_maybe(idle.then_some(Event::Refresh)),
        button(text(if batch_pending { "正在刷新额度…" } else { "刷新全部额度" }).size(13))
            .style(rounded_action_btn_style)
            .padding([8, 12])
            .on_press_maybe(can_refresh_usage.then_some(Event::RefreshUsage)),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .wrap()
    .into()
}

fn login_preferences<'a, U: Clone + 'static>(state: &State<'a>) -> Element<'a, Event<U>> {
    let mut context = row![].spacing(12).align_y(Alignment::Center);
    if let Some(scope) = state.scope {
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
    if let Some(connection) = state.connection {
        let host = if connection.host == local_app_auth::HOST_ID {
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
    let labels = column![
        title("登录时自动打开浏览器", 15),
        text("关闭后会复制登录链接，可用自己选择的浏览器完成授权。")
            .size(13)
            .style(settings_muted_text_style),
    ]
    .spacing(4)
    .width(Length::Fill);
    let toggle = settings_toggle(!state.copy_login_link).on_toggle(Event::OpenBrowserPreference);
    let mut preferences =
        column![row![labels, toggle].spacing(14).align_y(Alignment::Center),].spacing(10);
    if state.scope.is_some() || state.connection.is_some() {
        preferences = preferences.push(settings_divider()).push(context.wrap());
    }
    container(preferences).padding([13, 16]).width(Length::Fill).style(auth_panel_style).into()
}

fn auth_panel_style(theme: &Theme) -> container::Style {
    let mut style = settings_panel_style(theme);
    style.border.radius = 12.0.into();
    style.shadow = iced::Shadow::default();
    style
}

fn services<'a, U: Clone + 'static>(
    state: &State<'a>,
    mut usage: Usage<'a, U>,
) -> Element<'a, Event<U>> {
    if state.pending.is_some() {
        return list_placeholder(Icon::Clock, "正在读取授权账号…");
    }
    if let Some(error) = state.connection_error {
        return settings_error_banner(error.as_str());
    }
    let Some(rows) = state.rows else {
        return if cfg!(unix) {
            list_placeholder(Icon::Clock, "正在准备本机授权…")
        } else {
            list_placeholder(Icon::ShieldLock, "请刷新以读取本机账号。")
        };
    };

    let heading =
        row![title("授权服务", 16).width(Length::Fill), settings_value_badge(rows.len()),]
            .align_y(Alignment::Center);
    let mut list = column![heading].spacing(10).width(Length::Fill);
    let report = std::mem::take(&mut usage.lines);
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
        list = list.push(service_card(service, state, &mut usage));
    }
    list.into()
}

fn list_placeholder<U: Clone + 'static>(
    icon: Icon,
    label: &'static str,
) -> Element<'static, Event<U>> {
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

fn service_card<'a, U: Clone + 'static>(
    service: &'a ServiceAccounts,
    state: &State<'a>,
    usage: &mut Usage<'a, U>,
) -> Element<'a, Event<U>> {
    let heading = if state.connection.is_some() {
        responsive(move |size| {
            let summary = service_summary(service);
            let actions = login_actions(service.service);
            if size.width >= 490.0 {
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

    let mut card = column![container(heading).padding([10, 16]).width(Length::Fill)];
    match &service.accounts {
        Ok(accounts) if accounts.is_empty() => {
            card = card.push(settings_divider()).push(
                container(text("尚未添加账号").size(13).style(settings_muted_text_style))
                    .padding([11, 16]),
            );
        }
        Ok(accounts) => {
            for account in accounts {
                card = card.push(settings_divider()).push(
                    container(account_row(account, service.service, state, usage))
                        .padding([12, 16])
                        .width(Length::Fill),
                );
            }
        }
        Err(reason) => {
            let retry = button(text("重试此服务").size(13))
                .style(rounded_action_btn_style)
                .padding([7, 12])
                .on_press_maybe(
                    (state.connection.is_some()
                        && state.pending.is_none()
                        && state.login.is_none()
                        && state.account_action.is_none()
                        && state.form.is_none())
                    .then_some(Event::RetryService(service.service)),
                );
            card = card.push(settings_divider()).push(
                container(column![settings_error_banner(*reason), retry].spacing(8))
                    .padding([12, 16])
                    .width(Length::Fill),
            );
        }
    }

    let focused = Some(service.service) == state.focused_service;
    container(card)
        .width(Length::Fill)
        .style(move |theme| {
            let mut style = auth_panel_style(theme);
            if focused {
                style.border.color = theme.palette().primary;
            }
            style
        })
        .into()
}

fn service_summary<U: Clone + 'static>(service: &ServiceAccounts) -> Element<'static, Event<U>> {
    let logo = match service.service {
        AuthService::ClaudeOfficial => provider_logo_svg("anthropic", 21.0),
        AuthService::CodexOauth => provider_logo_svg("openai", 21.0),
        AuthService::GithubCopilot => provider_logo_svg("github-copilot", 21.0),
        AuthService::XaiOauth => provider_logo_svg("xai", 21.0),
        AuthService::KimiOauth => provider_logo_svg("kimi-for-coding", 21.0),
        AuthService::AgyOauth => icon_svg(Icon::AppAntigravity, 21.0),
        AuthService::DshAccount => provider_logo_svg("deepseek", 21.0),
    };
    let help_text = match service.service {
        AuthService::ClaudeOfficial => {
            "在浏览器中登录 Claude。需要手动完成时，可回填授权码或回调地址。"
        }
        AuthService::CodexOauth => "可直接在浏览器登录，也可在验证页输入设备码完成授权。",
        AuthService::GithubCopilot => {
            "在 GitHub 验证页输入设备码，登录具有 Copilot 访问权限的账号。"
        }
        AuthService::XaiOauth => "在 xAI 验证页输入设备码，授权当前执行主机。",
        AuthService::KimiOauth => "请选择与账号匹配的登录地区，再在验证页输入设备码。",
        AuthService::AgyOauth => "前往浏览器完成 Antigravity 登录和授权。",
        AuthService::DshAccount => "在浏览器中登录 DeepSeek 账号并完成授权。",
    };
    let name = help::settings_hover_help_label(
        title(service_name(service.service), 16),
        column![
            title(format!("{} 登录方式", service_name(service.service)), 14),
            text(help_text).size(13).style(settings_muted_text_style),
        ]
        .spacing(5)
        .into(),
    );
    row![
        container(logo).center_x(34).center_y(34).style(settings_icon_tile_style),
        name.width(Length::Fill),
    ]
    .align_y(Alignment::Center)
    .spacing(12)
    .width(Length::Fill)
    .into()
}

fn account_row<'a, U: Clone + 'static>(
    account: &'a AuthAccount,
    service: AuthService,
    state: &State<'a>,
    usage: &mut Usage<'a, U>,
) -> Element<'a, Event<U>> {
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
        entry = entry.push(account_actions(account, state.rows_generation));
        if let Some(card) = usage.accounts.remove(&(service, account.account_id.clone())) {
            entry = entry.push(card);
        }
    }
    entry.into()
}

fn login_actions<U: Clone + 'static>(service: AuthService) -> Element<'static, Event<U>> {
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
        let label = if targets.len() == 1 { "添加账号" } else { label };
        button(
            row![action_icon(Icon::Plus, 14.0), text(label).size(13)]
                .spacing(6)
                .align_y(Alignment::Center),
        )
        .style(rounded_action_btn_style)
        .padding([7, 12])
        .on_press(Event::StartLogin(target))
        .into()
    }))
    .spacing(8)
    .wrap()
    .into()
}

fn account_actions<U: Clone + 'static>(
    account: &AuthAccount,
    rows_generation: u64,
) -> Element<'_, Event<U>> {
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

pub fn overlay<'a, U: Clone + 'static>(
    state: &State<'a>,
    base: Element<'a, Event<U>>,
) -> Element<'a, Event<U>> {
    if let Some(action) = state.account_action {
        return account_action_modal::overlay(action, base);
    }
    if let Some(login) = state.login {
        return login_modal::overlay(login, base);
    }
    if let Some(binding) = state.binding { binding_modal::overlay(binding, base) } else { base }
}
