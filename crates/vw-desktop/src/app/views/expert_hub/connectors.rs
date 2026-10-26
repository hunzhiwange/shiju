//! 三列连接器卡片与统一配置弹窗；授权等待只属于对应卡片。
use super::styles;
use crate::app::{
    App, Message,
    components::system_settings_common::{
        SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, primary_action_btn_style,
        rounded_action_btn_style, settings_close_button, settings_divider, settings_modal_card,
        settings_modal_overlay, settings_muted_text_style, settings_section_card,
        settings_section_content, settings_text_input_style,
    },
    message::skill_hub::connectors::{ConnectorMessage, message},
    skill_hub::{
        ExpertHubSection, LoadState,
        connectors::{ConnectorForm, ConnectorWork},
    },
};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{Space, button, column, container, row, scrollable, text, text_input};
use iced::{Alignment, Element, Length};
use vw_gateway_client::ShijuConnector;

const GRID_COLUMNS: usize = 3;
const CARD_GAP: u32 = 16;
const SCROLLBAR_WIDTH: f32 = 4.0;
const SCROLLABLE_SIDE_PADDING: u16 = 8;

fn scrollbar() -> Direction {
    Direction::Vertical(Scrollbar::new().width(SCROLLBAR_WIDTH).scroller_width(SCROLLBAR_WIDTH))
}
pub(super) fn view(app: &App) -> Element<'_, Message> {
    let state = &app.expert_hub.connectors;
    let header = row![
        column![
            text("连接器账号").size(22),
            text("查看并授权你自己的连接器账号。").size(14).style(settings_muted_text_style)
        ]
        .spacing(6),
        Space::new().width(Length::Fill),
        button(text(if state.list_loading { "刷新中…" } else { "刷新" }).size(13))
            .padding([8, 16])
            .style(rounded_action_btn_style)
            .on_press_maybe((!state.list_loading).then(|| message(ConnectorMessage::Refresh))),
    ]
    .align_y(Alignment::Center)
    .spacing(12);
    let body: Element<'_, Message> = match &state.list {
        LoadState::Idle | LoadState::Loading { .. } => status("正在加载连接器账号…"),
        LoadState::NeedsLogin => status("请先登录识矩账户，再刷新连接器账号"),
        LoadState::Error(error) => container(
            column![
                text("连接器账号加载失败").size(16),
                text(error).size(14),
                button("重试")
                    .style(primary_action_btn_style)
                    .padding([8, 16])
                    .on_press(message(ConnectorMessage::Refresh))
            ]
            .spacing(12),
        )
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into(),
        LoadState::Loaded(connectors) if connectors.is_empty() => status("暂无可绑定连接器"),
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
    let id = connector.company_platform_id;
    let operation = app.expert_hub.connectors.work.get(&id);
    let idle = operation.is_none();
    let binding_status = match &connector.binding {
        Some(binding) if binding.status == "enabled" => "已绑定",
        Some(_) => "已停用",
        None => "未绑定",
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
            button(text("账号授权").size(13))
                .padding([8, 12])
                .style(primary_action_btn_style)
                .on_press_maybe(idle.then(|| message(ConnectorMessage::Authorize(id)))),
        );
    } else {
        actions = actions.push(
            button(text("手动绑定").size(13))
                .padding([8, 12])
                .style(rounded_action_btn_style)
                .on_press_maybe(idle.then(|| message(ConnectorMessage::Edit(id)))),
        );
    }
    content = content.push(actions);
    if let Some(operation) = operation {
        let label = match operation.work {
            ConnectorWork::Starting => "正在发起授权…",
            ConnectorWork::Authorizing { .. } => "等待浏览器授权…",
            ConnectorWork::Saving => "正在保存绑定…",
        };
        let mut waiting = row![text(label).size(12).style(settings_muted_text_style)]
            .spacing(8)
            .align_y(Alignment::Center);
        if !matches!(operation.work, ConnectorWork::Saving) {
            waiting = waiting.push(
                button(text("取消等待").size(12))
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
    if app.expert_hub.section != ExpertHubSection::Connectors {
        return base;
    }
    let Some(form) = &app.expert_hub.connectors.form else {
        return base;
    };
    let saving = app.expert_hub.connectors.form_is_saving();
    let close = message(ConnectorMessage::Cancel);
    let mut header = row![
        column![
            text(format!("绑定 {}", form.connector.platform_name)).size(20),
            text("配置仅属于当前用户，密码和密钥不会回显。")
                .size(12)
                .style(settings_muted_text_style)
        ]
        .spacing(6)
        .width(Length::Fill)
    ]
    .align_y(Alignment::Start);
    if !saving {
        header = header.push(settings_close_button(close.clone()));
    }
    let footer = row![
        Space::new().width(Length::Fill),
        button("取消")
            .padding([8, 16])
            .style(rounded_action_btn_style)
            .on_press_maybe((!saving).then_some(close.clone())),
        button(if saving { "保存中…" } else { "保存绑定" })
            .padding([8, 16])
            .style(primary_action_btn_style)
            .on_press_maybe((!saving).then(|| message(ConnectorMessage::Save)))
    ]
    .spacing(10)
    .align_y(Alignment::Center);
    let body = scrollable(
        container(binding_form(form, !saving))
            .padding([0, SCROLLABLE_SIDE_PADDING])
            .width(Length::Fill),
    )
    .direction(scrollbar())
    .width(Length::Fill)
    .height(Length::Fill);
    let card = settings_modal_card(
        column![header, settings_divider(), body, settings_divider(), footer]
            .spacing(16)
            .height(Length::Fill),
    )
    .width(Length::Fixed(720.0))
    .height(Length::Fixed(680.0));
    settings_modal_overlay(Some(base), close, card)
}
fn field<'a>(
    label: String,
    hint: &'static str,
    input: Element<'a, Message>,
) -> Element<'a, Message> {
    row![
        column![text(label).size(13), text(hint).size(11).style(settings_muted_text_style)]
            .spacing(5)
            .width(Length::Fixed(150.0)),
        container(input).width(Length::Fill)
    ]
    .spacing(20)
    .align_y(Alignment::Center)
    .padding([12, 0])
    .into()
}
fn binding_form(form: &ConnectorForm, editable: bool) -> Element<'_, Message> {
    let account_fields = column![
        field(
            "第三方账号 ID".into(),
            "连接器平台中的账号标识",
            text_input("填写第三方账号 ID", &form.account_id)
                .padding(SETTINGS_CONTROL_PADDING)
                .size(SETTINGS_CONTROL_TEXT_SIZE)
                .style(settings_text_input_style)
                .width(Length::Fill)
                .on_input_maybe(
                    editable.then_some(|value| message(ConnectorMessage::AccountId(value)))
                )
                .into()
        ),
        settings_divider(),
        field(
            "第三方账号名称".into(),
            "便于识别的账号名称",
            text_input("填写账号名称", &form.account_name)
                .padding(SETTINGS_CONTROL_PADDING)
                .size(SETTINGS_CONTROL_TEXT_SIZE)
                .style(settings_text_input_style)
                .width(Length::Fill)
                .on_input_maybe(
                    editable.then_some(|value| message(ConnectorMessage::AccountName(value)))
                )
                .into()
        ),
    ];
    let mut sections = column![
        settings_section_card("账号信息", "填写需要绑定的第三方账号"),
        settings_section_content(account_fields)
    ]
    .spacing(12);
    if !form.connector.account_binding_variables.is_empty() {
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
            let placeholder = if secret { "输入密码或密钥" } else { "填写绑定变量" };
            let hint = if secret {
                "已有密钥不回显；选填项留空保留原值"
            } else {
                "仅用于此账号的连接器配置"
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
            variables = variables.push(field(
                format!("{label}{}", if variable.required { " *" } else { "" }),
                hint,
                input.into(),
            ));
        }
        sections = sections
            .push(settings_section_card("绑定配置", "带 * 的字段为必填项"))
            .push(settings_section_content(variables));
    }
    sections.into()
}
