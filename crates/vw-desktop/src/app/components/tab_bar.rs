//! 标签栏组件模块
//!
//! 本模块实现了应用程序顶部的标签栏（Tab Bar）UI 组件，用于管理和显示当前打开的标签页。
//!
//! # 主要功能
//!
//! - **标签页显示**：展示所有已打开的标签页，支持点击切换活动标签
//! - **标签页关闭**：非首页标签支持悬停时显示关闭按钮
//! - **新增标签**：提供"+"按钮用于创建新标签或打开应用列表
//! - **自适应主题**：根据当前主题（亮色/暗色）自动调整样式
//! - **悬停交互**：支持鼠标悬停高亮和关闭按钮显示
//!
//! # 组件结构
//!
//! ```text
//! ┌────────────────────────────────────────────────────────┐
//! │ 🏠 │ 标签1 ╳ │ 标签2 │ 标签3 │ + │
//! └────────────────────────────────────────────────────────┘
//! ```
//!
//! # 使用示例
//!
//! ```ignore
//! use crate::app::components::tab_bar;
//!
//! // 在视图中渲染标签栏
//! let tab_bar_element = tab_bar::view(&app);
//! ```

use crate::app::assets::{self, Icon};
use crate::app::message::ViewMessage;
use crate::app::{App, Message, Screen};
use iced::widget::canvas::{Frame, Geometry, Path, Program, Stroke};
use iced::widget::svg::Svg;
use iced::widget::{MouseArea, Space, button, canvas, container, row, stack, text};
use iced::{Element, Length, Point, Rectangle, Renderer, Theme};

/// 标签栏的默认高度（像素）
///
/// 此常量定义了标签栏组件的固定高度，用于保持整个应用程序中标签栏高度的一致性。
pub const TAB_BAR_HEIGHT: f32 = 36.0;

fn is_dark_theme(theme: &Theme) -> bool {
    let base = theme.extended_palette().background.base.color;
    let luma = 0.2126 * base.r + 0.7152 * base.g + 0.0722 * base.b;
    luma < 0.5
}

fn add_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let palette = theme.extended_palette();
    let hover_bg = if is_dark_theme(theme) {
        palette.background.strong.color.scale_alpha(0.8)
    } else {
        palette.background.weak.color
    };

    iced::widget::button::Style {
        background: if status == iced::widget::button::Status::Hovered {
            Some(hover_bg.into())
        } else {
            None
        },
        text_color: theme.palette().text,
        border: iced::Border { radius: 999.0.into(), ..Default::default() },
        ..Default::default()
    }
}

fn tab_bar_style(theme: &Theme, hide_border_for_project: bool) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    let base = palette.background.base.color;
    let border_color = if is_dark_theme(theme) {
        palette.background.strong.color
    } else {
        palette.background.weak.color
    };

    iced::widget::container::Style {
        background: Some(base.into()),
        border: iced::Border {
            width: if hide_border_for_project { 0.0 } else { 1.0 },
            color: border_color,
            ..Default::default()
        },
        ..Default::default()
    }
}

fn display_tab_label(label: &str, is_home: bool) -> String {
    let max_chars = if is_home { 22 } else { 24 };
    if label.chars().count() > max_chars {
        let mut display_label: String = label.chars().take(max_chars).collect();
        display_label.push_str("...");
        display_label
    } else {
        label.to_string()
    }
}

fn tab_label_style(theme: &Theme, active: bool, hovered: bool) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    let is_dark = is_dark_theme(theme);
    let active_bg = if is_dark {
        palette.background.strong.color.scale_alpha(0.85)
    } else {
        palette.background.weak.color.scale_alpha(0.35)
    };
    let hover_bg = if is_dark {
        palette.background.weak.color.scale_alpha(0.7)
    } else {
        palette.background.weak.color.scale_alpha(0.25)
    };
    let background = if active {
        active_bg
    } else if hovered {
        hover_bg
    } else {
        iced::Color::TRANSPARENT
    };
    let text_color = if active {
        theme.palette().text
    } else {
        theme.palette().text.scale_alpha(if is_dark { 0.78 } else { 0.55 })
    };

    iced::widget::container::Style {
        background: Some(background.into()),
        text_color: Some(text_color),
        border: iced::Border {
            radius: iced::border::Radius {
                top_left: 6.0,
                top_right: 6.0,
                bottom_left: 0.0,
                bottom_right: 0.0,
            },
            ..Default::default()
        },
        ..Default::default()
    }
}

/// 渲染标签栏视图
///
/// 根据应用程序当前状态生成标签栏的 UI 元素，包括所有已打开的标签页和新增按钮。
///
/// # 参数
///
/// - `app`: 应用程序状态引用，包含打开的标签列表、活动标签 ID 等信息
///
/// # 返回值
///
/// 返回一个 `Element<Message>`，表示渲染后的标签栏 UI 组件
///
/// # 行为说明
///
/// - 过滤掉以 `"find:"` 开头的标签（这些是搜索功能的内部标签，不显示在标签栏中）
/// - 如果不存在 `"apps"` 标签，则在标签栏末尾显示一个"+"按钮
/// - 在项目屏幕下隐藏标签栏底部边框，以实现与内容区域的无缝衔接
///
/// # 示例
///
/// ```ignore
/// let tab_bar = view(&app);
/// // 将 tab_bar 添加到布局中
/// ```
pub fn view(app: &App) -> Element<'_, Message> {
    let mut tabs_row = row![];

    // 遍历所有打开的标签，过滤掉内部搜索标签
    for tab in app.open_tabs.iter().filter(|t| !t.id.starts_with("find:")) {
        let active = app.active_tab_id.as_deref() == Some(&tab.id);
        let hovered = app.hovered_tab_id.as_deref() == Some(&tab.id);
        tabs_row = tabs_row.push(tab_btn(
            tab.id.clone(),
            tab.title.clone(),
            active,
            hovered,
            Message::View(ViewMessage::TabSelected(tab.id.clone())),
        ));
    }

    // 如果没有 apps 标签，显示新增按钮
    if !app.open_tabs.iter().any(|t| t.id == "apps") {
        // 创建圆形"+"按钮
        let add_btn = container(
            button(
                container(
                    Svg::new(assets::get_icon(Icon::Plus))
                        .width(Length::Fixed(14.0))
                        .height(Length::Fixed(14.0))
                        .style(|theme: &Theme, _status| iced::widget::svg::Style {
                            color: Some(theme.palette().text),
                        }),
                )
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(iced::alignment::Horizontal::Center)
                .align_y(iced::alignment::Vertical::Center),
            )
            .on_press(Message::View(ViewMessage::OpenApps))
            .style(add_button_style)
            .padding(0)
            .width(Length::Fixed(22.0))
            .height(Length::Fixed(22.0)),
        )
        .height(Length::Fill)
        .align_y(iced::alignment::Vertical::Center);

        tabs_row = tabs_row.push(add_btn);
    }

    // 在项目屏幕下隐藏底部边框
    let hide_border_for_project = matches!(app.screen, Screen::Project);

    // 构建标签栏容器
    container(tabs_row.spacing(3).align_y(iced::Alignment::Center))
        .width(Length::Fill)
        .height(Length::Fixed(TAB_BAR_HEIGHT))
        .padding(iced::Padding { top: 4.0, right: 4.0, bottom: 0.0, left: 4.0 })
        .style(move |theme: &Theme| tab_bar_style(theme, hide_border_for_project))
        .into()
}

/// 关闭按钮角标三角形绘制器
///
/// 此结构体实现了 `Program` trait，用于在标签关闭按钮的右上角绘制一个红色三角形背景，
/// 并在其中绘制白色的"X"关闭图标。
///
/// # 视觉效果
///
/// ```text
/// ┌────────
/// │\  X   │
/// │ \     │
/// │  \    │
/// └────────┘
/// ```
///
/// 三角形位于按钮的右上角，使用红色填充（#FF3B30），中间绘制白色的叉号图标。
#[derive(Debug, Clone, Copy)]
struct CloseCornerTriangle;

impl Program<Message> for CloseCornerTriangle {
    /// 程序状态（此绘制器不需要维护状态）
    type State = ();

    /// 绘制关闭按钮角标
    ///
    /// # 参数
    ///
    /// - `_state`: 程序状态（未使用）
    /// - `renderer`: 渲染器引用，用于创建图形帧
    /// - `theme`: 当前主题（此绘制器使用固定颜色，未使用主题）
    /// - `bounds`: 绘制区域的边界矩形
    /// - `_cursor`: 鼠标光标位置（未使用）
    ///
    /// # 返回值
    ///
    /// 返回包含绘制几何体的向量
    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());

        // 边界检查：如果绘制区域无效，返回空几何体
        if bounds.width <= 0.0 || bounds.height <= 0.0 {
            return vec![frame.into_geometry()];
        }

        // 使用 iOS 系统红色作为三角形背景色
        let color = iced::Color::from_rgba8(255, 59, 48, 0.98);

        // 绘制右上角三角形路径
        let triangle = Path::new(|b| {
            b.move_to(Point::new(0.0, 0.0));
            b.line_to(Point::new(bounds.width, 0.0));
            b.line_to(Point::new(bounds.width, bounds.height));
            b.close();
        });

        frame.fill(&triangle, color);

        // 绘制白色"X"图标
        let w = bounds.width;
        let h = bounds.height;
        // X 图标中心点位置（偏向右上角）
        let cx = w * (2.0 / 3.0);
        let cy = h * (1.0 / 3.0);
        // X 图标大小，基于按钮尺寸的比例
        let s = w.min(h) * 0.18;
        let stroke = Stroke { width: 1.6, style: iced::Color::WHITE.into(), ..Stroke::default() };
        // 绘制 X 的两条对角线
        frame.stroke(&Path::line(Point::new(cx - s, cy - s), Point::new(cx + s, cy + s)), stroke);
        frame.stroke(&Path::line(Point::new(cx - s, cy + s), Point::new(cx + s, cy - s)), stroke);

        vec![frame.into_geometry()]
    }
}

/// 创建单个标签按钮元素
///
/// 根据标签的状态和属性创建一个可交互的标签按钮组件。
///
/// # 参数
///
/// - `id`: 标签的唯一标识符
/// - `label`: 标签显示的标题文本
/// - `active`: 是否为当前活动标签
/// - `hovered`: 鼠标是否悬停在此标签上
/// - `msg`: 点击标签时发送的消息
///
/// # 返回值
///
/// 返回一个 `Element<Message>`，表示渲染后的标签按钮组件
///
/// # 行为说明
///
/// - 首页标签（id == "home"）显示为房子图标，不可关闭
/// - 非首页标签在悬停时显示关闭按钮（右上角红色三角形 + X 图标）
/// - 标签标题过长时会被截断并添加省略号
/// - 活动标签和悬停标签有不同的背景色和文本颜色
///
/// # 示例
///
/// ```ignore
/// let tab = tab_btn(
///     "document-1".to_string(),
///     "我的文档.md".to_string(),
///     true,
///     false,
///     Message::View(ViewMessage::TabSelected("document-1".to_string())),
/// );
/// ```
fn tab_btn<'a>(
    id: String,
    label: String,
    active: bool,
    hovered: bool,
    msg: Message,
) -> Element<'a, Message> {
    let is_home = id == "home";
    let id_hover = id.clone();
    let close_btn_size = 18.0;
    let close_gap = 6.0;

    // 标签标题截断逻辑：首页标签最多22字符，其他标签最多24字符
    let display_label = display_tab_label(&label, is_home);

    // 关闭按钮：仅对非首页标签且悬停时显示
    let close_btn = if !is_home && hovered {
        let btn_id = id.clone();
        // 创建透明背景的按钮，覆盖整个关闭区域
        let btn = button(container(Space::new()).width(Length::Fill).height(Length::Fill))
            .on_press(Message::View(ViewMessage::TabClosed(btn_id)))
            .padding(0)
            .width(Length::Fixed(close_btn_size))
            .height(Length::Fixed(close_btn_size))
            .style(|_theme: &Theme, _status| iced::widget::button::Style {
                background: None,
                border: iced::Border::default(),
                ..Default::default()
            });

        // 将三角形背景和按钮叠加，定位到标签右上角
        Some(
            container(
                container(stack![
                    canvas(CloseCornerTriangle)
                        .width(Length::Fixed(close_btn_size))
                        .height(Length::Fixed(close_btn_size)),
                    btn
                ])
                .width(Length::Fixed(close_btn_size))
                .height(Length::Fixed(close_btn_size)),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::alignment::Horizontal::Right)
            .align_y(iced::alignment::Vertical::Top)
            .padding(iced::Padding { top: 0.0, right: 0.0, ..Default::default() }),
        )
    } else {
        None
    };

    // 标签内容：首页显示图标，其他标签显示文本
    let label_inner: Element<'_, Message> = if is_home {
        let icon = Svg::new(assets::get_icon(Icon::Home))
            .width(Length::Fixed(16.0))
            .height(Length::Fixed(16.0))
            .style(move |theme: &Theme, _status| {
                // 活动标签图标不透明，非活动标签图标半透明
                let color = if active {
                    theme.palette().text
                } else {
                    theme.palette().text.scale_alpha(0.7)
                };
                iced::widget::svg::Style { color: Some(color) }
            });
        container(icon)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center)
            .into()
    } else {
        text(display_label)
            .size(13)
            .align_y(iced::alignment::Vertical::Center)
            .align_x(iced::alignment::Horizontal::Center)
            .into()
    };

    // 标签内容容器，设置内边距和样式
    let label_content = container(label_inner)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center)
        .padding(if is_home {
            // 首页标签固定内边距
            iced::Padding { left: 18.0, right: 18.0, top: 0.0, bottom: 0.0 }
        } else {
            // 其他标签根据关闭按钮大小调整内边距，避免文字与按钮重叠
            let p = close_btn_size + close_gap;
            iced::Padding { left: p, right: p, top: 0.0, bottom: 0.0 }
        })
        .style(move |theme: &Theme| tab_label_style(theme, active, hovered));

    // 叠加标签内容和关闭按钮（如果存在）
    let stack_content = if let Some(close) = close_btn {
        stack![label_content, close]
    } else {
        stack![label_content]
    };

    // 鼠标交互区域：支持点击选中、悬停进入和离开事件
    let tab_interaction = MouseArea::new(container(stack_content).padding([0, 2]))
        .on_press(msg)
        .on_enter(Message::View(ViewMessage::TabHovered(Some(id_hover))))
        .on_exit(Message::View(ViewMessage::TabHovered(None)));

    container(tab_interaction).width(Length::Shrink).height(Length::Fill).into()
}

#[cfg(test)]
#[path = "tab_bar_tests.rs"]
mod tab_bar_tests;
