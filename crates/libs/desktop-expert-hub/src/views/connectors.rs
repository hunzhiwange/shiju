//! 三列连接器卡片与统一配置弹窗；授权等待只属于对应卡片。
use super::list::cards;
use super::styles;
use super::ViewInput;
use desktop_ui_controls::system_settings_common::*;
use desktop_i18n::I18n;
use crate::message::connectors::ConnectorMessage;
use desktop_skill_hub::skill_hub::{
    ExpertHubSection, LoadState,
    connectors::{ConnectorForm, ConnectorWork, ConnectorsTab},
};
/// 宿主提供原有可选择帮助文本及其选择/复制消息。
pub type HelpRenderer<M> = for<'a> fn(&'a str, std::borrow::Cow<'a, str>, Option<iced::Color>, I18n) -> Element<'a, M>;
fn help_label<'a, M: Clone + 'static>(render: HelpRenderer<M>, i18n: I18n, theme: &Theme, title: &'a str, label: iced::widget::Text<'a>, description: impl Into<std::borrow::Cow<'a, str>>) -> iced::widget::Row<'a, M> {
    desktop_ui_controls::system_settings_common::help::settings_hover_help_label(label, render(title, description.into(), Some(theme.palette().text), i18n))
}
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::tooltip::Position;
use iced::widget::{Space, button, column, container, responsive, row, scrollable, text, tooltip};
use iced::{Alignment, Background, Element, Length, Theme};
use gateway_shiju_connector::{
    ShijuConnector, ShijuConnectorCatalogEntry, ShijuConnectorCatalogScope,
    ShijuConnectorCatalogStatus,
};

const GRID_COLUMNS: usize = 3;
const CARD_GAP: u32 = 16;
const SCROLLBAR_WIDTH: f32 = 4.0;
const SCROLLABLE_SIDE_PADDING: u16 = 8;

fn scrollbar() -> Direction {
    Direction::Vertical(Scrollbar::new().width(SCROLLBAR_WIDTH).scroller_width(SCROLLBAR_WIDTH))
}
pub fn view<'a, M: Clone + 'static>(app: &ViewInput<'a>, help: HelpRenderer<M>, message: fn(ConnectorMessage) -> M) -> Element<'a, M> {
    let i18n = I18n::new(app.desktop_locale);
    let state = &app.expert_hub.connectors;
    let showing_accounts = state.tab == ConnectorsTab::Accounts;
    let loading = if showing_accounts { state.list_loading } else { state.catalog_loading };
    let title = i18n.__(if showing_accounts { "连接器账号" } else { "连接器" });
    let heading: Element<'_, M> = if showing_accounts {
        help_label(
            help,
            i18n,
            app.app_theme,
            title,
            text(title).size(22),
            i18n.__("查看并授权你自己的连接器账号。"),
        )
        .wrap()
        .into()
    } else {
        text(title).size(22).into()
    };
    let mut header = row![heading, Space::new().width(Length::Fill)]
        .align_y(Alignment::Center)
        .spacing(12);
    if !showing_accounts {
        header = header.push(
            settings_text_input(i18n.__("搜索连接器"), &state.query)
                .width(Length::Fixed(220.0))
                .on_input(move |query| message(ConnectorMessage::Search(query))),
        );
    }
    let header = header.push(
        button(
            text(if loading {
                i18n.__("刷新中…")
            } else {
                i18n.__("刷新")
            })
            .size(13)
        )
        .padding([8, 16])
        .style(rounded_action_btn_style)
        .on_press_maybe((!loading).then(|| message(ConnectorMessage::Refresh))),
    );
    let mut tabs = row![].spacing(8).align_y(Alignment::Center);
    for (label, tab) in [
        ("连接器账号", ConnectorsTab::Accounts),
        ("系统连接器", ConnectorsTab::Catalog(ShijuConnectorCatalogScope::System)),
        ("自定义连接器", ConnectorsTab::Catalog(ShijuConnectorCatalogScope::Custom)),
    ] {
        let selected = state.tab == tab;
        tabs = tabs.push(
            button(text(i18n.__(label)).size(14))
                .padding([8, 12])
                .style(move |theme, status| styles::selected_tab_style(theme, status, selected))
                .on_press(message(ConnectorMessage::SelectTab(tab))),
        );
    }
    let body = if showing_accounts {
        list_body(&state.list, i18n, true, message, |items| {
            scrolling_grid(card_grid(app, items, message))
        })
    } else {
        list_body(&state.catalog, i18n, false, message, |items| {
            catalog_grid(app, items, message)
        })
    };
    column![header, tabs.wrap(), body]
        .spacing(18)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn list_body<'a, M: Clone + 'static, T>(
    list: &'a LoadState<Vec<T>>,
    i18n: I18n,
    accounts: bool,
    message: fn(ConnectorMessage) -> M,
    render: impl FnOnce(&'a [T]) -> Element<'a, M>,
) -> Element<'a, M> {
    match list {
        LoadState::Idle | LoadState::Loading { .. } => {
            status(i18n.__(if accounts { "正在加载连接器账号…" } else { "正在加载连接器…" }))
        }
        LoadState::NeedsLogin => status(
            i18n.__(if accounts {
                "请先登录识矩账户，再刷新连接器账号"
            } else {
                "请先登录识矩账户，再刷新连接器"
            }),
        ),
        LoadState::Error(error) => container(
            column![
                text(i18n.__(if accounts { "连接器账号加载失败" } else { "连接器目录加载失败" }))
                    .size(16),
                text(error.render(i18n)).size(14),
                button(i18n.__("重试"))
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
            status(i18n.__(if accounts { "暂无可绑定连接器" } else { "暂无连接器" }))
        }
        LoadState::Loaded(items) => render(items),
    }
}
// 始终三列；末行补空位，内容两侧留出 4px 滚动条及间距。
pub fn card_grid<'a, M: Clone + 'static>(
    app: &ViewInput<'a>,
    connectors: &'a [ShijuConnector],
    message: fn(ConnectorMessage) -> M,
) -> Element<'a, M> {
    grid(connectors, |connector| connector_card(app, connector, None, message))
}

fn grid<'a, 'items, T, M: Clone + 'static>(
    items: &'items [T],
    render: impl Fn(&'items T) -> Element<'a, M>,
) -> Element<'a, M> {
    let mut grid = column![].spacing(CARD_GAP).width(Length::Fill);
    for chunk in items.chunks(GRID_COLUMNS) {
        let mut cards = row![].spacing(CARD_GAP).width(Length::Fill);
        for item in chunk {
            cards = cards.push(render(item));
        }
        for _ in chunk.len()..GRID_COLUMNS {
            cards = cards.push(Space::new().width(Length::FillPortion(1)));
        }
        grid = grid.push(cards);
    }
    container(grid).padding([4, SCROLLABLE_SIDE_PADDING]).width(Length::Fill).into()
}

fn catalog_grid<'a, M: Clone + 'static>(
    app: &ViewInput<'a>,
    entries: &'a [ShijuConnectorCatalogEntry],
    message: fn(ConnectorMessage) -> M,
) -> Element<'a, M> {
    let query = app.expert_hub.connectors.query.trim().to_lowercase();
    let filtered: Vec<_> = entries
        .iter()
        .filter(|entry| {
            query.is_empty()
                || [&entry.name, &entry.code, &entry.description]
                    .iter()
                    .any(|value| value.to_lowercase().contains(&query))
        })
        .collect();
    if filtered.is_empty() {
        return status(I18n::new(app.desktop_locale).__("没有匹配的连接器"));
    }
    scrolling_grid(grid(&filtered, |&entry| catalog_card(app, entry, message)))
}

fn scrolling_grid<'a, M: Clone + 'static>(content: Element<'a, M>) -> Element<'a, M> {
    scrollable(content)
        .direction(scrollbar())
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn catalog_identity<'a, M: Clone + 'static>(entry: &ShijuConnectorCatalogEntry) -> Element<'a, M> {
    let subtitle = match (entry.code.is_empty(), entry.description.is_empty()) {
        (true, _) => entry.description.clone(),
        (_, true) => entry.code.clone(),
        _ => format!("{} · {}", entry.code, entry.description),
    };
    cards::identity(entry.platform_id, &entry.name, subtitle)
}

fn catalog_card<'a, M: Clone + 'static>(
    app: &ViewInput<'a>,
    entry: &'a ShijuConnectorCatalogEntry,
    message: fn(ConnectorMessage) -> M,
) -> Element<'a, M> {
    if let Some(connector) = &entry.account {
        return connector_card(app, connector, Some(entry), message);
    }
    let label = I18n::new(app.desktop_locale).__(match entry.status {
        ShijuConnectorCatalogStatus::Unavailable => "公司未开通",
        ShijuConnectorCatalogStatus::Disabled => "已停用",
        ShijuConnectorCatalogStatus::NoBinding => "无需授权",
        ShijuConnectorCatalogStatus::NoAccess | ShijuConnectorCatalogStatus::Bindable => "无权绑定",
    });
    cards::card_surface(
        row![
            catalog_identity(entry),
            container(text(label).size(11)).padding([4, 8]).style(styles::meta_badge_style),
        ]
        .spacing(8)
        .height(60)
        .align_y(Alignment::Start),
        false,
    )
    .into()
}

fn connector_card<'a, M: Clone + 'static>(
    app: &ViewInput<'a>,
    connector: &'a ShijuConnector,
    catalog: Option<&ShijuConnectorCatalogEntry>,
    message: fn(ConnectorMessage) -> M,
) -> Element<'a, M> {
    let i18n = I18n::new(app.desktop_locale);
    let id = connector.company_platform_id;
    let operation = app.expert_hub.connectors.work.get(&id);
    let idle = operation.is_none();
    let is_catalog = catalog.is_some();
    let connected = connector.binding.as_ref().is_some_and(|binding| binding.status == "enabled");
    let binding_status = i18n.__(match (is_catalog, &connector.binding) {
        (true, Some(_)) if connected => "已连接",
        (false, Some(_)) if connected => "已绑定",
        (true, Some(binding)) if binding.status == "unbound" => "未连接",
        (_, Some(_)) => "已停用",
        (true, None) => "未连接",
        (false, None) => "未绑定",
    });
    let identity = catalog.map_or_else(
        || cards::identity(connector.platform_id, &connector.platform_name, connector.platform_code.clone()),
        catalog_identity,
    );
    let mut binding_details = column![
        container(text(binding_status).size(11))
            .padding([4, 8])
            .style(move |theme| {
                let mut style = styles::meta_badge_style(theme);
                if is_catalog && connected {
                    let success = theme.extended_palette().success.weak;
                    style.background = Some(success.color.into());
                    style.text_color = Some(success.text);
                }
                style
            }),
    ]
    .spacing(4)
    .align_x(Alignment::End);
    if let Some(binding) = &connector.binding {
        let account =
            [binding.third_party_account_name.as_str(), binding.third_party_account_id.as_str()]
                .into_iter()
                .filter(|value| !value.is_empty())
                .collect::<Vec<_>>();
        if !account.is_empty() {
            let labels = column(account.iter().map(|value| {
                text(*value).size(12).style(cards::muted_text).wrapping(text::Wrapping::None).into()
            }))
            .spacing(2)
            .align_x(Alignment::End)
            .width(Length::Fill);
            binding_details = binding_details.width(Length::FillPortion(1)).push(
                tooltip(
                    container(labels).width(Length::Fill).clip(true),
                    container(text(account.join("\n")).size(12).width(Length::Fixed(240.0)))
                        .padding([6, 10])
                        .style(icons::icon_btn_tooltip_style),
                    Position::Top,
                )
                .gap(5),
            );
        }
    }
    let action = if is_catalog {
        let action = if connector.authorization_available {
            ConnectorMessage::Authorize(id)
        } else {
            ConnectorMessage::Edit(id)
        };
        button(text(i18n.__(if connected { "重新连接" } else { "连接" })).size(13))
            .padding([8, 12])
            .style(move |theme, status| {
                if connected {
                    rounded_action_btn_style(theme, status)
                } else {
                    primary_action_btn_style(theme, status)
                }
            })
            .on_press_maybe(idle.then(|| message(action)))
    } else if connector.authorization_available {
        button(text(i18n.__("账号授权")).size(13))
            .padding([8, 12])
            .style(primary_action_btn_style)
            .on_press_maybe(idle.then(|| message(ConnectorMessage::Authorize(id))))
    } else {
        button(text(i18n.__("手动绑定")).size(13))
            .padding([8, 12])
            .style(rounded_action_btn_style)
            .on_press_maybe(idle.then(|| message(ConnectorMessage::Edit(id))))
    };
    let heading = row![identity, binding_details].spacing(8).height(60).align_y(Alignment::Start);
    let mut content =
        column![heading, cards::line(), row![Space::new().width(Length::Fill), action]].spacing(10);
    if let Some(operation) = operation {
        let label = match operation.work {
            ConnectorWork::Starting => i18n.__("正在发起授权…"),
            ConnectorWork::Authorizing { .. } => i18n.__("等待浏览器授权…"),
            ConnectorWork::Saving => i18n.__("正在保存绑定…"),
        };
        let mut waiting = row![text(label).size(12).style(settings_muted_text_style)]
            .spacing(8)
            .align_y(Alignment::Center);
        if !matches!(operation.work, ConnectorWork::Saving) {
            waiting = waiting.push(
                button(text(i18n.__("取消等待")).size(12))
                    .padding([6, 10])
                    .style(rounded_action_btn_style)
                    .on_press(message(ConnectorMessage::CancelAuthorization(id))),
            );
        }
        content = content.push(waiting.wrap());
    }
    cards::card_surface(content, false).into()
}
fn status<'a, M: Clone + 'static>(label: &'a str) -> Element<'a, M> {
    container(text(label).size(16)).center_x(Length::Fill).center_y(Length::Fill).into()
}

pub fn binding_modal<'a, M: Clone + 'static>(app: &ViewInput<'a>, help: HelpRenderer<M>, message: fn(ConnectorMessage) -> M) -> Option<(M, Element<'a, M>)> {
    let i18n = desktop_i18n::I18n::new(app.desktop_locale);
    if app.expert_hub.section != ExpertHubSection::Connectors {
        return None;
    }
    let Some(form) = &app.expert_hub.connectors.form else {
        return None;
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
        button(desktop_i18n::I18n::new(app.desktop_locale).__("取消"))
            .padding([8, 16])
            .style(rounded_action_btn_style)
            .on_press_maybe((!saving).then_some(close.clone())),
        button(if saving {
            desktop_i18n::I18n::new(app.desktop_locale).__("保存中…")
        } else {
            desktop_i18n::I18n::new(app.desktop_locale).__("保存绑定")
        })
        .padding([8, 16])
        .style(primary_action_btn_style)
        .on_press_maybe((!saving).then(|| message(ConnectorMessage::Save)))
    ]
    .spacing(10)
    .align_y(Alignment::Center);
    let theme = app.app_theme;
    let body = responsive(move |size| {
        binding_form(i18n, theme, form, !saving, help, message, size.width >= 600.0)
    })
    .height(Length::Shrink);
    let card = settings_modal_layout(header, body, Some(footer.into()))
        .width(Length::Fixed(720.0))
        .height(Length::Fixed(680.0))
        .style(|theme| {
            let mut style = settings_modal_card_style(theme);
            if let Some(Background::Color(color)) = &mut style.background {
                color.a = 1.0;
            }
            style
        });
    Some((close, container(card).padding(12).into()))
}

fn field<'a, M: Clone + 'static>(
    wide: bool,
    label: impl Into<Element<'a, M>>,
    control: impl Into<Element<'a, M>>,
) -> Element<'a, M> {
    if wide {
        settings_flexible_row_with_label(container(label).width(Length::Fill), 460.0, control)
            .padding([12, 0])
            .into()
    } else {
        column![label.into(), control.into()].spacing(6).padding([12, 0]).width(Length::Fill).into()
    }
}

fn binding_form<'a, M: Clone + 'static>(
    i18n: I18n,
    theme: &Theme,
    form: &'a ConnectorForm,
    editable: bool,
    help: HelpRenderer<M>,
    message: fn(ConnectorMessage) -> M,
    wide: bool,
) -> Element<'a, M> {
    let account_id = i18n.__("第三方账号 ID");
    let account_name = i18n.__("第三方账号名称");
    let account_fields = column![
        field(
            wide,
            help_label(
                help,
                i18n,
                theme,
                account_id,
                settings_field_label(account_id),
                i18n.__("连接器平台中的账号标识"),
            )
            .wrap(),
            settings_text_input(i18n.__("填写第三方账号 ID"), &form.account_id)
                .width(Length::Fill)
                .on_input_maybe(
                    editable.then_some(move |value| message(ConnectorMessage::AccountId(value)))
                )
        ),
        settings_divider(),
        field(
            wide,
            help_label(
                help,
                i18n,
                theme,
                account_name,
                settings_field_label(account_name),
                i18n.__("便于识别的账号名称"),
            )
            .wrap(),
            settings_text_input(i18n.__("填写账号名称"), &form.account_name)
                .width(Length::Fill)
                .on_input_maybe(
                    editable.then_some(move |value| message(ConnectorMessage::AccountName(value)))
                )
        ),
    ];
    let mut sections = column![
        help_label(help, i18n, theme, i18n.__("账号信息"), text(i18n.__("账号信息")).size(17), i18n.__("填写需要绑定的第三方账号")).wrap(),
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
            let input = settings_text_input(placeholder, value)
                .secure(secret)
                .width(Length::Fill)
                .on_input_maybe(editable.then_some(move |value| {
                    message(ConnectorMessage::Variable(name.clone(), value))
                }));
            let label =
                text(format!("{label}{}", if variable.required { " *" } else { "" })).size(15);
            let field = if secret {
                field(
                    wide,
                    label,
                    settings_control_with_hint(
                        input,
                        i18n.__("已有密钥不回显；选填项留空保留原值"),
                    ),
                )
            } else {
                field(
                    wide,
                    help_label(
                        help,
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
