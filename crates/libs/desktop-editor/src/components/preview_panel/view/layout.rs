use iced::{Element, Length, Padding, Theme};
#[cfg(not(target_arch = "wasm32"))]
use iced::{Background, Border};
use iced::widget::{Space, button, column, container, row, text, svg};
use std::path::Path;
use desktop_assets::assets::Icon;
use super::super::{styles::small_icon_svg, widgets::PreviewOverlayHost};
use super::controls::{overlay_icon_button_style, preview_control_tooltip, fullscreen_icon_style};
use super::presentation::*;
/// 本视图产生的导航、窗口与菜单动作；子视图消息原样上抛。
#[derive(Clone, Debug)]
pub enum ViewMessage<External> {
    External(External), OpenPathInFinder(String), ToggleHalfFullscreen, ToggleFullscreen, ContextMenuClose,
}
/// 主视图实际使用的借用投影，不拥有应用状态。
pub struct ViewInput<'a> {
    pub active_path: Option<&'a str>,
    pub project_path: Option<&'a str>,
    pub read_only_title: Option<&'a str>,
    pub show_fullscreen_controls: bool,
    pub fullscreen: bool,
    pub show_changes: bool,
    pub show_context_menu: bool,
    pub context_menu_pos: Option<(f32, f32)>,
    pub i18n: desktop_i18n::I18n,
}
/// 已有子视图的组合入口；跨域消息由桌面壳映射。
pub struct ViewParts<'a, External> {
    pub header_tabs: Element<'a, ViewMessage<External>>,
    pub header_tools: Option<Element<'a, ViewMessage<External>>>,
    pub tools: Element<'a, ViewMessage<External>>,
    pub content: Element<'a, ViewMessage<External>>,
    pub menu_ui: Element<'a, ViewMessage<External>>,
    #[cfg(not(target_arch = "wasm32"))]
    pub lsp_badge: Option<Element<'a, ViewMessage<External>>>,
}
pub fn view<'a, External: Clone + 'static>(input: ViewInput<'a>, parts: ViewParts<'a, External>) -> Element<'a, ViewMessage<External>> {
    let ViewParts { header_tabs, header_tools, tools, content, menu_ui,
        #[cfg(not(target_arch = "wasm32"))] lsp_badge } = parts;
    let header_trace_nav: Element<'a, ViewMessage<External>> = {
        let breadcrumb: Element<'a, ViewMessage<External>> = if let Some(active_path) =
            input.active_path
        {
            let display_segments =
                preview_breadcrumb_segments(Path::new(active_path), input.project_path);

            if let Some(title) = input.read_only_title {
                text(title).size(12).into()
            } else if display_segments.is_empty() {
                // Space 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                Space::new().into()
            } else {
                let mut row = row![].spacing(2).align_y(iced::Alignment::Center);
                for (i, name) in display_segments.iter().enumerate() {
                    if i > 0 {
                        row = row.push(small_icon_svg(Icon::ChevronRight));
                    }
                    let target = Path::new(active_path)
                        .ancestors()
                        .nth(display_segments.len() - i - 1)
                        .unwrap_or_else(|| Path::new(active_path));
                    row = row.push(
                        button(text(name.clone()).size(12))
                            .padding([2, 6])
                            .style(iced::widget::button::text)
                            .on_press(ViewMessage::OpenPathInFinder(
                                target.to_string_lossy().into_owned(),
                            )),
                    );
                }
                row.into()
            }
        } else {
            // Space 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            Space::new().into()
        };

        let trace_row = row![breadcrumb].spacing(6).align_y(iced::Alignment::Center);

        #[cfg(not(target_arch = "wasm32"))]
        let trace_row = if let Some(lsp_badge) = lsp_badge {
            trace_row.push(lsp_badge)
        } else {
            trace_row
        };

        let trace_row = if input.active_path.is_some() {
            trace_row.push(Space::new().width(Length::Fill)).push(tools)
        } else {
            trace_row
        };
        container(trace_row).padding([0, 4]).width(Length::Fill).into()
    };

    let content_container =
        container(content).width(Length::Fill).height(Length::Fill);
    let mut layout = if let Some(tools_left) = header_tools {
        column![container(tools_left).padding([0, 4]), content_container].spacing(6)
    } else {
        column![content_container].spacing(0)
    };
    if !input.show_changes {
        let header_row: Element<'a, ViewMessage<External>> = if input.show_fullscreen_controls {
            row![
                container(header_tabs).padding([0, 4]).width(Length::Fill),
                container(fullscreen_controls(input.fullscreen, input.i18n)).padding([0, 4]).width(Length::Shrink)
            ]
            .spacing(6)
            .align_y(iced::Alignment::Start)
            .into()
        } else {
            container(header_tabs).padding([0, 4]).width(Length::Fill).into()
        };

        layout = column![header_row, header_trace_nav, layout].spacing(0);
    }

    let menu_overlay_show = input.show_context_menu;
    let overlay_pos = if input.show_context_menu { input.context_menu_pos } else { None };

    let layout: Element<'a, ViewMessage<External>> =
        container(layout).width(Length::Fill).height(Length::Fill).padding(Padding::ZERO).into();

    let menu_host: iced::Element<'a, ViewMessage<External>> = PreviewOverlayHost::new(layout, menu_ui)
        .show(menu_overlay_show)
        .pos(overlay_pos)
        .on_close(ViewMessage::ContextMenuClose)
        .into();

    let host: iced::Element<'a, ViewMessage<External>> = menu_host;

    host
}

fn fullscreen_controls<'a, External: Clone + 'static>(fullscreen: bool, i18n: desktop_i18n::I18n) -> Element<'a, ViewMessage<External>> {
    let half_button: Element<'a, ViewMessage<External>> = preview_control_tooltip(
        button(
            svg::Svg::new(desktop_assets::assets::get_icon(Icon::LayoutTextWindow))
                .width(Length::Fixed(11.0))
                .height(Length::Fixed(11.0))
                .style(move |theme: &Theme, _status| fullscreen_icon_style(theme)),
        )
        .padding(4)
        .width(Length::Fixed(21.0))
        .height(Length::Fixed(21.0))
        .style(overlay_icon_button_style)
        .on_press(ViewMessage::ToggleHalfFullscreen)
        .into(),
        i18n.__("半屏"),
    );

    let fullscreen_icon =
        if fullscreen { Icon::FullscreenExit } else { Icon::Fullscreen };
    let fullscreen_label = if fullscreen {
        i18n.__("退出全屏")
    } else {
        i18n.__("全屏")
    };

    let fullscreen_button: Element<'a, ViewMessage<External>> = preview_control_tooltip(
        button(
            svg::Svg::new(desktop_assets::assets::get_icon(fullscreen_icon))
                .width(Length::Fixed(11.0))
                .height(Length::Fixed(11.0))
                .style(move |theme: &Theme, _status| fullscreen_icon_style(theme)),
        )
        .padding(4)
        .width(Length::Fixed(21.0))
        .height(Length::Fixed(21.0))
        .style(overlay_icon_button_style)
        .on_press(ViewMessage::ToggleFullscreen)
        .into(),
        fullscreen_label,
    );

    row![half_button, fullscreen_button].spacing(4).align_y(iced::Alignment::Center).into()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn lsp_badge<'a, M: 'a>(server: Option<&'static str>, progress: bool, language: bool, badge_theme: &Theme, i18n: desktop_i18n::I18n) -> Element<'a, M> {
    let (label, tone) = active_preview_lsp_badge_state(server, progress, language);
    let label = if label == "Plain Text" {
        i18n.__("纯文本")
    } else {
        label
    };
    let dot = active_preview_lsp_badge_dot_color(badge_theme, tone);
    let (background, text_color, border_color) =
        active_preview_lsp_badge_colors(badge_theme, tone);

    container(
            row![
                container(Space::new()).width(Length::Fixed(6.0)).height(Length::Fixed(6.0)).style(
                    move |_theme: &iced::Theme| iced::widget::container::Style {
                        // background 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                        background: Some(Background::Color(dot)),
                        // border 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                        border: Border { radius: 999.0.into(), ..Border::default() },
                        ..Default::default()
                    }
                ),
                text(label).size(11),
            ]
            .spacing(6)
            .align_y(iced::Alignment::Center),
        )
        .padding([2, 8])
        .style(move |_theme: &iced::Theme| iced::widget::container::Style {
            // text_color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            text_color: Some(text_color),
            // background 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            background: Some(Background::Color(background)),
            // border 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            border: Border { width: 1.0, color: border_color, radius: 999.0.into() },
            // shadow 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            shadow: iced::Shadow::default(),
            // snap 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            snap: false,
        })
        .into()
}
