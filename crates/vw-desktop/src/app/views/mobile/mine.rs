//! ShiJu 账号页复用真实账户仓库与登录操作。
use crate::app::{App, Message};
use iced::widget::{column, container, row, scrollable, svg, text};
use iced::{Element, Length};

pub(super) fn person_icon() -> svg::Handle {
    svg::Handle::from_memory(br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="1.8" stroke-linecap="round"><circle cx="12" cy="7" r="4"/><path d="M4 21v-2a8 8 0 0 1 16 0v2z"/></svg>"#)
}

pub fn view(app: &App) -> Element<'_, Message> {
    let account = app.account.active_account();
    let name = account
        .map(|a| {
            a.profile()
                .map(|p| p.name().to_string())
                .unwrap_or_else(|| a.identity().company_name().to_string())
        })
        .unwrap_or_else(|| "登录 ShiJu".to_string());
    let company =
        account.map(|a| a.identity().company_name()).unwrap_or("登录后管理你的 ShiJu 账号");
    let avatar = container(
        svg(person_icon())
            .width(28)
            .height(28)
            .style(|theme: &iced::Theme, _| svg::Style { color: Some(theme.palette().primary) }),
    )
    .center_x(52)
    .center_y(52)
    .style(|theme: &iced::Theme| container::Style {
        background: Some(theme.palette().primary.scale_alpha(0.12).into()),
        border: iced::Border { radius: 26.0.into(), ..Default::default() },
        ..Default::default()
    });
    let identity = container(
        row![
            avatar,
            column![
                text(name).size(20),
                text(company).size(13).style(
                    crate::app::components::system_settings_common::settings_muted_text_style
                )
            ]
            .spacing(6)
            .width(Length::Fill)
        ]
        .spacing(12)
        .align_y(iced::Alignment::Center),
    )
    .padding(16)
    .width(Length::Fill)
    .style(|theme: &iced::Theme| container::Style {
        background: Some(theme.extended_palette().background.weak.color.into()),
        border: iced::Border { radius: 16.0.into(), ..Default::default() },
        ..Default::default()
    });
    let actions =
        desktop_account_ui::view::inline_menu(&app.account, app.general_settings.desktop_locale)
            .map(crate::app::components::account_status::map);
    scrollable(
        container(column![identity, actions].spacing(16).width(Length::Fill))
            .padding([8, 7])
            .width(Length::Fill),
    )
    .direction(scrollable::Direction::Vertical(
        scrollable::Scrollbar::new().width(4).scroller_width(4),
    ))
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
