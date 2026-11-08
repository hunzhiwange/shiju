//! 三列连接器卡片与统一配置弹窗；授权等待只属于对应卡片。
use super::styles;
use crate::app::{
    App, Message,
    components::system_settings_common::{
        SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, primary_action_btn_style,
        rounded_action_btn_style, settings_close_button, settings_control_with_hint,
        settings_divider, settings_group_card, settings_group_heading,
        settings_group_heading_with_help, settings_hover_help_label, settings_modal_header,
        settings_modal_layout, settings_modal_overlay, settings_muted_text_style,
        settings_text_input_style,
    },
    i18n::I18n,
    message::skill_hub::connectors::{ConnectorMessage, message},
    skill_hub::{
        ExpertHubSection, LoadState,
        connectors::{ConnectorForm, ConnectorWork},
    },
};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{Space, button, column, container, row, scrollable, text, text_input};
use iced::{Alignment, Element, Length, Theme};
use vw_gateway_client::ShijuConnector;

const GRID_COLUMNS: usize = 3;
const CARD_GAP: u32 = 16;
const SCROLLBAR_WIDTH: f32 = 4.0;
const SCROLLABLE_SIDE_PADDING: u16 = 8;

fn scrollbar() -> Direction {
    Direction::Vertical(Scrollbar::new().width(SCROLLBAR_WIDTH).scroller_width(SCROLLBAR_WIDTH))
}
pub(super) fn view(app: &App) -> Element<'_, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let state = &app.expert_hub.connectors;
    let title = i18n.__("连接器账号");
    let header = row![
        settings_hover_help_label(
            i18n,
            &app.app_theme,
            title,
            text(title).size(22),
            i18n.__("查看并授权你自己的连接器账号。"),
        )
        .wrap(),
        Space::new().width(Length::Fill),
        button(
            text(crate::app::i18n::I18n::new(app.desktop_locale).__(if state.list_loading {
                crate::app::i18n::I18n::new(app.desktop_locale).__("刷新中…")
            } else {
                crate::app::i18n::I18n::new(app.desktop_locale).__("刷新")
            }))
            .size(13)
        )
        .padding([8, 16])
        .style(rounded_action_btn_style)
        .on_press_maybe((!state.list_loading).then(|| message(ConnectorMessage::Refresh))),
    ]
    .align_y(Alignment::Center)
    .spacing(12);
    let body: Element<'_, Message> = match &state.list {
        LoadState::Idle | LoadState::Loading { .. } => {
            status(crate::app::i18n::I18n::new(app.desktop_locale).__("正在加载连接器账号…"))
        }
        LoadState::NeedsLogin => status(
            crate::app::i18n::I18n::new(app.desktop_locale)
                .__("请先登录识矩账户，再刷新连接器账号"),
        ),
        LoadState::Error(error) => container(
            column![
                text(crate::app::i18n::I18n::new(app.desktop_locale).__("连接器账号加载失败"))
                    .size(16),
                text(error.render(crate::app::i18n::I18n::new(app.desktop_locale))).size(14),
                button(crate::app::i18n::I18n::new(app.desktop_locale).__("重试"))
                    .style(primary_action_btn_style)
                    .padding([8, 16])
                    .on_press(message(ConnectorMessage::Refresh))
            ]
            .spacing(12),
        )
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into(),
        LoadState::Loaded(connectors) if connectors.is_empty() => {
            status(crate::app::i18n::I18n::new(app.desktop_locale).__("暂无可绑定连接器"))
        }
        LoadState::Loaded(connectors) => scrollable(card_grid(app, connectors))
            .direction(scrollbar())
            .width(Length::Fill)
            .height(Length::Fill)
            .into(),
    };
    column![header, body].spacing(18).width(Length::Fill).height(Length::Fill).into()
}
// 始终三列；末行补空位，内容两侧留出 4px 滚动条及间距。
pub(super) fn card_grid<'a>(
    app: &'a App,
    connectors: &'a [ShijuConnector],
) -> Element<'a, Message> {
    let mut grid = column![].spacing(CARD_GAP).width(Length::Fill);
    for chunk in connectors.chunks(GRID_COLUMNS) {
        let mut cards = row![].spacing(CARD_GAP).width(Length::Fill);
        for connector in chunk {
            cards = cards.push(connector_card(app, connector));
        }
        for _ in chunk.len()..GRID_COLUMNS {
            cards = cards.push(Space::new().width(Length::FillPortion(1)));
        }
        grid = grid.push(cards);
    }
    container(grid).padding([4, SCROLLABLE_SIDE_PADDING]).width(Length::Fill).into()
}
fn connector_card<'a>(app: &'a App, connector: &'a ShijuConnector) -> Element<'a, Message> {
    let i18n = crate::app::i18n::I18n::new(app.desktop_locale);
    let id = connector.company_platform_id;
    let operation = app.expert_hub.connectors.work.get(&id);
    let idle = operation.is_none();
    let binding_status = match &connector.binding {
        Some(binding) if binding.status == "enabled" => {
            crate::app::i18n::I18n::new(app.desktop_locale).__("已绑定")
        }
        Some(_) => crate::app::i18n::I18n::new(app.desktop_locale).__("已停用"),
        None => crate::app::i18n::I18n::new(app.desktop_locale).__("未绑定"),
    };
    let identity = column![
        text(&connector.platform_name).size(17),
        text(&connector.platform_code).size(12).style(settings_muted_text_style),
    ]
    .spacing(6)
    .width(Length::FillPortion(1));
    let mut binding_details = column![
        container(text(binding_status).size(11)).padding([4, 8]).style(styles::meta_badge_style)
    ]
    .spacing(5)
    .align_x(Alignment::End)
    .width(Length::FillPortion(1));
    if let Some(binding) = &connector.binding {
        if !binding.third_party_account_name.is_empty() {
            binding_details =
                binding_details.push(text(&binding.third_party_account_name).size(13));
        }
        if !binding.third_party_account_id.is_empty() {
            binding_details = binding_details.push(
                text(&binding.third_party_account_id).size(12).style(settings_muted_text_style),
            );
        }
    }
    let mut content =
        column![row![identity, binding_details].spacing(12).align_y(Alignment::Start),].spacing(12);
    let mut actions = row![].spacing(8);
    if connector.authorization_available {
        actions = actions.push(
            button(text(crate::app::i18n::I18n::new(app.desktop_locale).__("账号授权")).size(13))
                .padding([8, 12])
                .style(primary_action_btn_style)
                .on_press_maybe(idle.then(|| message(ConnectorMessage::Authorize(id)))),
        );
    } else {
        actions = actions.push(
            button(text(crate::app::i18n::I18n::new(app.desktop_locale).__("手动绑定")).size(13))
                .padding([8, 12])
                .style(rounded_action_btn_style)
                .on_press_maybe(idle.then(|| message(ConnectorMessage::Edit(id)))),
        );
    }
    content = content.push(actions);
    if let Some(operation) = operation {
        let label = match operation.work {
            ConnectorWork::Starting => {
                crate::app::i18n::I18n::new(app.desktop_locale).__("正在发起授权…")
            }
            ConnectorWork::Authorizing { .. } => {
                crate::app::i18n::I18n::new(app.desktop_locale).__("等待浏览器授权…")
            }
            ConnectorWork::Saving => {
                crate::app::i18n::I18n::new(app.desktop_locale).__("正在保存绑定…")
            }
        };
        let mut waiting = row![text(i18n.__(label)).size(12).style(settings_muted_text_style)]
            .spacing(8)
            .align_y(Alignment::Center);
        if !matches!(operation.work, ConnectorWork::Saving) {
            waiting = waiting.push(
                button(
                    text(crate::app::i18n::I18n::new(app.desktop_locale).__("取消等待")).size(12),
                )
                .padding([6, 10])
                .style(rounded_action_btn_style)
                .on_press(message(ConnectorMessage::CancelAuthorization(id))),
            );
        }
        content = content.push(waiting);
    }
    container(content).padding(16).width(Length::FillPortion(1)).style(styles::card_style).into()
}
fn status(label: &str) -> Element<'_, Message> {
    container(text(label).size(16)).center_x(Length::Fill).center_y(Length::Fill).into()
}

pub(super) fn with_binding_modal<'a>(
    app: &'a App,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    let i18n = crate::app::i18n::I18n::new(app.desktop_locale);
    if app.expert_hub.section != ExpertHubSection::Connectors {
        return base;
    }
    let Some(form) = &app.expert_hub.connectors.form else {
        return base;
    };
    let saving = app.expert_hub.connectors.form_is_saving();
    let close = message(ConnectorMessage::Cancel);
    let header = settings_modal_header(
        column![
            text(
                i18n.format("绑定 %s", &[form.connector.platform_name.clone().into()])
                    .expect("valid display template")
            )
            .size(20),
            text(i18n.__("配置仅属于当前用户，密码和密钥不会回显。"))
                .size(13)
                .style(settings_muted_text_style)
        ]
        .spacing(6),
        if saving { Space::new().into() } else { settings_close_button(close.clone()) },
    );
    let footer = row![
        Space::new().width(Length::Fill),
        button(crate::app::i18n::I18n::new(app.desktop_locale).__("取消"))
            .padding([8, 16])
            .style(rounded_action_btn_style)
            .on_press_maybe((!saving).then_some(close.clone())),
        button(if saving {
            crate::app::i18n::I18n::new(app.desktop_locale).__("保存中…")
        } else {
            crate::app::i18n::I18n::new(app.desktop_locale).__("保存绑定")
        })
        .padding([8, 16])
        .style(primary_action_btn_style)
        .on_press_maybe((!saving).then(|| message(ConnectorMessage::Save)))
    ]
    .spacing(10)
    .align_y(Alignment::Center);
    let card = settings_modal_layout(
        header,
        binding_form(i18n, &app.app_theme, form, !saving),
        Some(footer.into()),
    )
    .width(Length::Fixed(720.0))
    .height(Length::Fixed(680.0));
    settings_modal_overlay(Some(base), close, container(card).padding(12))
}

fn field<'a>(
    label: impl Into<Element<'a, Message>>,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    column![label.into(), control.into()].spacing(6).padding([12, 0]).width(Length::Fill).into()
}

fn binding_form<'a>(
    i18n: I18n,
    theme: &Theme,
    form: &'a ConnectorForm,
    editable: bool,
) -> Element<'a, Message> {
    let account_id = i18n.__("第三方账号 ID");
    let account_name = i18n.__("第三方账号名称");
    let account_fields = column![
        field(
            settings_hover_help_label(
                i18n,
                theme,
                account_id,
                text(account_id).size(13),
                i18n.__("连接器平台中的账号标识"),
            )
            .wrap(),
            text_input(i18n.__("填写第三方账号 ID"), &form.account_id)
                .padding(SETTINGS_CONTROL_PADDING)
                .size(SETTINGS_CONTROL_TEXT_SIZE)
                .style(settings_text_input_style)
                .width(Length::Fill)
                .on_input_maybe(
                    editable.then_some(|value| message(ConnectorMessage::AccountId(value)))
                )
        ),
        settings_divider(),
        field(
            settings_hover_help_label(
                i18n,
                theme,
                account_name,
                text(account_name).size(13),
                i18n.__("便于识别的账号名称"),
            )
            .wrap(),
            text_input(i18n.__("填写账号名称"), &form.account_name)
                .padding(SETTINGS_CONTROL_PADDING)
                .size(SETTINGS_CONTROL_TEXT_SIZE)
                .style(settings_text_input_style)
                .width(Length::Fill)
                .on_input_maybe(
                    editable.then_some(|value| message(ConnectorMessage::AccountName(value)))
                )
        ),
    ];
    let mut sections = column![
        settings_group_heading_with_help(
            i18n,
            theme,
            i18n.__("账号信息"),
            i18n.__("填写需要绑定的第三方账号"),
        ),
        settings_group_card(account_fields)
    ]
    .spacing(12);
    if !form.connector.account_binding_variables.is_empty() {
        let binding_title = i18n.__("绑定配置");
        let mut variables = column![];
        for (index, variable) in form.connector.account_binding_variables.iter().enumerate() {
            if index > 0 {
                variables = variables.push(settings_divider());
            }
            let label = if variable.description.trim().is_empty() {
                variable.name.clone()
            } else {
                variable.description.clone()
            };
            let name = variable.name.clone();
            let value = form.values.get(&name).map(String::as_str).unwrap_or_default();
            let secret = variable.kind == "password";
            let placeholder = if secret {
                i18n.__("输入密码或密钥")
            } else {
                i18n.__("填写绑定变量")
            };
            let input = text_input(placeholder, value)
                .secure(secret)
                .padding(SETTINGS_CONTROL_PADDING)
                .size(SETTINGS_CONTROL_TEXT_SIZE)
                .style(settings_text_input_style)
                .width(Length::Fill)
                .on_input_maybe(editable.then_some(move |value| {
                    message(ConnectorMessage::Variable(name.clone(), value))
                }));
            let label =
                text(format!("{label}{}", if variable.required { " *" } else { "" })).size(13);
            let field = if secret {
                field(
                    label,
                    settings_control_with_hint(
                        input,
                        i18n.__("已有密钥不回显；选填项留空保留原值"),
                    ),
                )
            } else {
                field(
                    settings_hover_help_label(
                        i18n,
                        theme,
                        binding_title,
                        label,
                        i18n.__("仅用于此账号的连接器配置"),
                    )
                    .wrap(),
                    input,
                )
            };
            variables = variables.push(field);
        }
        sections = sections
            .push(settings_group_heading(binding_title, i18n.__("带 * 的字段为必填项")))
            .push(settings_group_card(variables));
    }
    sections.into()
}

#[cfg(test)]
#[path = "connectors_ui_refresh_39_tests.rs"]
mod ui_refresh_39_tests;
