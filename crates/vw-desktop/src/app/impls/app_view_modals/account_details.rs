//! 独立展示当前登录账户的公司与个人基础信息。

use super::{App, Message};
use crate::app::components::system_settings_common::{
    primary_action_btn_style, rounded_action_btn_style, settings_close_button, settings_divider,
    settings_modal_card, settings_modal_overlay, settings_muted_text_style,
    settings_text_input_style,
};
use crate::app::account::{AccountPasswordField, AccountPasswordInput};
use crate::app::message::AccountMessage;
use crate::app::state::{AccountPasswordEditorState, AccountProfileEditorState};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::text::Wrapping;
use iced::widget::{Space, button, column, container, row, scrollable, text, text_input};
use iced::{Alignment, Background, Border, Element, Length, Theme};

const DETAILS_SCROLLBAR_WIDTH: u32 = 4;
const PROFILE_EDITOR_SCROLLBAR_WIDTH: u32 = 4;
const PROFILE_EDITOR_LABELS: [&str; 5] =
    ["编辑我的资料", "返回我的", "显示名称", "登录账号", "保存资料"];
const PASSWORD_EDITOR_SCROLLBAR_WIDTH: u32 = 4;
const PASSWORD_INPUTS_SECURE: bool = true;
const PASSWORD_EDITOR_LABELS: [&str; 6] = [
    "修改我的密码",
    "返回我的",
    "当前密码",
    "新密码",
    "确认新密码",
    "修改密码",
];

fn close_message() -> Message {
    Message::Account(AccountMessage::CloseDetails)
}

fn account_message(message: AccountMessage) -> Message {
    Message::Account(message)
}

fn profile_input_style(
    theme: &Theme,
    status: iced::widget::text_input::Status,
) -> iced::widget::text_input::Style {
    settings_text_input_style(theme, status)
}

fn profile_save_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    primary_action_btn_style(theme, status)
}

fn profile_editor_can_submit(state: &AccountProfileEditorState) -> bool {
    matches!(state, AccountProfileEditorState::Editing(draft) if draft.update_request().is_ok())
}

fn password_input_style(
    theme: &Theme,
    status: iced::widget::text_input::Status,
) -> iced::widget::text_input::Style {
    settings_text_input_style(theme, status)
}

fn password_submit_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    primary_action_btn_style(theme, status)
}

fn password_error_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    iced::widget::container::Style {
        background: Some(Background::Color(palette.danger.weak.color.scale_alpha(0.22))),
        text_color: Some(palette.danger.strong.color),
        border: Border {
            color: palette.danger.base.color.scale_alpha(0.7),
            width: 1.0,
            radius: 10.0.into(),
        },
        ..Default::default()
    }
}

fn password_editor_can_submit(state: &AccountPasswordEditorState) -> bool {
    matches!(
        state,
        AccountPasswordEditorState::Editing(_) | AccountPasswordEditorState::Failed { .. }
    )
}

fn section_card_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    iced::widget::container::Style {
        background: Some(Background::Color(palette.background.weak.color.scale_alpha(0.34))),
        border: Border {
            color: palette.background.strong.color.scale_alpha(0.72),
            width: 1.0,
            radius: 14.0.into(),
        },
        ..Default::default()
    }
}

fn status_badge_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    iced::widget::container::Style {
        background: Some(Background::Color(palette.success.base.color.scale_alpha(0.16))),
        text_color: Some(palette.success.strong.color),
        border: Border { radius: 10.0.into(), ..Default::default() },
        ..Default::default()
    }
}

fn status_label(status: &str) -> String {
    match status.trim().to_ascii_lowercase().as_str() {
        "active" | "enabled" => "启用".to_string(),
        "inactive" | "disabled" => "停用".to_string(),
        _ if status.trim().is_empty() => "—".to_string(),
        _ => status.trim().to_string(),
    }
}

fn values_label(values: &[String]) -> String {
    if values.is_empty() { "—".to_string() } else { values.join("、") }
}

fn detail_row(label: &'static str, value: String) -> Element<'static, Message> {
    row![
        text(label)
            .size(13)
            .width(Length::Fixed(92.0))
            .style(settings_muted_text_style),
        text(value).size(13).width(Length::Fill).wrapping(Wrapping::Word),
    ]
    .spacing(12)
    .align_y(Alignment::Start)
    .width(Length::Fill)
    .into()
}

fn status_row(label: &'static str, status: String) -> Element<'static, Message> {
    row![
        text(label)
            .size(13)
            .width(Length::Fixed(92.0))
            .style(settings_muted_text_style),
        container(text(status_label(&status)).size(12))
            .padding([4, 10])
            .style(status_badge_style),
    ]
    .spacing(12)
    .align_y(Alignment::Center)
    .width(Length::Fill)
    .into()
}

fn section_card(
    title: &'static str,
    rows: Vec<Element<'static, Message>>,
) -> Element<'static, Message> {
    let mut content = column![text(title).size(16), settings_divider()].spacing(12);
    for row in rows {
        content = content.push(row).push(settings_divider());
    }
    container(content)
        .width(Length::FillPortion(1))
        .padding([18, 20])
        .style(section_card_style)
        .into()
}

fn edit_profile_action() -> Element<'static, Message> {
    row![
        Space::new().width(Length::Fill),
        button(text("更新资料").size(13))
            .on_press(account_message(AccountMessage::OpenProfileEditor))
            .padding([8, 14])
            .style(primary_action_btn_style),
        button(text("修改密码").size(13))
            .on_press(account_message(AccountMessage::OpenPasswordEditor))
            .padding([8, 14])
            .style(rounded_action_btn_style),
    ]
    .spacing(10)
    .width(Length::Fill)
    .into()
}

fn profile_editor_body(app: &App) -> Element<'_, Message> {
    let mut header_back =
        button(text("← 返回我的").size(13)).padding([8, 12]).style(rounded_action_btn_style);
    if !app.account.profile_editor.is_busy() {
        header_back = header_back.on_press(account_message(AccountMessage::CloseProfileEditor));
    }
    let header = row![
        column![
            text(PROFILE_EDITOR_LABELS[0]).size(22),
            text("更新当前登录用户的显示名称和已有登录账号。")
                .size(12)
                .style(settings_muted_text_style),
        ]
        .spacing(4)
        .width(Length::Fill),
        header_back,
    ]
    .align_y(Alignment::Start);

    let content: Element<'_, Message> = match &app.account.profile_editor {
        AccountProfileEditorState::Closed => Space::new().into(),
        AccountProfileEditorState::Loading => {
            container(text("正在获取账户资料…").size(14).style(settings_muted_text_style))
                .width(Length::Fill)
                .height(Length::Fixed(220.0))
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into()
        }
        AccountProfileEditorState::Failed(error) => {
            let retry = button(text("重新获取").size(13))
                .on_press(account_message(AccountMessage::OpenProfileEditor))
                .padding([8, 14])
                .style(primary_action_btn_style);
            column![
                text("账户资料获取失败").size(16),
                text(error).size(13).style(settings_muted_text_style),
                retry,
            ]
            .spacing(12)
            .width(Length::Fill)
            .into()
        }
        AccountProfileEditorState::Editing(draft) | AccountProfileEditorState::Saving(draft) => {
            let saving =
                matches!(&app.account.profile_editor, AccountProfileEditorState::Saving(_));
            let mut name = text_input("请输入显示名称", draft.name())
                .padding([10, 12])
                .size(14)
                .style(profile_input_style);
            let mut username = text_input("请输入登录账号", draft.username())
                .padding([10, 12])
                .size(14)
                .style(profile_input_style);
            if !saving {
                name = name
                    .on_input(|value| account_message(AccountMessage::ProfileNameChanged(value)));
                username = username.on_input(|value| {
                    account_message(AccountMessage::ProfileUsernameChanged(value))
                });
            }

            let save_label = if saving { "保存中…" } else { PROFILE_EDITOR_LABELS[4] };
            let mut save = button(text(save_label).size(14))
                .padding([10, 18])
                .style(profile_save_button_style);
            if profile_editor_can_submit(&app.account.profile_editor) {
                save = save.on_press(account_message(AccountMessage::SubmitProfileEditor));
            }

            column![
                row![
                    text(PROFILE_EDITOR_LABELS[2])
                        .size(13)
                        .width(Length::Fixed(130.0))
                        .style(settings_muted_text_style),
                    column![
                        name,
                        text("用于后台页面展示和操作记录识别。")
                            .size(12)
                            .style(settings_muted_text_style),
                    ]
                    .spacing(8)
                    .width(Length::Fill),
                ]
                .spacing(18)
                .align_y(Alignment::Start),
                row![
                    text(PROFILE_EDITOR_LABELS[3])
                        .size(13)
                        .width(Length::Fixed(130.0))
                        .style(settings_muted_text_style),
                    column![
                        username,
                        text("仅已有登录账号的用户可在这里更新账号名。")
                            .size(12)
                            .style(settings_muted_text_style),
                    ]
                    .spacing(8)
                    .width(Length::Fill),
                ]
                .spacing(18)
                .align_y(Alignment::Start),
                row![Space::new().width(Length::Fixed(148.0)), save],
            ]
            .spacing(24)
            .width(Length::Fill)
            .into()
        }
    };

    let form = scrollable(container(content).padding([18, 8]).width(Length::Fill))
        .height(Length::Fixed(390.0))
        .direction(Direction::Vertical(
            Scrollbar::new()
                .width(PROFILE_EDITOR_SCROLLBAR_WIDTH)
                .scroller_width(PROFILE_EDITOR_SCROLLBAR_WIDTH),
        ));
    let card = settings_modal_card(column![header, settings_divider(), form].spacing(16))
        .width(Length::Fill)
        .max_width(960.0);
    settings_modal_overlay(None, close_message(), card)
}

fn password_editor_body(app: &App) -> Element<'_, Message> {
    let saving = app.account.password_editor.is_busy();
    let mut header_back =
        button(text("← 返回我的").size(13)).padding([8, 12]).style(rounded_action_btn_style);
    if !saving {
        header_back = header_back.on_press(account_message(AccountMessage::ClosePasswordEditor));
    }
    let header = row![
        column![
            text(PASSWORD_EDITOR_LABELS[0]).size(22),
            text("使用当前密码设置新的登录密码。")
                .size(12)
                .style(settings_muted_text_style),
        ]
        .spacing(4)
        .width(Length::Fill),
        header_back,
    ]
    .align_y(Alignment::Start);

    let error: Element<'_, Message> = if let Some(message) = app.account.password_editor.error() {
        container(
            row![
                column![
                    text("操作失败").size(14),
                    text(message).size(12).wrapping(Wrapping::Word),
                ]
                .spacing(4)
                .width(Length::Fill),
                button(text("关闭").size(12))
                    .on_press(account_message(AccountMessage::DismissPasswordError))
                    .padding([6, 10])
                    .style(rounded_action_btn_style),
            ]
            .spacing(12)
            .align_y(Alignment::Start),
        )
        .padding([12, 14])
        .width(Length::Fill)
        .style(password_error_style)
        .into()
    } else {
        Space::new().height(Length::Shrink).into()
    };

    let draft = app.account.password_editor.draft().cloned().unwrap_or_default();
    let mut current = text_input("请输入当前密码", draft.current_password())
        .secure(PASSWORD_INPUTS_SECURE)
        .padding([10, 12])
        .size(14)
        .style(password_input_style);
    let mut new_password = text_input("请输入新密码", draft.new_password())
        .secure(PASSWORD_INPUTS_SECURE)
        .padding([10, 12])
        .size(14)
        .style(password_input_style);
    let mut confirmation = text_input("请再次输入新密码", draft.confirmation())
        .secure(PASSWORD_INPUTS_SECURE)
        .padding([10, 12])
        .size(14)
        .style(password_input_style);
    if !saving {
        current = current.on_input(|value| {
            account_message(AccountMessage::PasswordFieldChanged {
                field: AccountPasswordField::Current,
                value: AccountPasswordInput::new(value),
            })
        });
        new_password = new_password.on_input(|value| {
            account_message(AccountMessage::PasswordFieldChanged {
                field: AccountPasswordField::New,
                value: AccountPasswordInput::new(value),
            })
        });
        confirmation = confirmation.on_input(|value| {
            account_message(AccountMessage::PasswordFieldChanged {
                field: AccountPasswordField::Confirmation,
                value: AccountPasswordInput::new(value),
            })
        });
    }

    let submit_label = if saving { "修改中…" } else { PASSWORD_EDITOR_LABELS[5] };
    let mut submit = button(text(submit_label).size(14))
        .padding([10, 18])
        .style(password_submit_button_style);
    if password_editor_can_submit(&app.account.password_editor) {
        submit = submit.on_press(account_message(AccountMessage::SubmitPasswordEditor));
    }

    let form_content = column![
        error,
        row![
            text(PASSWORD_EDITOR_LABELS[2])
                .size(13)
                .width(Length::Fixed(130.0))
                .style(settings_muted_text_style),
            current,
        ]
        .spacing(18)
        .align_y(Alignment::Center),
        row![
            text(PASSWORD_EDITOR_LABELS[3])
                .size(13)
                .width(Length::Fixed(130.0))
                .style(settings_muted_text_style),
            column![
                new_password,
                text("密码需为 8–64 个字符，并同时包含字母和数字。")
                    .size(12)
                    .style(settings_muted_text_style),
            ]
            .spacing(8)
            .width(Length::Fill),
        ]
        .spacing(18)
        .align_y(Alignment::Start),
        row![
            text(PASSWORD_EDITOR_LABELS[4])
                .size(13)
                .width(Length::Fixed(130.0))
                .style(settings_muted_text_style),
            confirmation,
        ]
        .spacing(18)
        .align_y(Alignment::Center),
        row![Space::new().width(Length::Fixed(148.0)), submit],
    ]
    .spacing(24)
    .width(Length::Fill);

    let form = scrollable(container(form_content).padding([18, 8]).width(Length::Fill))
        .height(Length::Fixed(430.0))
        .direction(Direction::Vertical(
            Scrollbar::new()
                .width(PASSWORD_EDITOR_SCROLLBAR_WIDTH)
                .scroller_width(PASSWORD_EDITOR_SCROLLBAR_WIDTH),
        ));
    let card = settings_modal_card(column![header, settings_divider(), form].spacing(16))
        .width(Length::Fill)
        .max_width(960.0);
    settings_modal_overlay(None, close_message(), card)
}

pub(crate) fn with_account_details<'a>(
    app: &'a App,
    root_content: Element<'a, Message>,
) -> Element<'a, Message> {
    if !app.account.details_open {
        return root_content;
    }
    let Some(account) = app.account.active_account() else {
        return root_content;
    };
    if !matches!(&app.account.profile_editor, AccountProfileEditorState::Closed) {
        let editor = profile_editor_body(app);
        return iced::widget::stack![root_content, editor].into();
    }
    if !matches!(&app.account.password_editor, AccountPasswordEditorState::Closed) {
        let editor = password_editor_body(app);
        return iced::widget::stack![root_content, editor].into();
    }

    let identity = account.identity();
    let company_status = account
        .profile()
        .map(|profile| profile.company_status())
        .unwrap_or_else(|| identity.status())
        .to_string();
    let company_card = section_card(
        "公司信息",
        vec![
            detail_row("公司名称", identity.company_name().to_string()),
            status_row("状态", company_status),
        ],
    );

    let user_card = if let Some(profile) = account.profile() {
        section_card(
            "我的信息",
            vec![
                detail_row("显示名称", profile.name().to_string()),
                detail_row("登录账号", profile.account_name().to_string()),
                status_row("状态", profile.user_status().to_string()),
                detail_row("直属部门", values_label(profile.departments())),
                detail_row("直属角色", values_label(profile.roles())),
                edit_profile_action(),
            ],
        )
    } else {
        section_card(
            "我的信息",
            vec![detail_row("资料状态", "旧账户尚未获取个人资料，请重新登录刷新。".to_string())],
        )
    };

    let details = scrollable(
        row![company_card, user_card]
            .spacing(16)
            .align_y(Alignment::Start)
            .width(Length::Fill),
    )
    .height(Length::Fixed(430.0))
    .direction(Direction::Vertical(
        Scrollbar::new()
            .width(DETAILS_SCROLLBAR_WIDTH)
            .scroller_width(DETAILS_SCROLLBAR_WIDTH),
    ));
    let close = close_message();
    let card = settings_modal_card(
        column![
            row![
                column![
                    text("账户信息").size(22),
                    text("当前登录账户的公司与个人基础资料")
                        .size(12)
                        .style(settings_muted_text_style),
                ]
                .spacing(4)
                .width(Length::Fill),
                settings_close_button(close.clone()),
            ]
            .align_y(Alignment::Start),
            details,
        ]
        .spacing(16),
    )
    .width(Length::Fill)
    .max_width(960.0);

    settings_modal_overlay(Some(root_content), close, card)
}

#[cfg(test)]
#[path = "account_details_tests.rs"]
mod account_details_tests;
