use crate::app::components::overlays::AboveOverlay;
use crate::app::message::{AccountMessage, SkillHubMessage};
use crate::app::{App, Message};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{button, column, container, row, scrollable, text, text_input};
use iced::{Background, Border, Color, Element, Length, Shadow, Theme};

pub(crate) const ACCOUNT_SCROLLBAR_WIDTH: u32 = 4;

fn account_message(message: AccountMessage) -> Message {
    Message::Account(message)
}

fn install_guide_message() -> Message {
    Message::SkillHub(SkillHubMessage::OpenInstallGuide)
}

pub(crate) fn trigger_label(app: &App) -> String {
    if let Some(label) = app.account.operation.label() {
        return label.to_string();
    }
    app.account
        .active_account()
        .map(|account| account.identity().company_name().to_string())
        .unwrap_or_else(|| "登录账号".to_string())
}

fn account_label(app: &App, index: usize) -> String {
    let account = &app.account.accounts()[index];
    let company_name = account.identity().company_name();
    let matching = app
        .account
        .accounts()
        .iter()
        .enumerate()
        .filter(|(_, candidate)| candidate.identity().company_name() == company_name)
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    if matching.len() <= 1 {
        company_name.to_string()
    } else {
        let position = matching.iter().position(|candidate| *candidate == index).unwrap_or(0) + 1;
        format!("{company_name} · 账户 {position}")
    }
}

fn menu_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    iced::widget::container::Style {
        background: Some(Background::Color(palette.background.base.color)),
        text_color: Some(palette.background.base.text),
        border: Border { color: palette.background.strong.color, width: 1.0, radius: 12.0.into() },
        shadow: Shadow {
            color: Color::BLACK.scale_alpha(0.26),
            offset: iced::Vector::new(0.0, 5.0),
            blur_radius: 18.0,
        },
        ..Default::default()
    }
}

fn trigger_style(
    theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let palette = theme.extended_palette();
    let background = match status {
        iced::widget::button::Status::Hovered => palette.background.strong.color,
        _ => palette.background.weak.color,
    };
    iced::widget::button::Style {
        background: Some(Background::Color(background)),
        text_color: palette.background.base.text,
        border: Border { color: palette.background.strong.color, width: 1.0, radius: 9.0.into() },
        ..Default::default()
    }
}

fn account_row_style(
    theme: &Theme,
    status: iced::widget::button::Status,
    active: bool,
) -> iced::widget::button::Style {
    let palette = theme.extended_palette();
    let background = if active {
        Some(Background::Color(palette.primary.weak.color))
    } else if matches!(status, iced::widget::button::Status::Hovered) {
        Some(Background::Color(palette.background.weak.color))
    } else {
        None
    };
    iced::widget::button::Style {
        background,
        text_color: if active { palette.primary.weak.text } else { palette.background.base.text },
        border: Border { radius: 7.0.into(), ..Default::default() },
        ..Default::default()
    }
}

fn login_input_style(
    theme: &Theme,
    status: iced::widget::text_input::Status,
) -> iced::widget::text_input::Style {
    let mut style =
        crate::app::components::system_settings_common::settings_text_input_style(theme, status);
    style.border.radius = 10.0.into();
    style
}

fn login_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let palette = theme.extended_palette();
    let surface = if palette.is_dark {
        palette.background.weak.color
    } else {
        crate::app::theme_catalog::panel_surface_color(theme, 1.0)
    };
    let background = match status {
        iced::widget::button::Status::Hovered => {
            if palette.is_dark {
                palette.background.strong.color
            } else {
                Color {
                    r: surface.r + (palette.primary.weak.color.r - surface.r) * 0.08,
                    g: surface.g + (palette.primary.weak.color.g - surface.g) * 0.08,
                    b: surface.b + (palette.primary.weak.color.b - surface.b) * 0.08,
                    a: surface.a,
                }
            }
        }
        iced::widget::button::Status::Pressed => palette.primary.weak.color,
        _ => surface,
    };
    let text_color = if matches!(status, iced::widget::button::Status::Disabled) {
        theme.palette().text.scale_alpha(0.42)
    } else if matches!(status, iced::widget::button::Status::Pressed) {
        palette.primary.weak.text
    } else {
        theme.palette().text
    };

    iced::widget::button::Style {
        background: Some(Background::Color(background)),
        text_color,
        border: Border {
            color: palette.background.strong.color.scale_alpha(if palette.is_dark {
                0.78
            } else {
                0.42
            }),
            width: 1.0,
            radius: 10.0.into(),
        },
        shadow: if palette.is_dark {
            Shadow::default()
        } else {
            Shadow {
                color: Color::BLACK.scale_alpha(0.035),
                offset: iced::Vector::new(0.0, 3.0),
                blur_radius: 9.0,
            }
        },
        ..Default::default()
    }
}

fn delete_account_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let palette = theme.extended_palette();
    let background = match status {
        iced::widget::button::Status::Hovered => Some(Background::Color(palette.danger.weak.color)),
        iced::widget::button::Status::Pressed => {
            Some(Background::Color(palette.danger.base.color.scale_alpha(0.20)))
        }
        _ => None,
    };
    iced::widget::button::Style {
        background,
        text_color: palette.danger.base.color,
        border: Border {
            color: palette.danger.base.color.scale_alpha(0.36),
            width: 1.0,
            radius: 8.0.into(),
        },
        ..Default::default()
    }
}

fn menu(app: &App) -> Element<'_, Message> {
    let busy = app.account.operation.is_busy();
    let active_id = app.account.repository().active_account_id();
    let mut account_rows = column![].spacing(3).width(Length::Fill);
    if app.account.accounts().is_empty() {
        account_rows = account_rows.push(
            container(text("暂无本地账户").size(12).style(|theme: &Theme| {
                iced::widget::text::Style { color: Some(theme.palette().text.scale_alpha(0.58)) }
            }))
            .padding([8, 10]),
        );
    } else {
        for (index, account) in app.account.accounts().iter().enumerate() {
            let active = active_id == Some(account.id());
            let marker = if active { "✓" } else { "" };
            let content = row![
                text(account_label(app, index)).size(13).width(Length::Fill),
                text(marker).size(13),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center);
            let mut item = button(content)
                .width(Length::Fill)
                .padding([8, 10])
                .style(move |theme, status| account_row_style(theme, status, active));
            if !busy && !active {
                item = item.on_press(account_message(AccountMessage::SelectSaved(
                    account.id().to_string(),
                )));
            }
            let mut delete =
                button(text("删除").size(12)).padding([8, 10]).style(delete_account_button_style);
            if !busy {
                delete = delete.on_press(account_message(AccountMessage::DeleteSaved(
                    account.id().to_string(),
                )));
            }
            account_rows = account_rows.push(
                row![item, delete].spacing(4).align_y(iced::Alignment::Center).width(Length::Fill),
            );
        }
    }

    let accounts =
        scrollable(account_rows).height(Length::Fixed(148.0)).direction(Direction::Vertical(
            Scrollbar::new().width(ACCOUNT_SCROLLBAR_WIDTH).scroller_width(ACCOUNT_SCROLLBAR_WIDTH),
        ));
    let input = text_input("手工输入 skey", app.account.manual_input.expose())
        .secure(true)
        .on_input(|value| {
            account_message(AccountMessage::ManualInputChanged(
                crate::app::account::ManualSkeyInput::new(value),
            ))
        })
        .on_submit(account_message(AccountMessage::SubmitManual))
        .padding([7, 9])
        .size(12)
        .style(login_input_style);
    let mut confirm = button(text("确认").size(12)).padding([7, 12]).style(login_button_style);
    if !busy && !app.account.manual_input.expose().trim().is_empty() {
        confirm = confirm.on_press(account_message(AccountMessage::SubmitManual));
    }
    let waiting_for_authorization = matches!(
        &app.account.operation,
        crate::app::state::AccountOperation::WaitingAuthorization { .. }
    );
    let browser_label = if waiting_for_authorization {
        "取消等待"
    } else if app.account.active_account().is_some() {
        "切换账号登录"
    } else {
        "登录"
    };
    let mut browser = button(text(browser_label).size(13))
        .width(Length::Fill)
        .padding([9, 10])
        .style(login_button_style);
    if waiting_for_authorization {
        browser = browser.on_press(account_message(AccountMessage::CancelBrowserLogin));
    } else if !busy {
        browser = browser.on_press(account_message(AccountMessage::StartBrowserLogin));
    }

    let mut content = column![
        text(if active_id.is_some() { "账户" } else { "登录账户系统" }).size(14),
        accounts,
        row![input, confirm].spacing(6).align_y(iced::Alignment::Center),
        browser,
    ]
    .spacing(8)
    .width(Length::Fill);
    if active_id.is_some() {
        let mut details = button(text("账户信息").size(13))
            .width(Length::Fill)
            .padding([9, 10])
            .style(login_button_style);
        if !busy {
            details = details.on_press(account_message(AccountMessage::OpenDetails));
        }
        let mut sign_out = button(text("退出登录").size(13))
            .width(Length::Fill)
            .padding([9, 10])
            .style(login_button_style);
        let mut install_guide = button(text("安装识矩技能").size(13))
            .width(Length::Fill)
            .padding([9, 10])
            .style(login_button_style);
        if !busy {
            install_guide = install_guide.on_press(install_guide_message());
            sign_out = sign_out.on_press(account_message(AccountMessage::SignOut));
        }
        content = content.push(details);
        content = content.push(install_guide);
        content = content.push(sign_out);
    }

    container(content).width(Length::Fixed(320.0)).padding(12).style(menu_style).into()
}

pub(crate) fn view(app: &App) -> Element<'_, Message> {
    let trigger = button(
        row![text("◉").size(12), text(trigger_label(app)).size(12), text("⌃").size(10),]
            .spacing(6)
            .align_y(iced::Alignment::Center),
    )
    .on_press(account_message(AccountMessage::ToggleMenu))
    .padding([5, 9])
    .style(trigger_style);

    AboveOverlay::new(trigger, menu(app))
        .show(app.account.menu_open)
        .gap(5.0)
        .on_close(account_message(AccountMessage::CloseMenu))
        .block_background_events(true)
        .into()
}

#[cfg(test)]
#[path = "account_status_tests.rs"]
mod account_status_tests;
