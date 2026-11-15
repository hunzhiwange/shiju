use crate::{SystemTab, SETTINGS_SCROLL_GUTTER};
use desktop_i18n::I18n;
use desktop_assets::assets::Icon;
use desktop_ui_controls::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, icon_btn, icon_svg,
    settings_muted_text_style, settings_text_input_style,
};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::text::Wrapping;
use iced::widget::{
    Space, button, column, container, row, scrollable, stack, svg, text, text_input,
};
use iced::{Alignment, Border, Element, Font, Length, Padding, Theme, font};

pub const GROUPS: &[(&str, &[SystemTab])] = {
    use SystemTab::*;
    &[
        (
            "应用与项目",
            &[
                General,
                Apps,
                #[cfg(not(target_arch = "wasm32"))]
                ComputerControl,
                #[cfg(not(target_arch = "wasm32"))]
                Auth,
                #[cfg(not(target_arch = "wasm32"))]
                Voice,
                Projects,
                Worktrees,
                ArchivedChats,
            ],
        ),
        ("模型与智能体", &[Providers, Models, Agents, Memory]),
        ("网络与集成", &[GatewayClient, Gateway, WebSearch]),
        ("运行与安全", &[Cron, Security, Autonomy, Hooks, Observability]),
    ]
};

pub struct NavigationEntry<Message> { pub label: &'static str, pub parent_label: Option<std::borrow::Cow<'static, str>>, pub tab: SystemTab, pub active: bool, pub on_press: Message }
pub struct NavigationGroup<Message> { pub label: Option<&'static str>, pub entries: Vec<NavigationEntry<Message>> }
pub const SETTINGS_NAVIGATION_SCROLL: &str = "system-settings-navigation-scroll";
pub fn view<'a, Message: Clone + 'static>(query: &'a str, i18n: I18n, width: f32, groups: Vec<NavigationGroup<Message>>, close: Message, on_query: impl Fn(String) -> Message + 'a) -> Element<'a, Message> {
    let title = row![
        icon_btn(
            Icon::ArrowLeft,
            i18n.__("返回"),
            Some(close),
        ),
        text(i18n.__("设置"))
            .size(18)
            .font(Font { weight: font::Weight::Semibold, ..Font::DEFAULT }),
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    let entries = if groups.iter().all(|group| group.entries.is_empty()) {
        column![container(text(i18n.__("没有匹配的配置")).size(12).style(settings_muted_text_style)).padding([8, 8])]
    } else {
        column(groups.into_iter().map(|group| {
            let items = column(group.entries.into_iter().map(navigation_entry)).spacing(if group.label.is_some() { 1 } else { 2 });
            if let Some(label) = group.label {
                column![container(text(i18n.__(label)).size(12).style(settings_muted_text_style)).padding([3, 8]), items].spacing(3).into()
            } else { items.into() }
        })).spacing(12)
    };

    let entries =
        scrollable(container(entries).padding([0, SETTINGS_SCROLL_GUTTER]).width(Length::Fill))
            .id(SETTINGS_NAVIGATION_SCROLL)
            .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
            .height(Length::Fill);
    container(
        column![
            container(title).padding([0, SETTINGS_SCROLL_GUTTER]),
            container(search_box(query, i18n, on_query)).padding([0, SETTINGS_SCROLL_GUTTER]),
            entries,
        ]
        .spacing(10),
    )
    .width(width)
    .height(Length::Fill)
    .padding([10, 0])
    .style(|theme: &Theme| {
        let palette = theme.palette();
        let tint = if theme.extended_palette().is_dark { 0.045 } else { 0.028 };
        container::Style {
            background: Some(palette.text.scale_alpha(tint).into()),
            ..Default::default()
        }
    })
    .into()
}

fn search_box<'a, Message: Clone + 'static>(query: &'a str, i18n: I18n, on_query: impl Fn(String) -> Message + 'a) -> Element<'a, Message> {
    let [vertical, horizontal] = SETTINGS_CONTROL_PADDING;
    let clear = on_query(String::new());
    let input = text_input(i18n.__("搜索"), query)
        .on_input(on_query)
        .padding(Padding {
            top: vertical,
            bottom: vertical,
            left: horizontal + 16.0,
            right: horizontal + if query.is_empty() { 0.0 } else { 17.0 },
        })
        .size(SETTINGS_CONTROL_TEXT_SIZE)
        .style(|theme: &Theme, status| {
            let mut style = settings_text_input_style(theme, status);
            style.border.radius = 999.0.into();
            if !matches!(status, text_input::Status::Focused { .. }) {
                let palette = theme.palette();
                style.background = palette.text.scale_alpha(0.055).into();
                style.border.width = 0.0;
            }
            style
        });
    let icon = container(icon_svg(Icon::Search, 14.0).style(|theme: &Theme, _| svg::Style {
        color: Some(theme.palette().text.scale_alpha(0.64)),
    }))
    .padding([0, 10]);
    let mut adornments = row![icon, Space::new().width(Length::Fill)]
        .height(Length::Fill)
        .align_y(Alignment::Center);
    if !query.is_empty() {
        adornments = adornments.push(
            container(icon_btn(
                Icon::X,
                i18n.__("清除搜索"),
                Some(clear),
            ))
            .padding([0, 4]),
        );
    }
    stack![input, adornments].into()
}

fn navigation_entry<Message: Clone + 'static>(entry: NavigationEntry<Message>) -> Element<'static, Message> {
    let active = entry.active;
    let mut label =
        column![text(entry.label).size(SETTINGS_CONTROL_TEXT_SIZE).wrapping(Wrapping::WordOrGlyph)]
            .spacing(2)
            .width(Length::Fill);
    if let Some(parent) = entry.parent_label.clone() {
        label = label.push(
            text(parent).size(11).wrapping(Wrapping::WordOrGlyph).style(settings_muted_text_style),
        );
    }

    let icon = icon_svg(tab_icon(entry.tab), 15.0)
        .style(|theme: &Theme, _| svg::Style { color: Some(theme.palette().text) });
    let title = row![icon, label].spacing(8).align_y(if entry.parent_label.is_some() {
        Alignment::Start
    } else {
        Alignment::Center
    });
    button(title)
        .width(Length::Fill)
        .padding([6, 8])
        .clip(true)
        .on_press(entry.on_press)
        .style(move |theme: &Theme, status| {
            let palette = theme.palette();
            let tint = match status {
                button::Status::Pressed => 0.15,
                button::Status::Hovered => 0.10,
                _ if active => 0.085,
                _ => 0.0,
            };
            button::Style {
                background: (tint > 0.0).then(|| palette.text.scale_alpha(tint).into()),
                text_color: palette.text,
                border: Border { radius: 8.0.into(), ..Default::default() },
                ..Default::default()
            }
        })
        .into()
}

fn tab_icon(tab: SystemTab) -> Icon {
    match tab {
        #[cfg(not(target_arch = "wasm32"))]
        SystemTab::Voice => Icon::Sliders,
        SystemTab::General => Icon::Gear,
        SystemTab::Themes => Icon::Palette,
        SystemTab::Editor => Icon::Code,
        SystemTab::Apps => Icon::Grid1x2,
        #[cfg(not(target_arch = "wasm32"))]
        SystemTab::ComputerControl => Icon::Sliders,
        #[cfg(not(target_arch = "wasm32"))]
        SystemTab::Auth => Icon::ShieldLock,
        SystemTab::Projects | SystemTab::Worktrees => Icon::FolderOpen,
        SystemTab::ArchivedChats => Icon::ChatTextFill,
        SystemTab::Providers => Icon::Box,
        SystemTab::Models => Icon::Hexagon,
        SystemTab::Agents => Icon::Robot,
        SystemTab::Channels => Icon::ChatTextFill,
        SystemTab::Cron => Icon::Clock,
        SystemTab::Memory => Icon::Journals,
        SystemTab::GatewayClient => Icon::Link,
        SystemTab::Gateway => Icon::HddNetwork,
        SystemTab::WebSearch | SystemTab::HttpRequest | SystemTab::Browser => Icon::Globe,
        SystemTab::Proxy | SystemTab::Tunnel => Icon::Link45Deg,
        SystemTab::Runtime => Icon::Terminal,
        SystemTab::Security => Icon::ShieldLock,
        SystemTab::Autonomy => Icon::Sliders,
        SystemTab::Hooks => Icon::GitBranch,
        SystemTab::Observability => Icon::Speedometer2,
    }
}
