//! 预览面板视图组件。
//!
//! 本模块负责预览内容、菜单、面包屑、LSP 标识或浮层宿主的局部构建。

mod content;
/// header_tabs 子模块承载当前组件的一部分独立职责。
mod header_tabs;
/// menus 子模块承载当前组件的一部分独立职责。
mod menus;
/// settings 子模块承载当前组件的一部分独立职责。
mod settings;

#[cfg(test)]
#[path = "content_tests.rs"]
mod content_tests;
#[cfg(test)]
#[path = "header_tabs_tests.rs"]
mod header_tabs_tests;
#[cfg(test)]
#[path = "menus_tests.rs"]
mod menus_tests;
#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
#[cfg(test)]
#[path = "settings_tests.rs"]
mod settings_tests;
#[cfg(test)]
mod tests;

/// 重新导出 use crate::app::assets::Icon，让上层模块通过稳定路径访问。
use crate::app::assets::Icon;
/// 重新导出 use crate::app::components::editor_toolbar，让上层模块通过稳定路径访问。
use crate::app::components::editor_toolbar;
/// 重新导出 use crate::app::{App, Message, message}，让上层模块通过稳定路径访问。
use crate::app::{App, Message, message};
/// 重新导出 use iced::widget::svg，让上层模块通过稳定路径访问。
use iced::widget::svg;
/// 重新导出 use iced::widget::tooltip::{Position as TooltipPosition, Tooltip}，让上层模块通过稳定路径访问。
use iced::widget::tooltip::{Position as TooltipPosition, Tooltip};
/// 重新导出 use iced::widget::{Space, button, column, container, row, text}，让上层模块通过稳定路径访问。
use iced::widget::{Space, button, column, container, row, text};
/// 重新导出 use iced::{Background, Border, Color, Element, Length, Padding, Theme}，让上层模块通过稳定路径访问。
use iced::{Background, Border, Color, Element, Length, Padding, Theme};
/// 重新导出 use std::path::Path，让上层模块通过稳定路径访问。
use std::path::Path;

/// 重新导出 use super::styles::small_icon_svg，让上层模块通过稳定路径访问。
use super::styles::small_icon_svg;
/// 重新导出 use super::widgets::PreviewOverlayHost，让上层模块通过稳定路径访问。
use super::widgets::PreviewOverlayHost;

/// 重新导出 content::build_content_base，让上层模块通过稳定路径访问。
pub use content::build_content_base;
/// 重新导出 header_tabs::build_header_tabs，让上层模块通过稳定路径访问。
pub use header_tabs::build_header_tabs;
/// 重新导出 menus::build_menu_ui，让上层模块通过稳定路径访问。
pub use menus::build_menu_ui;
/// 重新导出 settings::build_settings_overlay，让上层模块通过稳定路径访问。
pub use settings::build_settings_overlay;

fn show_fullscreen_controls(app: &App) -> bool {
    matches!(app.screen, crate::app::Screen::Project)
        && matches!(app.workspace_mode, crate::app::state::WorkspaceMode::Code)
}

/// 处理 view 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 返回值是 Iced `Element`，调用方继续组合到当前界面树中。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub fn view(app: &App) -> Element<'_, Message> {
    let show_fullscreen_controls = show_fullscreen_controls(app);

    let header_tabs = build_header_tabs(app);

    let header_tools = if let Some(path) = app.active_preview_path.as_deref() {
        app.preview_tabs.iter().find(|t| t.path == path).map(|tab| {
            // editor_toolbar 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            editor_toolbar::view(
                app,
                Some(Message::Preview(message::PreviewMessage::SaveFile)),
                tab.is_dirty,
            )
        })
    } else {
        None
    };

    let header_trace_nav: Element<'_, Message> = {
        let breadcrumb: Element<'_, Message> = if let Some(active_path) =
            app.active_preview_path.as_deref()
        {
            let display_segments =
                preview_breadcrumb_segments(Path::new(active_path), app.project_path.as_deref());

            if display_segments.is_empty() {
                // Space 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                Space::new().into()
            } else {
                let mut row = row![].spacing(2).align_y(iced::Alignment::Center);
                for (i, name) in display_segments.iter().enumerate() {
                    if i > 0 {
                        row = row.push(small_icon_svg(Icon::ChevronRight));
                    }
                    row = row.push(container(text(name.clone()).size(12)).padding([2, 6]));
                }
                row.into()
            }
        } else {
            // Space 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            Space::new().into()
        };

        let trace_row = row![breadcrumb].spacing(6).align_y(iced::Alignment::Center);

        #[cfg(not(target_arch = "wasm32"))]
        let trace_row = if let Some(lsp_badge) = active_preview_lsp_badge(app) {
            trace_row.push(lsp_badge)
        } else {
            trace_row
        };

        container(trace_row).padding([0, 4]).width(Length::Fill).into()
    };

    let content_base = build_content_base(app);

    let content = if app.show_preview_settings {
        build_settings_overlay(app, content_base)
    } else {
        content_base
    };

    let menu_ui = build_menu_ui(app);

    let content_container = container(content).width(Length::Fill).height(Length::Fill);
    let mut layout = if let Some(tools_left) = header_tools {
        column![container(tools_left).padding([0, 4]), content_container].spacing(6)
    } else {
        column![content_container].spacing(0)
    };
    if !app.file_manager_show_changes {
        let header_row: Element<'_, Message> = if show_fullscreen_controls {
            row![
                container(header_tabs).padding([0, 4]).width(Length::Fill),
                container(fullscreen_controls(app)).padding([0, 4]).width(Length::Shrink)
            ]
            .spacing(6)
            .align_y(iced::Alignment::Start)
            .into()
        } else {
            container(header_tabs).padding([0, 4]).width(Length::Fill).into()
        };

        layout = column![header_row, header_trace_nav, layout].spacing(0);
    }

    let menu_overlay_show = app.show_preview_context_menu;

    #[cfg(not(target_arch = "wasm32"))]
    let lsp_overlay_show = app.active_preview_path.is_some()
        && app.lsp_overlay_path.as_ref() == app.active_preview_path.as_ref()
        && (app.lsp_overlay.hover_visible || app.lsp_overlay.completion_visible);
    #[cfg(not(target_arch = "wasm32"))]
    tracing::debug!(
        "[LSP OVERLAY] lsp_overlay_show={}, hover_visible={}, completion_visible={}, context_menu={}, nav_popup={}",
        lsp_overlay_show,
        app.lsp_overlay.hover_visible,
        app.lsp_overlay.completion_visible,
        app.show_preview_context_menu,
        app.preview_nav_popup.is_some()
    );

    let overlay_pos =
        if app.show_preview_context_menu { app.preview_context_menu_pos } else { None };

    let layout: Element<'_, Message> =
        container(layout).width(Length::Fill).height(Length::Fill).padding(Padding::ZERO).into();

    let menu_host: iced::Element<'_, Message> = PreviewOverlayHost::new(layout, menu_ui)
        .show(menu_overlay_show)
        .pos(overlay_pos)
        .on_close(Message::Preview(message::PreviewMessage::ContextMenuClose))
        .into();

    let host: iced::Element<'_, Message> = menu_host;

    host
}

/// 处理 active preview lsp badge 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// `None` 表示输入缺少必要字段、当前状态不匹配，或该视图片段不需要展示。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
#[cfg(not(target_arch = "wasm32"))]
fn active_preview_lsp_badge(app: &App) -> Option<Element<'_, Message>> {
    let path = app.active_preview_path.as_deref()?;
    let tab = app.preview_tabs.iter().find(|tab| tab.path == path)?;
    let language = crate::app::lsp::config::lsp_language_for_path(std::path::Path::new(path));
    let has_progress = tab.lsp_server_key.is_some_and(|server_key| {
        app.lsp_progress.get(server_key).is_some_and(|entries| !entries.is_empty())
    });
    let (label, tone) =
        active_preview_lsp_badge_state(tab.lsp_server_key, has_progress, language.is_some());
    let badge_theme = app.effective_editor_theme();
    let dot = active_preview_lsp_badge_dot_color(&badge_theme, tone);
    let (background, text_color, border_color) =
        active_preview_lsp_badge_colors(&badge_theme, tone);

    Some(
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
        .into(),
    )
}

/// BadgeTone 描述 mod 模块支持的离散状态。
///
/// 新增变体时需要同步检查显式分支，避免未知状态被静默吞掉。
#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum BadgeTone {
    Ready,
    Working,
    Error,
    Neutral,
}

/// 根据主题与语义状态计算 active preview lsp badge dot color。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 返回值保持当前模块的领域语义，供相邻视图或测试继续使用。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn active_preview_lsp_badge_dot_color(theme: &Theme, tone: BadgeTone) -> Color {
    let palette = theme.extended_palette();
    match tone {
        // BadgeTone 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        BadgeTone::Ready => palette.success.base.color,
        // BadgeTone 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        BadgeTone::Working => palette.warning.base.color,
        // BadgeTone 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        BadgeTone::Error => palette.danger.base.color,
        // BadgeTone 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        BadgeTone::Neutral => palette.background.strong.color,
    }
}

/// 根据主题与语义状态计算 active preview lsp badge colors。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn active_preview_lsp_badge_colors(
    theme: &Theme,
    // tone 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    tone: BadgeTone,
) -> (Color, Color, Color) {
    let palette = theme.extended_palette();
    match tone {
        // BadgeTone 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        BadgeTone::Ready => (
            palette.success.base.color.scale_alpha(0.12),
            palette.success.base.color,
            palette.success.base.color.scale_alpha(0.32),
        ),
        // BadgeTone 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        BadgeTone::Working => (
            palette.warning.base.color.scale_alpha(0.12),
            palette.warning.base.color,
            palette.warning.base.color.scale_alpha(0.32),
        ),
        // BadgeTone 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        BadgeTone::Error => (
            palette.danger.base.color.scale_alpha(0.10),
            palette.danger.base.color,
            palette.danger.base.color.scale_alpha(0.26),
        ),
        // BadgeTone 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        BadgeTone::Neutral => (
            palette.background.weak.color,
            palette.background.weak.text,
            palette.background.strong.color.scale_alpha(0.18),
        ),
    }
}

/// 处理 preview breadcrumb segments 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub(super) fn preview_breadcrumb_segments(
    active_path: &Path,
    // project_path 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    project_path: Option<&str>,
) -> Vec<String> {
    if let Some(relative) = project_relative_segments(active_path, project_path) {
        return relative;
    }

    vec![active_path.to_string_lossy().to_string()]
}

/// 处理 project relative segments 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
fn project_relative_segments(
    active_path: &Path,
    // project_path 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    project_path: Option<&str>,
) -> Option<Vec<String>> {
    let project_path = Path::new(project_path?);

    relative_segments_from_root(active_path, project_path).or_else(|| {
        let active_canonical = active_path.canonicalize().ok()?;
        let project_canonical = project_path.canonicalize().ok()?;
        relative_segments_from_root(&active_canonical, &project_canonical)
    })
}

/// 生成 segments from root，用于界面中显示更短的相对信息。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// `None` 表示输入缺少必要字段、当前状态不匹配，或该视图片段不需要展示。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
fn relative_segments_from_root(active_path: &Path, project_root: &Path) -> Option<Vec<String>> {
    let relative = active_path.strip_prefix(project_root).ok()?;
    let segments = relative
        .components()
        .map(|component| component.as_os_str().to_string_lossy().to_string())
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();

    if segments.is_empty() { None } else { Some(segments) }
}

/// 处理 active preview lsp badge state 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn active_preview_lsp_badge_state(
    lsp_server_key: Option<&'static str>,
    // has_progress 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    has_progress: bool,
    // has_language 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    has_language: bool,
) -> (&'static str, BadgeTone) {
    if lsp_server_key.is_some() {
        if has_progress { ("LSP", BadgeTone::Working) } else { ("LSP", BadgeTone::Ready) }
    } else if has_language {
        ("LSP", BadgeTone::Error)
    } else {
        ("Plain Text", BadgeTone::Neutral)
    }
}

/// 构建或定位 overlay icon button style，用于把浮层稳定附着到目标控件。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 返回值保持当前模块的领域语义，供相邻视图或测试继续使用。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
fn overlay_icon_button_style(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();
    let primary = palette.primary.base.color;
    let bg = match status {
        // button 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        button::Status::Hovered => {
            Some(Background::Color(Color::from_rgba(primary.r, primary.g, primary.b, 0.12)))
        }
        button::Status::Pressed => {
            Some(Background::Color(Color::from_rgba(primary.r, primary.g, primary.b, 0.20)))
        }
        _ => None,
    };
    let border_color = match status {
        // button 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        button::Status::Hovered => Color::from_rgba(primary.r, primary.g, primary.b, 0.28),
        // button 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        button::Status::Pressed => Color::from_rgba(primary.r, primary.g, primary.b, 0.36),
        _ => Color::TRANSPARENT,
    };

    button::Style {
        // background 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        background: bg,
        // text_color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        text_color: theme.palette().text,
        // border 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        border: Border { width: 1.0, color: border_color, radius: 6.0.into() },
        // shadow 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        shadow: iced::Shadow::default(),
        ..Default::default()
    }
}

/// 构建或定位 fullscreen button overlay，用于把浮层稳定附着到目标控件。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 返回值是 Iced `Element`，调用方继续组合到当前界面树中。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
fn fullscreen_controls<'a>(app: &'a App) -> Element<'a, Message> {
    let half_button: Element<'a, Message> = fullscreen_control_tooltip(
        button(
            svg::Svg::new(crate::app::assets::get_icon(Icon::LayoutTextWindow))
                .width(Length::Fixed(11.0))
                .height(Length::Fixed(11.0))
                .style(move |theme: &Theme, _status| fullscreen_icon_style(theme)),
        )
        .padding(4)
        .width(Length::Fixed(21.0))
        .height(Length::Fixed(21.0))
        .style(overlay_icon_button_style)
        .on_press(Message::Git(message::GitMessage::ToggleHalfFullscreen))
        .into(),
        "半屏",
    );

    let fullscreen_icon =
        if app.git_diff_fullscreen { Icon::FullscreenExit } else { Icon::Fullscreen };
    let fullscreen_label = if app.git_diff_fullscreen { "退出全屏" } else { "全屏" };

    let fullscreen_button: Element<'a, Message> = fullscreen_control_tooltip(
        button(
            svg::Svg::new(crate::app::assets::get_icon(fullscreen_icon))
                .width(Length::Fixed(11.0))
                .height(Length::Fixed(11.0))
                .style(move |theme: &Theme, _status| fullscreen_icon_style(theme)),
        )
        .padding(4)
        .width(Length::Fixed(21.0))
        .height(Length::Fixed(21.0))
        .style(overlay_icon_button_style)
        .on_press(Message::Git(message::GitMessage::ToggleFullscreen))
        .into(),
        fullscreen_label,
    );

    row![half_button, fullscreen_button].spacing(4).align_y(iced::Alignment::Center).into()
}

fn fullscreen_control_tooltip<'a>(
    content: Element<'a, Message>,
    label: &'a str,
) -> Element<'a, Message> {
    let tip_content =
        container(text(label).size(12)).padding([4, 8]).style(fullscreen_tooltip_style);

    Tooltip::new(content, tip_content, TooltipPosition::Bottom).gap(6.0).into()
}

fn fullscreen_tooltip_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    let is_dark =
        theme.palette().background.r + theme.palette().background.g + theme.palette().background.b
            < 1.5;

    if is_dark {
        iced::widget::container::Style {
            text_color: Some(palette.background.strong.text),
            background: Some(Background::Color(palette.background.base.color)),
            border: Border {
                width: 1.0,
                color: palette.background.strong.color,
                radius: 4.0.into(),
            },
            shadow: iced::Shadow::default(),
            snap: false,
        }
    } else {
        iced::widget::container::Style {
            text_color: Some(Color::WHITE),
            background: Some(Background::Color(Color::from_rgba8(12, 13, 15, 0.97))),
            border: Border {
                width: 1.0,
                color: Color::from_rgba8(255, 255, 255, 0.08),
                radius: 10.0.into(),
            },
            shadow: iced::Shadow {
                color: Color::BLACK.scale_alpha(0.32),
                offset: iced::Vector::new(0.0, 6.0),
                blur_radius: 20.0,
            },
            snap: false,
        }
    }
}

fn fullscreen_icon_style(theme: &Theme) -> svg::Style {
    let bg = theme.palette().background;
    let is_dark = bg.r + bg.g + bg.b < 1.5;
    let palette = theme.extended_palette();
    svg::Style {
        color: Some(if is_dark {
            palette.background.strong.text.scale_alpha(0.88)
        } else {
            Color::from_rgba8(30, 41, 59, 0.78)
        }),
    }
}
