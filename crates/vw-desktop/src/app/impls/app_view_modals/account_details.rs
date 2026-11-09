//! 独立展示当前登录账户的公司与个人基础信息。

use super::{App, Message};
use crate::app::account::{AccountPasswordField, AccountPasswordInput};
use crate::app::components::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, primary_action_btn_style,
    rounded_action_btn_style, settings_close_button, settings_divider, settings_error_banner,
    settings_field_label, settings_group_card, settings_modal_header, settings_modal_layout,
    settings_modal_overlay, settings_muted_text_style, settings_text_input_style,
    settings_value_badge,
};
use crate::app::i18n::I18n;
use crate::app::message::AccountMessage;
use crate::app::state::{AccountPasswordEditorState, AccountProfileEditorState};
use iced::widget::text::Wrapping;
use iced::widget::{Space, button, column, container, row, text, text_input};
use iced::{Alignment, Element, Length};

fn close_message() -> Message {
    Message::Account(AccountMessage::CloseDetails)
}

fn account_message(message: AccountMessage) -> Message {
    Message::Account(message)
}

fn profile_editor_can_submit(state: &AccountProfileEditorState) -> bool {
    matches!(state, AccountProfileEditorState::Editing(draft) if draft.update_request().is_ok())
}

fn password_editor_can_submit(state: &AccountPasswordEditorState) -> bool {
    matches!(
        state,
        AccountPasswordEditorState::Editing(_) | AccountPasswordEditorState::Failed { .. }
    )
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

fn detail_row(i18n: I18n, label: &'static str, value: String) -> Element<'static, Message> {
    row![
        text(i18n.__(label)).size(13).width(Length::Fixed(92.0)).style(settings_muted_text_style),
        text(value).size(13).width(Length::Fill).wrapping(Wrapping::WordOrGlyph),
    ]
    .spacing(12)
    .align_y(Alignment::Start)
    .width(Length::Fill)
    .into()
}

fn status_row(i18n: I18n, label: &'static str, status: String) -> Element<'static, Message> {
    row![
        text(i18n.__(label)).size(13).width(Length::Fixed(92.0)).style(settings_muted_text_style),
        settings_value_badge(i18n.__(&status_label(&status))),
    ]
    .spacing(12)
    .align_y(Alignment::Center)
    .width(Length::Fill)
    .into()
}

fn section_card(
    i18n: I18n,
    title: &'static str,
    rows: Vec<Element<'static, Message>>,
) -> Element<'static, Message> {
    let mut content = column![text(i18n.__(title)).size(16), settings_divider()].spacing(12);
    for (index, row) in rows.into_iter().enumerate() {
        if index > 0 {
            content = content.push(settings_divider());
        }
        content = content.push(row);
    }
    settings_group_card(container(content).padding([14, 0])).into()
}

fn edit_profile_action(i18n: I18n) -> Element<'static, Message> {
    row![
        button(text(i18n.__("更新资料")).size(13))
            .on_press(account_message(AccountMessage::OpenProfileEditor))
            .padding([8, 14])
            .style(primary_action_btn_style),
        button(text(i18n.__("修改密码")).size(13))
            .on_press(account_message(AccountMessage::OpenPasswordEditor))
            .padding([8, 14])
            .style(rounded_action_btn_style),
    ]
    .spacing(10)
    .align_y(Alignment::Center)
    .into()
}

fn account_header<'a>(
    title: &'a str,
    description: &'a str,
    action: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    settings_modal_header(
        column![text(title).size(22), text(description).size(13).style(settings_muted_text_style)]
            .spacing(4),
        action,
    )
}

fn editor_back(i18n: I18n, message: AccountMessage, busy: bool) -> Element<'static, Message> {
    button(text(i18n.__("← 返回我的")).size(13))
        .padding([8, 12])
        .style(rounded_action_btn_style)
        .on_press_maybe((!busy).then(|| account_message(message)))
        .into()
}

fn editor_input<'a>(placeholder: &str, value: &str) -> text_input::TextInput<'a, Message> {
    text_input(placeholder, value)
        .padding(SETTINGS_CONTROL_PADDING)
        .size(SETTINGS_CONTROL_TEXT_SIZE)
        .style(settings_text_input_style)
}

fn editor_field<'a>(
    label: &'a str,
    input: text_input::TextInput<'a, Message>,
    hint: Option<&'a str>,
) -> Element<'a, Message> {
    let mut field = column![
        row![settings_field_label(label).width(112), input].spacing(12).align_y(Alignment::Center)
    ]
    .spacing(6);
    if let Some(hint) = hint {
        field = field.push(text(hint).size(12).style(settings_muted_text_style));
    }
    container(field).width(Length::Fill).padding([12, 0]).into()
}

fn modal_card<'a>(
    header: Element<'a, Message>,
    body: Element<'a, Message>,
    action: Option<Element<'a, Message>>,
    height: u32,
) -> Element<'a, Message> {
    let footer = action.map(|action| {
        column![settings_divider(), container(action).align_right(Length::Fill)].spacing(12).into()
    });
    container(
        settings_modal_layout(header, body, footer)
            .width(Length::Fill)
            .max_width(960.0)
            .height(height),
    )
    .center_x(Length::Fill)
    .padding(12)
    .into()
}

fn profile_editor_body(app: &App) -> Element<'_, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let state = &app.account.profile_editor;
    let header = account_header(
        i18n.__("编辑我的资料"),
        i18n.__("更新当前登录用户的显示名称和已有登录账号。"),
        editor_back(i18n, AccountMessage::CloseProfileEditor, state.is_busy()),
    );
    let (content, action): (Element<'_, Message>, Option<Element<'_, Message>>) = match state {
        AccountProfileEditorState::Closed => (Space::new().into(), None),
        AccountProfileEditorState::Loading => (
            container(text(i18n.__("正在获取账户资料…")).size(14).style(settings_muted_text_style))
                .center_x(Length::Fill)
                .center_y(180)
                .into(),
            None,
        ),
        AccountProfileEditorState::Failed(error) => (
            column![text(i18n.__("账户资料获取失败")).size(16), settings_error_banner(error)]
                .spacing(10)
                .into(),
            Some(
                button(text(i18n.__("重新获取")).size(14))
                    .on_press(account_message(AccountMessage::OpenProfileEditor))
                    .padding([10, 16])
                    .style(primary_action_btn_style)
                    .into(),
            ),
        ),
        AccountProfileEditorState::Editing(draft) | AccountProfileEditorState::Saving(draft) => {
            let saving = state.is_busy();
            let mut name = editor_input(i18n.__("请输入显示名称"), draft.name());
            let mut username = editor_input(i18n.__("请输入登录账号"), draft.username());
            if !saving {
                name = name
                    .on_input(|value| account_message(AccountMessage::ProfileNameChanged(value)));
                username = username.on_input(|value| {
                    account_message(AccountMessage::ProfileUsernameChanged(value))
                });
            }
            let save_label = if saving { i18n.__("保存中…") } else { i18n.__("保存资料") };
            let save = button(text(save_label).size(14))
                .padding([10, 16])
                .style(primary_action_btn_style)
                .on_press_maybe(
                    profile_editor_can_submit(state)
                        .then(|| account_message(AccountMessage::SubmitProfileEditor)),
                );
            (
                settings_group_card(column![
                    editor_field(
                        i18n.__("显示名称"),
                        name,
                        Some(i18n.__("用于后台页面展示和操作记录识别。")),
                    ),
                    settings_divider(),
                    editor_field(
                        i18n.__("登录账号"),
                        username,
                        Some(i18n.__("仅已有登录账号的用户可在这里更新账号名。")),
                    ),
                ])
                .into(),
                Some(save.into()),
            )
        }
    };

    modal_card(header, content, action, 360)
}

fn password_editor_body(app: &App) -> Element<'_, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let state = &app.account.password_editor;
    let saving = state.is_busy();
    let header = account_header(
        i18n.__("修改我的密码"),
        i18n.__("使用当前密码设置新的登录密码。"),
        editor_back(i18n, AccountMessage::ClosePasswordEditor, saving),
    );
    let mut form = column![].spacing(12).width(Length::Fill);
    if let Some(message) = state.error() {
        form = form.push(
            column![
                settings_modal_header(
                    text(i18n.__("操作失败")).size(14),
                    button(text(i18n.__("关闭")).size(13))
                        .on_press(account_message(AccountMessage::DismissPasswordError))
                        .padding([6, 10])
                        .style(rounded_action_btn_style),
                ),
                settings_error_banner(message),
            ]
            .spacing(6),
        );
    }

    let draft = state.draft().cloned().unwrap_or_default();
    let password_input = |placeholder, value, field| {
        let input = editor_input(placeholder, value).secure(true);
        if saving {
            input
        } else {
            input.on_input(move |value| {
                account_message(AccountMessage::PasswordFieldChanged {
                    field,
                    value: AccountPasswordInput::new(value),
                })
            })
        }
    };
    form = form.push(settings_group_card(column![
        editor_field(
            i18n.__("当前密码"),
            password_input(
                i18n.__("请输入当前密码"),
                draft.current_password(),
                AccountPasswordField::Current
            ),
            None,
        ),
        settings_divider(),
        editor_field(
            i18n.__("新密码"),
            password_input(
                i18n.__("请输入新密码"),
                draft.new_password(),
                AccountPasswordField::New
            ),
            Some(i18n.__("密码需为 8–64 个字符，并同时包含字母和数字。")),
        ),
        settings_divider(),
        editor_field(
            i18n.__("确认新密码"),
            password_input(
                i18n.__("请再次输入新密码"),
                draft.confirmation(),
                AccountPasswordField::Confirmation
            ),
            None,
        ),
    ]));
    let submit_label = if saving { i18n.__("修改中…") } else { i18n.__("修改密码") };
    let submit = button(text(submit_label).size(14))
        .padding([10, 16])
        .style(primary_action_btn_style)
        .on_press_maybe(
            password_editor_can_submit(state)
                .then(|| account_message(AccountMessage::SubmitPasswordEditor)),
        );

    modal_card(header, form.into(), Some(submit.into()), 520)
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
        return settings_modal_overlay(Some(root_content), close_message(), editor);
    }
    if !matches!(&app.account.password_editor, AccountPasswordEditorState::Closed) {
        let editor = password_editor_body(app);
        return settings_modal_overlay(Some(root_content), close_message(), editor);
    }

    let i18n = I18n::new(app.desktop_locale);
    let identity = account.identity();
    let company_status = account
        .profile()
        .map(|profile| profile.company_status())
        .unwrap_or_else(|| identity.status())
        .to_string();
    let company_card = section_card(
        i18n,
        "公司信息",
        vec![
            detail_row(i18n, "公司名称", identity.company_name().to_string()),
            status_row(i18n, "状态", company_status),
        ],
    );

    let user_card = if let Some(profile) = account.profile() {
        section_card(
            i18n,
            "我的信息",
            vec![
                detail_row(i18n, "显示名称", profile.name().to_string()),
                detail_row(i18n, "登录账号", profile.account_name().to_string()),
                status_row(i18n, "状态", profile.user_status().to_string()),
                detail_row(i18n, "直属部门", values_label(profile.departments())),
                detail_row(i18n, "直属角色", values_label(profile.roles())),
            ],
        )
    } else {
        section_card(
            i18n,
            "我的信息",
            vec![detail_row(
                i18n,
                "资料状态",
                i18n.__("旧账户尚未获取个人资料，请重新登录刷新。").to_string(),
            )],
        )
    };

    let close = close_message();
    let card = modal_card(
        account_header(
            i18n.__("账户信息"),
            i18n.__("当前登录账户的公司与个人基础资料"),
            settings_close_button(close.clone()),
        ),
        column![company_card, user_card].spacing(12).into(),
        account.profile().map(|_| edit_profile_action(i18n)),
        610,
    );

    settings_modal_overlay(Some(root_content), close, card)
}

#[cfg(test)]
#[path = "account_details_tests.rs"]
mod account_details_tests;
