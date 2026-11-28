//! Server list rows and their expandable inspection details.
use super::*;
use desktop_assets::assets::Icon;
use iced::alignment::Horizontal;
use iced::widget::column;
use iced::widget::text::Wrapping;

fn muted<'a>(value: impl Into<String>) -> iced::widget::Text<'a> {
    text(value.into()).size(12).style(settings_muted_text_style).wrapping(Wrapping::WordOrGlyph)
}

fn record<'a, Message: Clone + 'a>(
    identity: impl Into<Element<'a, Message>>,
    actions: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    row![
        container(identity).width(Length::FillPortion(3)),
        container(actions).align_right(Length::FillPortion(2)),
    ]
    .width(Length::Fill)
    .spacing(12)
    .align_y(Alignment::Center)
    .into()
}

pub(super) fn server_list<'a, Message: Clone + 'static>(
    locale: DesktopLocale,
    entries: impl IntoIterator<Item = Element<'a, Message>>,
    empty_label: &'static str,
) -> Element<'a, Message> {
    let mut entries = entries.into_iter().peekable();
    if entries.peek().is_none() {
        return settings_group_card(
            container(muted(tr(locale, empty_label))).center_x(Length::Fill),
        )
        .padding([28, 0])
        .into();
    }
    let mut list = column![
        container(record(muted(tr(locale, "名称")), muted(tr(locale, "操作")))).padding([12, 0])
    ];
    for entry in entries {
        list = list.push(settings_divider()).push(entry);
    }
    settings_group_card(list).into()
}

fn identity<'a, Message: Clone + 'static>(
    name: String,
    metadata: &[&str],
    status: Element<'a, Message>,
) -> Element<'a, Message> {
    column![
        text(name).size(15).wrapping(Wrapping::WordOrGlyph),
        row![muted(metadata.join(" · ")), status].spacing(10).align_y(Alignment::Center).wrap(),
    ]
    .spacing(5)
    .into()
}

fn server_entry<'a, Message: Clone + 'static>(
    locale: DesktopLocale,
    header: Element<'a, Message>,
    inspect: Option<&McpServerInspectDto>,
    expanded: bool,
    previewing: bool,
) -> Element<'a, Message> {
    let mut entry = column![header].spacing(10).width(Length::Fill);
    if expanded {
        entry = entry.push(settings_divider());
        if let Some(directory) = inspect.and_then(|entry| entry.workspace_directory.as_deref()) {
            let label = tr(locale, "检查工作区");
            entry = entry.push(muted(format!("{label} · {directory}")).width(Length::Fill));
        }
        entry = entry.push(capability_preview(locale, inspect, previewing));
    }
    container(entry).padding([12, 0]).width(Length::Fill).into()
}

fn preview_button<'a, Message: Clone + 'static>(
    event: fn(McpMessage) -> Message,
    locale: DesktopLocale,
    preview_key: String,
    expanded: bool,
    first_connection: Option<Message>,
) -> iced::widget::Button<'a, Message> {
    if !expanded && let Some(connect) = first_connection {
        return toolbar_button(tr(locale, "连接"), Some(connect));
    }
    let label = if expanded { "收起" } else { "预览" };
    toolbar_button(tr(locale, label), Some(event(McpMessage::PreviewExpandedToggled(preview_key))))
}

pub fn configured_server_panel<'a, Message: Clone + 'static>(
    event: fn(McpMessage) -> Message,
    locale: DesktopLocale,
    server: ConfiguredMcpServerRow,
    inspect: Option<&'a McpServerInspectDto>,
    preview_key: String,
    preview_expanded: bool,
    controls_enabled: bool,
    previewing: bool,
) -> Element<'a, Message> {
    configured_server_panel_with_sync(
        event, locale, server, inspect, preview_key, preview_expanded, controls_enabled,
        previewing, None,
    )
}

pub(super) fn configured_server_panel_with_sync<'a, Message: Clone + 'static>(
    event: fn(McpMessage) -> Message,
    locale: DesktopLocale,
    server: ConfiguredMcpServerRow,
    inspect: Option<&'a McpServerInspectDto>,
    preview_key: String,
    preview_expanded: bool,
    controls_enabled: bool,
    previewing: bool,
    sync: Option<Message>,
) -> Element<'a, Message> {
    let server_index = server.index;
    let builtin = config_tools::is_builtin_mcp_server_id(&server.name);
    let toggle_label = if server.enabled { "启用" } else { "禁用" };
    let mut enabled = settings_toggle(server.enabled).label(tr(locale, toggle_label)).text_size(12);
    if controls_enabled {
        enabled = enabled
            .on_toggle(move |value| event(McpMessage::ServerEnabledToggled(server_index, value)));
    }

    let mut metadata = vec![server.transport.as_str()];
    if builtin {
        metadata.push(tr(locale, "内置"));
    }
    if server.required {
        metadata.push(tr(locale, "必需"));
    }

    let mut menu = vec![(tr(locale, "编辑"), McpMessage::ServerEditRequested(server_index))];
    if !builtin {
        menu.push((tr(locale, "删除"), McpMessage::RequestDelete(server_index)));
    }
    if server.enabled && (inspect.is_some() || preview_expanded) {
        let label = if inspect.is_some() { "重新连接" } else { "连接" };
        menu.insert(0, (tr(locale, label), McpMessage::Connect(preview_key.clone())));
    }
    let labels = menu.iter().map(|(label, _)| *label).collect::<Vec<_>>();
    let menu = settings_select(0..menu.len(), None, move |index| event(menu[index].1.clone()))
        .format_labels(|index| labels[*index].to_owned())
        .menu_icon(Icon::DotsThreeHorizontal, 140.0)
        .align_menu_right()
        .width(36)
        .disabled(!controls_enabled);

    let connect = (inspect.is_none() && controls_enabled && server.enabled)
        .then(|| event(McpMessage::Connect(preview_key.clone())));
    let mut actions = row![enabled, preview_button(event, locale, preview_key, preview_expanded, connect)];
    if let Some(sync) = sync {
        actions = actions.push(toolbar_button(I18n::new(locale).__("同步到第三方"), Some(sync)));
    }
    let actions = actions.push(menu).spacing(8)
            .width(Length::Fill)
            .align_y(Alignment::Center)
            .wrap()
            .align_x(Horizontal::Right);
    let header = record(
        identity(server.name, &metadata, status_indicator(locale, inspect, server.enabled)),
        actions,
    );
    server_entry(locale, header, inspect, preview_expanded, previewing)
}

pub fn runtime_server_panel<'a, Message: Clone + 'static>(
    event: fn(McpMessage) -> Message,
    locale: DesktopLocale,
    server: &'a McpServerInspectDto,
    preview_key: String,
    preview_expanded: bool,
    controls_enabled: bool,
    previewing: bool,
) -> Element<'a, Message> {
    let toggle_label = if server.enabled { "启用" } else { "禁用" };
    let mut enabled = settings_toggle(server.enabled).label(tr(locale, toggle_label)).text_size(12);
    if controls_enabled {
        let name = server.name.clone();
        enabled = enabled.on_toggle(move |value| {
            event(McpMessage::RuntimeServerEnabledToggled(name.clone(), value))
        });
    }
    let mut metadata = vec![server.transport.as_str()];
    if server.required {
        metadata.push(tr(locale, "必需"));
    }

    let label = if previewing { "正在连接 MCP…" } else { "连接" };
    let connect = (controls_enabled && server.enabled)
        .then(|| event(McpMessage::Connect(preview_key.clone())));
    let actions = row![
        enabled,
        toolbar_button(tr(locale, label), connect),
        preview_button(event, locale, preview_key, preview_expanded, None),
    ]
    .spacing(8)
    .width(Length::Fill)
    .align_y(Alignment::Center)
    .wrap()
    .align_x(Horizontal::Right);
    let header = record(
        identity(
            server.name.clone(),
            &metadata,
            status_indicator(locale, Some(server), server.enabled),
        ),
        actions,
    );
    server_entry(locale, header, Some(server), preview_expanded, previewing)
}
