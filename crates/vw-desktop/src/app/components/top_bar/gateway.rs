//! 顶栏网关服务弹窗。

use super::widgets::{color_with_alpha, icon_svg, menu_item_btn};
use crate::app::assets::Icon;
use crate::app::components::overlays::BelowOverlay;
use crate::app::components::system_settings::SystemTab;
use crate::app::message::settings::{
    AgentConfigTabMessage, GatewayClientMessage, McpMessage, SettingsMessage,
};
use crate::app::message::view::MenuType;
use crate::app::state::{AgentConfigTab, TopBarGatewayTab};
use crate::app::{App, Message, message};
use iced::widget::svg;
use iced::widget::tooltip::{Position as TooltipPosition, Tooltip};
use iced::widget::{Space, button, column, container, row, text};
use iced::{Color, Element, Length, Theme};

fn status_dot(active: bool) -> Element<'static, Message> {
    container(Space::new().width(Length::Fixed(8.0)).height(Length::Fixed(8.0)))
        .style(move |theme: &Theme| {
            let color = if active {
                Color::from_rgb8(18, 190, 35)
            } else {
                theme.palette().text.scale_alpha(0.28)
            };
            iced::widget::container::Style {
                background: Some(iced::Background::Color(color)),
                border: iced::Border { width: 0.0, color: Color::TRANSPARENT, radius: 4.0.into() },
                ..Default::default()
            }
        })
        .into()
}

fn gateway_server_healthy(app: &App, server: &crate::app::state::GatewayClientServerDraft) -> bool {
    crate::app::message::gateway_health::server_health_key(server)
        .and_then(|key| app.gateway_client_settings.health.get(&key).copied())
        .unwrap_or(false)
}

fn active_gateway_healthy(app: &App) -> bool {
    app.gateway_client_settings
        .servers
        .iter()
        .find(|server| server.id == app.gateway_client_settings.selected_server_id)
        .is_some_and(|server| gateway_server_healthy(app, server))
}

fn gateway_services_button(active: bool, usable: bool) -> Element<'static, Message> {
    let gateway_icon = container(icon_svg(Icon::HddNetwork).style(|theme: &Theme, _status| {
        svg::Style { color: Some(theme.palette().text.scale_alpha(0.88)) }
    }))
    .width(Length::Fixed(16.0))
    .height(Length::Fixed(16.0))
    .align_x(iced::Alignment::Center)
    .align_y(iced::Alignment::Center);

    let btn = button(
        container(
            row![
                gateway_icon,
                container(status_dot(usable))
                    .width(Length::Fixed(10.0))
                    .height(Length::Fixed(16.0))
                    .align_y(iced::Alignment::Start),
            ]
            .spacing(2)
            .align_y(iced::Alignment::Center),
        )
        .height(Length::Fixed(24.0))
        .align_y(iced::Alignment::Center),
    )
    .height(Length::Fixed(24.0))
    .padding([0, 6])
    .on_press(Message::View(message::ViewMessage::ToggleMenu(Some(MenuType::GatewayServices))))
    .style(move |theme: &Theme, status| {
        let bg = match status {
            iced::widget::button::Status::Hovered => {
                Some(color_with_alpha(theme.palette().text, 0.14).into())
            }
            iced::widget::button::Status::Pressed => {
                Some(color_with_alpha(theme.palette().text, 0.22).into())
            }
            _ if active => Some(color_with_alpha(theme.palette().text, 0.10).into()),
            _ => None,
        };
        iced::widget::button::Style {
            background: bg,
            border: iced::Border { width: 0.0, color: Color::TRANSPARENT, radius: 6.0.into() },
            text_color: theme.palette().text,
            ..Default::default()
        }
    });

    let tip_content = container(text("网关服务")).padding([6, 10]).style(|theme: &Theme| {
        iced::widget::container::Style {
            text_color: Some(theme.palette().text),
            background: Some(iced::Background::Color(theme.palette().background)),
            border: iced::Border {
                width: 1.0,
                color: theme.extended_palette().background.strong.color,
                radius: 8.0.into(),
            },
            shadow: iced::Shadow::default(),
            snap: false,
        }
    });

    Tooltip::new(btn, tip_content, TooltipPosition::Bottom).gap(10).into()
}

fn gateway_menu_container<'a>(content: Element<'a, Message>) -> Element<'a, Message> {
    container(content)
        .padding(6)
        .width(Length::Fixed(460.0))
        .style(|theme: &Theme| iced::widget::container::Style {
            text_color: Some(theme.palette().text),
            background: Some(iced::Background::Color(color_with_alpha(
                theme.palette().background,
                0.96,
            ))),
            border: iced::Border {
                width: 1.0,
                color: theme.extended_palette().background.strong.color.scale_alpha(0.70),
                radius: 8.0.into(),
            },
            shadow: iced::Shadow {
                color: Color::BLACK.scale_alpha(0.18),
                offset: iced::Vector::new(0.0, 10.0),
                blur_radius: 26.0,
            },
            snap: false,
        })
        .into()
}

fn tab_label(
    label: &'static str,
    tab: TopBarGatewayTab,
    active: bool,
) -> Element<'static, Message> {
    let label_text =
        move |theme: &Theme, status: iced::widget::button::Status| iced::widget::text::Style {
            color: Some(if active {
                theme.palette().text
            } else if status == iced::widget::button::Status::Hovered {
                theme.palette().text.scale_alpha(0.82)
            } else {
                theme.palette().text.scale_alpha(0.62)
            }),
        };
    let underline = container(Space::new().width(Length::Fill).height(Length::Fixed(2.0))).style(
        move |theme: &Theme| iced::widget::container::Style {
            background: active.then_some(iced::Background::Color(theme.palette().text)),
            ..Default::default()
        },
    );

    button(column![text(label).size(13), underline].spacing(6).align_x(iced::Alignment::Center))
        .on_press(Message::View(message::ViewMessage::GatewayServicesTabSelected(tab)))
        .padding([4, 8])
        .style(move |theme: &Theme, status| iced::widget::button::Style {
            background: None,
            text_color: label_text(theme, status).color.unwrap_or(theme.palette().text),
            border: iced::Border { width: 0.0, color: Color::TRANSPARENT, radius: 4.0.into() },
            ..Default::default()
        })
        .into()
}

fn gateway_row<'a>(
    icon: Element<'a, Message>,
    label: String,
    mark: Option<&'static str>,
    msg: Option<Message>,
) -> Element<'a, Message> {
    gateway_row_with_menu_behavior(icon, label, mark, msg, true)
}

fn gateway_row_keep_open<'a>(
    icon: Element<'a, Message>,
    label: String,
    mark: Option<&'static str>,
    msg: Option<Message>,
) -> Element<'a, Message> {
    gateway_row_with_menu_behavior(icon, label, mark, msg, false)
}

fn gateway_press_message(msg: Message, close_menu: bool) -> Message {
    if close_menu { Message::View(message::ViewMessage::MenuAction(Box::new(msg))) } else { msg }
}

fn gateway_row_with_menu_behavior<'a>(
    icon: Element<'a, Message>,
    label: String,
    mark: Option<&'static str>,
    msg: Option<Message>,
    close_menu: bool,
) -> Element<'a, Message> {
    let interactive = msg.is_some();
    let mark_el: Element<'a, Message> =
        mark.map(|value| text(value).size(12).into()).unwrap_or_else(|| Space::new().into());
    let content = row![
        container(icon)
            .width(Length::Fixed(12.0))
            .height(Length::Fixed(18.0))
            .align_x(iced::Alignment::Center)
            .align_y(iced::Alignment::Center),
        Space::new().width(Length::Fixed(4.0)),
        text(label).size(13),
        Space::new().width(Length::Fill),
        mark_el,
    ]
    .width(Length::Fill)
    .align_y(iced::alignment::Vertical::Center);

    let base = button(content)
        .style(move |theme: &Theme, status| {
            let palette = theme.extended_palette();
            let bg = if interactive && status == iced::widget::button::Status::Hovered {
                Some(palette.primary.base.color)
            } else {
                None
            };
            iced::widget::button::Style {
                background: bg.map(iced::Background::Color),
                text_color: if interactive && status == iced::widget::button::Status::Hovered {
                    Color::WHITE
                } else {
                    theme.palette().text
                },
                border: iced::Border { width: 0.0, color: Color::TRANSPARENT, radius: 5.0.into() },
                ..Default::default()
            }
        })
        .width(Length::Fill)
        .padding([6, 12]);

    if let Some(msg) = msg {
        base.on_press(gateway_press_message(msg, close_menu)).into()
    } else {
        base.into()
    }
}

fn gateway_menu_item_btn_keep_open<'a>(
    label: &'a str,
    msg: Option<Message>,
) -> Element<'a, Message> {
    let enabled = msg.is_some();
    let content = row![text(label).size(13), Space::new().width(Length::Fill)]
        .width(Length::Fill)
        .align_y(iced::alignment::Vertical::Center);

    let base = button(content)
        .style(move |theme: &Theme, status| {
            let palette = theme.extended_palette();
            let bg = if enabled && status == iced::widget::button::Status::Hovered {
                Some(palette.primary.base.color)
            } else {
                None
            };
            iced::widget::button::Style {
                background: bg.map(iced::Background::Color),
                text_color: if enabled && status == iced::widget::button::Status::Hovered {
                    Color::WHITE
                } else if enabled {
                    theme.palette().text
                } else {
                    theme.palette().text.scale_alpha(0.35)
                },
                border: iced::Border { width: 0.0, color: Color::TRANSPARENT, radius: 5.0.into() },
                ..Default::default()
            }
        })
        .width(Length::Fill)
        .padding([4, 12]);

    if let Some(msg) = msg { base.on_press(msg).into() } else { base.into() }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct McpServerSummary {
    name: String,
    transport: String,
    enabled: bool,
    required: bool,
}

fn compact_gateway_label(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let truncated = chars.by_ref().take(max_chars).collect::<String>();
    if chars.next().is_some() { format!("{truncated}...") } else { truncated }
}

fn mcp_server_summaries(editor_text: &str) -> Result<Vec<McpServerSummary>, String> {
    let trimmed = editor_text.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    let value = serde_json::from_str::<serde_json::Value>(trimmed)
        .map_err(|err| format!("MCP JSON 解析失败: {err}"))?;
    let servers = value.as_array().ok_or_else(|| "MCP 配置必须是 server 数组".to_string())?;

    let mut summaries = Vec::with_capacity(servers.len());
    for (index, server) in servers.iter().enumerate() {
        let Some(object) = server.as_object() else {
            summaries.push(McpServerSummary {
                name: format!("server {}", index + 1),
                transport: "invalid".to_string(),
                enabled: false,
                required: false,
            });
            continue;
        };
        let name = object
            .get("name")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("未命名")
            .to_string();
        let transport = object
            .get("type")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("stdio")
            .to_string();
        let enabled = object.get("enabled").and_then(serde_json::Value::as_bool).unwrap_or(true);
        let required = object.get("required").and_then(serde_json::Value::as_bool).unwrap_or(false);

        summaries.push(McpServerSummary { name, transport, enabled, required });
    }

    Ok(summaries)
}

fn mcp_tab_content(app: &App) -> Vec<Element<'_, Message>> {
    let mut rows: Vec<Element<'_, Message>> = Vec::new();

    if app.mcp_settings.loading {
        rows.push(gateway_row(status_dot(false), "MCP 配置加载中".to_string(), Some("..."), None));
    }
    if app.mcp_settings.saving {
        rows.push(gateway_row(status_dot(true), "MCP 配置保存中".to_string(), Some("..."), None));
    }

    if let Some(error) = &app.mcp_settings.save_error {
        rows.push(gateway_row(
            status_dot(false),
            format!("MCP 配置错误: {}", compact_gateway_label(error, 34)),
            Some("!"),
            None,
        ));
    } else {
        match mcp_server_summaries(&app.mcp_settings.editor.text()) {
            Ok(summaries) => {
                for server in &summaries {
                    let mark = if !server.enabled {
                        Some("禁用")
                    } else if server.required {
                        Some("必需")
                    } else {
                        None
                    };
                    rows.push(gateway_row(
                        status_dot(server.enabled),
                        format!(
                            "MCP  {}  {}",
                            compact_gateway_label(&server.name, 30),
                            server.transport
                        ),
                        mark,
                        None,
                    ));
                }

                if summaries.is_empty() && !app.mcp_settings.loading && !app.mcp_settings.saving {
                    rows.push(gateway_row(
                        status_dot(false),
                        "暂无 MCP 服务器".to_string(),
                        None,
                        None,
                    ));
                }
            }
            Err(error) => rows.push(gateway_row(
                status_dot(false),
                compact_gateway_label(&error, 42),
                Some("!"),
                None,
            )),
        }
    }

    if let Some(message) = &app.mcp_settings.status_message {
        rows.push(gateway_row(status_dot(true), message.clone(), None, None));
    }

    let refresh_message = (!app.mcp_settings.loading && !app.mcp_settings.saving)
        .then(|| Message::Settings(SettingsMessage::Mcp(McpMessage::Refresh)));
    let refresh_label = if app.mcp_settings.loading { "刷新中..." } else { "刷新 MCP 配置" };
    rows.push(gateway_menu_item_btn_keep_open(refresh_label, refresh_message));
    rows.push(menu_item_btn(
        "管理 MCP 配置",
        None,
        Some(Message::Batch(vec![
            Message::View(message::ViewMessage::OpenSystemSettingsTab(SystemTab::Agents)),
            Message::Settings(SettingsMessage::AgentConfigTab(AgentConfigTabMessage::Selected(
                AgentConfigTab::Mcp,
            ))),
        ])),
    ));

    rows
}

fn plugin_tab_content(app: &App) -> Vec<Element<'_, Message>> {
    let mut rows: Vec<Element<'_, Message>> = Vec::new();

    if app.skills_settings.plugin_loading && app.skills_settings.plugin_catalog.is_empty() {
        rows.push(gateway_row(status_dot(false), "插件列表加载中".to_string(), Some("…"), None));
    }

    for plugin in &app.skills_settings.plugin_catalog {
        rows.push(gateway_row_keep_open(
            status_dot(plugin.enabled),
            format!("{}  v{}", plugin.display_name, plugin.version),
            Some(if plugin.enabled { "禁用" } else { "启用" }),
            Some(Message::Settings(SettingsMessage::PluginsSetEnabledRequested {
                plugin_id: plugin.id.clone(),
                enabled: !plugin.enabled,
            })),
        ));
    }

    if app.skills_settings.plugin_catalog.is_empty() && !app.skills_settings.plugin_loading {
        rows.push(gateway_row(status_dot(false), "暂无已安装插件".to_string(), None, None));
    }

    rows.push(gateway_menu_item_btn_keep_open(
        "刷新插件状态",
        Some(Message::Settings(SettingsMessage::PluginsRefresh)),
    ));
    rows.push(menu_item_btn("管理插件", None, Some(manage_plugins_message())));
    rows
}

fn manage_plugins_message() -> Message {
    Message::Batch(vec![
        Message::View(message::ViewMessage::OpenSystemSettingsTab(SystemTab::Agents)),
        Message::Settings(SettingsMessage::AgentConfigTab(AgentConfigTabMessage::Selected(
            AgentConfigTab::Plugins,
        ))),
    ])
}

fn gateway_tab_content(app: &App) -> Vec<Element<'_, Message>> {
    let mut rows: Vec<Element<'_, Message>> = Vec::new();
    for server in &app.gateway_client_settings.servers {
        let selected = server.id == app.gateway_client_settings.selected_server_id;
        let healthy = gateway_server_healthy(app, server);
        rows.push(gateway_row(
            status_dot(healthy),
            format!("{}  {}:{}", server.name, server.host, server.port),
            selected.then_some("✓"),
            Some(Message::Settings(SettingsMessage::GatewayClient(
                GatewayClientMessage::SelectServer(server.id.clone()),
            ))),
        ));
    }

    rows.push(menu_item_btn(
        "管理客户端网关",
        None,
        Some(Message::View(message::ViewMessage::OpenSystemSettingsTab(SystemTab::GatewayClient))),
    ));
    rows.push(menu_item_btn(
        "新增网关服务",
        None,
        Some(Message::Batch(vec![
            Message::Settings(SettingsMessage::GatewayClient(GatewayClientMessage::AddServer)),
            Message::View(message::ViewMessage::OpenSystemSettingsTab(SystemTab::GatewayClient)),
        ])),
    ));
    rows
}

#[cfg(not(target_arch = "wasm32"))]
fn lsp_tab_content(app: &App) -> Vec<Element<'_, Message>> {
    let mut server_keys = app
        .preview_tabs
        .iter()
        .filter_map(|tab| tab.lsp_server_key.map(str::to_string))
        .chain(app.lsp_progress.keys().cloned())
        .collect::<Vec<_>>();
    server_keys.sort();
    server_keys.dedup();

    let mut rows = Vec::new();
    if app.lsp_disabled {
        rows.push(gateway_row(status_dot(false), "LSP 已禁用".to_string(), None, None));
        return rows;
    }

    for server_key in server_keys {
        let progress = app.lsp_progress.get(&server_key);
        let working = progress.is_some_and(|entries| !entries.is_empty());
        let detail = progress
            .and_then(|entries| entries.values().next())
            .map(|progress| {
                let percent =
                    progress.percentage.map(|value| format!(" {value}%")).unwrap_or_default();
                let message = progress
                    .message
                    .as_deref()
                    .filter(|value| !value.trim().is_empty())
                    .map(|value| format!("  {value}"))
                    .unwrap_or_default();
                format!("LSP  {}{}{}", progress.title, percent, message)
            })
            .unwrap_or_else(|| format!("LSP  {server_key} 就绪"));
        rows.push(gateway_row(status_dot(true), detail, working.then_some("…"), None));
    }

    if rows.is_empty() {
        let status = app.lsp_status.clone().unwrap_or_else(|| "LSP 尚未连接".to_string());
        rows.push(gateway_row(status_dot(false), status, None, None));
    } else if let Some(status) = &app.lsp_status {
        rows.push(gateway_row(status_dot(true), status.clone(), None, None));
    }
    rows
}

#[cfg(target_arch = "wasm32")]
fn lsp_tab_content(app: &App) -> Vec<Element<'_, Message>> {
    let connected = active_gateway_healthy(app);
    let status = if connected {
        "LSP 由网关提供".to_string()
    } else {
        "LSP 等待网关连接".to_string()
    };
    vec![gateway_row(status_dot(connected), status, None, None)]
}

pub(super) fn gateway_services_module(app: &App) -> Element<'_, Message> {
    let active = app.active_menu == Some(MenuType::GatewayServices);
    let button = gateway_services_button(active, active_gateway_healthy(app));

    let tabs = row![
        tab_label(
            "网关",
            TopBarGatewayTab::Gateway,
            app.top_bar_gateway_tab == TopBarGatewayTab::Gateway
        ),
        tab_label("MCP", TopBarGatewayTab::Mcp, app.top_bar_gateway_tab == TopBarGatewayTab::Mcp),
        tab_label("LSP", TopBarGatewayTab::Lsp, app.top_bar_gateway_tab == TopBarGatewayTab::Lsp),
        tab_label(
            "插件",
            TopBarGatewayTab::Plugins,
            app.top_bar_gateway_tab == TopBarGatewayTab::Plugins
        ),
    ]
    .spacing(18)
    .padding([2, 8]);

    let mut rows: Vec<Element<'_, Message>> = vec![tabs.into()];
    rows.extend(match app.top_bar_gateway_tab {
        TopBarGatewayTab::Gateway => gateway_tab_content(app),
        TopBarGatewayTab::Mcp => mcp_tab_content(app),
        TopBarGatewayTab::Lsp => lsp_tab_content(app),
        TopBarGatewayTab::Plugins => plugin_tab_content(app),
    });

    let content = container(column(rows).spacing(2)).width(Length::Fill);
    let menu = gateway_menu_container(content.into());

    BelowOverlay::new(button, menu)
        .show(active)
        .on_close(Message::View(message::ViewMessage::ToggleMenu(None)))
        .into()
}

#[cfg(test)]
#[path = "gateway_tests.rs"]
mod gateway_tests;
