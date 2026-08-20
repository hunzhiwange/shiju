//! UI 组件和样式模块
//!
//! 本模块提供应用程序视图中可复用的 UI 组件和样式函数。
//! 主要包含以下功能：
//!
//! - SVG 图标渲染工具
//! - 工具提示气泡组件
//! - 按钮样式定义（磁贴按钮、主要按钮、图标按钮、酷炫按钮）
//! - 文本输入框和文本编辑器样式
//! - 磁贴卡片组件构建器
//!
//! 这些组件基于 iced 框架构建，遵循统一的设计语言和视觉规范。

use crate::app::Message;
use crate::app::assets::{self, Icon};
use crate::app::components::system_settings_common::{
    primary_action_btn_style as settings_primary_button_style,
    rounded_action_btn_style as settings_rounded_button_style, settings_panel_style,
    settings_text_editor_style as settings_editor_style,
    settings_text_input_style as settings_input_style,
};
use iced::widget::svg::Svg;
use iced::widget::tooltip::{Position as TooltipPosition, Tooltip};
use iced::widget::{button, column, container, row, text};
use iced::{Background, Border, Color, Element, Length, Theme};

/// 创建指定大小的 SVG 图标组件
///
/// # 参数
///
/// - `icon`: 图标枚举，指定要渲染的图标类型
/// - `size`: 图标的尺寸（宽度和高度相同），单位为逻辑像素
///
/// # 返回值
///
/// 返回一个静态生命周期的 `Svg` 组件，其宽高被设置为固定值
///
/// # 示例
///
/// ```ignore
/// let icon = icon_svg(Icon::ChevronRight, 18.0);
/// ```
pub(super) fn icon_svg(icon: Icon, size: f32) -> Svg<'static> {
    Svg::new(assets::get_icon(icon)).width(Length::Fixed(size)).height(Length::Fixed(size))
}

/// 创建工具提示气泡容器（高级版本）
///
/// 接受任意 `Element` 作为内容，将其包装在一个带有深色背景、
/// 圆角边框和阴影效果的容器中。
///
/// # 参数
///
/// - `content`: 要包装的 UI 元素
///
/// # 返回值
///
/// 返回一个带有样式的 `Container` 组件
///
/// # 样式细节
///
/// - 背景色：深灰色半透明 (24, 24, 24, 0.96)
/// - 文字颜色：白色
/// - 边框：无可见边框，圆角半径 8.0
/// - 阴影：黑色 40% 透明度，向下偏移 6px，模糊半径 18px
pub(super) fn tooltip_bubble_el<'a>(
    content: Element<'a, Message>,
) -> iced::widget::Container<'a, Message> {
    container(content).padding([6, 8]).style(|_theme: &Theme| iced::widget::container::Style {
        background: Some(Color::from_rgba8(24, 24, 24, 0.96).into()),
        text_color: Some(Color::WHITE),
        border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 8.0.into() },
        shadow: iced::Shadow {
            color: Color::BLACK.scale_alpha(0.40),
            offset: iced::Vector::new(0.0, 6.0),
            blur_radius: 18.0,
        },
        snap: false,
    })
}

/// 创建文本工具提示气泡
///
/// 是 `tooltip_bubble_el` 的便捷封装，接受字符串文本并创建
/// 一个 12px 字号的工具提示气泡。
///
/// # 参数
///
/// - `tip`: 提示文本内容
///
/// # 返回值
///
/// 返回一个包含文本的样式化容器
pub(super) fn tooltip_bubble<'a>(tip: &'a str) -> iced::widget::Container<'a, Message> {
    tooltip_bubble_el(text(tip).size(12).into())
}

/// 根据操作标签获取对应的图标
///
/// 将中文操作标签映射到相应的图标枚举值，
/// 用于在按钮等组件中显示语义化的图标。
///
/// # 参数
///
/// - `label`: 操作的中文标签，如 "打开"、"删除" 等
///
/// # 返回值
///
/// 返回对应的 `Icon` 枚举值，未匹配的标签默认返回 `Icon::ChevronRight`
///
/// # 支持的标签映射
///
/// | 标签 | 图标 |
/// |------|------|
/// | 打开 | ChevronRight |
/// | 打开最近 | ArrowClockwise |
/// | 打开文件夹 | FolderOpen |
/// | 项目管理 | Gear |
/// | 添加 | Plus |
/// | 独立窗口 | Box |
/// | 浏览器 | ArrowUp |
/// | 编辑 | Pencil |
/// | 取消 | X |
/// | 删除 | Trash |
/// | 保存 | Save |
/// | 其他 | ChevronRight（默认） |
pub(super) fn action_icon(label: &str) -> Icon {
    match label {
        "打开" => Icon::ChevronRight,
        "打开最近" => Icon::ArrowClockwise,
        "打开文件夹" => Icon::FolderOpen,
        "项目管理" => Icon::Gear,
        "添加" => Icon::Plus,
        "独立窗口" => Icon::Box,
        "浏览器" => Icon::ArrowUp,
        "编辑" => Icon::Pencil,
        "取消" => Icon::X,
        "删除" => Icon::Trash,
        "保存" => Icon::Save,
        _ => Icon::ChevronRight,
    }
}

/// 酷炫图标按钮样式
///
/// 定义一种现代风格的圆形图标按钮样式，具有深色背景和微妙的阴影效果。
/// 根据按钮状态（悬停、按下、禁用）动态调整背景色和阴影。
///
/// # 参数
///
/// - `_theme`: 主题引用（当前未使用，保留以支持未来主题化）
/// - `status`: 按钮的当前交互状态
///
/// # 返回值
///
/// 返回对应状态的按钮样式配置
///
/// # 颜色方案
///
/// | 状态 | 背景色 RGBA | 说明 |
/// |------|-------------|------|
/// | 默认 | (24, 24, 24, 0.92) | 深灰半透明 |
/// | 悬停 | (34, 34, 34, 0.95) | 稍亮 |
/// | 按下 | (44, 44, 44, 0.95) | 更亮 |
/// | 禁用 | (24, 24, 24, 0.55) | 更透明 |
pub(super) fn cool_icon_button_style(
    _theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let base = Color::from_rgba8(24, 24, 24, 0.92);
    let hover = Color::from_rgba8(34, 34, 34, 0.95);
    let pressed = Color::from_rgba8(44, 44, 44, 0.95);
    let disabled = Color::from_rgba8(24, 24, 24, 0.55);

    // 根据按钮状态选择对应的背景色
    let bg = match status {
        iced::widget::button::Status::Hovered => hover,
        iced::widget::button::Status::Pressed => pressed,
        iced::widget::button::Status::Disabled => disabled,
        _ => base,
    };

    iced::widget::button::Style {
        background: Some(Background::Color(bg)),
        border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 999.0.into() },
        text_color: Color::WHITE,
        shadow: iced::Shadow {
            // 禁用状态下无阴影，其他状态有 30% 透明度的黑色阴影
            color: Color::BLACK.scale_alpha(
                if matches!(status, iced::widget::button::Status::Disabled) { 0.0 } else { 0.30 },
            ),
            offset: iced::Vector::new(0.0, 4.0),
            blur_radius: 12.0,
        },
        ..Default::default()
    }
}

/// 判断当前主题是否为深色主题
///
/// 通过计算背景色的 RGB 通道之和来判断，
/// 若总和小于 1.5 则认为是深色主题。
///
/// # 参数
///
/// - `theme`: 主题引用
///
/// # 返回值
///
/// 若为深色主题返回 `true`，否则返回 `false`
fn is_dark_theme(theme: &Theme) -> bool {
    let palette = theme.palette();
    palette.background.r + palette.background.g + palette.background.b < 1.5
}

/// 磁贴按钮样式
///
/// 定义应用磁贴卡片的按钮样式，支持深色和浅色主题自适应。
/// 具有圆角边框和动态阴影效果。
///
/// # 参数
///
/// - `theme`: 主题引用，用于获取调色板和判断主题类型
/// - `status`: 按钮的当前交互状态
///
/// # 返回值
///
/// 返回对应状态和主题的按钮样式配置
///
/// # 行为说明
///
/// - 背景色根据主题和状态动态变化
/// - 深色主题阴影更明显（35% 透明度），浅色主题较淡（10% 透明度）
/// - 边框使用背景的强调色，圆角半径 16.0
pub(super) fn tile_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let panel_style = settings_panel_style(theme);
    let dark = is_dark_theme(theme);
    let primary = theme.palette().primary;
    let base_bg = match panel_style.background {
        Some(Background::Color(color)) => color,
        _ => theme.extended_palette().background.base.color,
    };
    let border_color = match status {
        iced::widget::button::Status::Hovered => {
            primary.scale_alpha(if dark { 0.34 } else { 0.20 })
        }
        iced::widget::button::Status::Pressed => {
            primary.scale_alpha(if dark { 0.42 } else { 0.26 })
        }
        _ => panel_style.border.color,
    };
    let background = match status {
        iced::widget::button::Status::Hovered => {
            if dark {
                theme.extended_palette().background.weak.color.scale_alpha(0.28)
            } else {
                Color::WHITE.scale_alpha(0.96)
            }
        }
        iced::widget::button::Status::Pressed => {
            if dark {
                theme.extended_palette().background.strong.color.scale_alpha(0.32)
            } else {
                theme.extended_palette().background.weak.color.scale_alpha(0.92)
            }
        }
        _ => base_bg,
    };

    let shadow = iced::Shadow {
        color: Color::BLACK.scale_alpha(if dark { 0.18 } else { 0.08 }),
        offset: iced::Vector::new(0.0, 12.0),
        blur_radius: 24.0,
    };

    iced::widget::button::Style {
        background: Some(Background::Color(background)),
        border: Border {
            width: panel_style.border.width,
            color: border_color,
            radius: 20.0.into(),
        },
        shadow,
        text_color: theme.palette().text,
        ..Default::default()
    }
}

/// 主要按钮样式
///
/// 定义应用中的主要操作按钮样式，使用主题色作为背景。
/// 禁用状态下使用半透明的灰白色（深色主题）或浅黑色（浅色主题）。
///
/// # 参数
///
/// - `theme`: 主题引用
/// - `status`: 按钮的当前交互状态
///
/// # 返回值
///
/// 返回对应状态的按钮样式配置
///
/// # 行为说明
///
/// - 悬停时主题色透明度降至 90%
/// - 按下时主题色透明度降至 85%
/// - 禁用时文字颜色也会降低透明度
pub(super) fn primary_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let mut style = settings_primary_button_style(theme, status);
    style.border.radius = 12.0.into();
    style
}

/// 图标按钮样式
///
/// 定义简洁的图标按钮样式，默认无背景，
/// 仅在悬停和按下状态下显示背景色。
///
/// # 参数
///
/// - `theme`: 主题引用
/// - `status`: 按钮的当前交互状态
///
/// # 返回值
///
/// 返回对应状态的按钮样式配置
pub(super) fn icon_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let mut style = settings_rounded_button_style(theme, status);
    style.border.radius = 12.0.into();
    style
}

/// Figma 风格文本输入框样式
///
/// 定义类似 Figma 设计工具的文本输入框样式，
/// 支持深色和浅色主题自适应，聚焦时边框变为主题色。
///
/// # 参数
///
/// - `theme`: 主题引用
/// - `status`: 文本输入框的当前状态（聚焦、非聚焦等）
///
/// # 返回值
///
/// 返回对应的文本输入框样式配置
///
/// # 样式细节
///
/// - 深色主题背景：(40, 40, 40, 1.0)
/// - 浅色主题背景：(245, 246, 248, 1.0)
/// - 聚焦时边框色：主题色
/// - 非聚焦时边框色：背景强调色
/// - 选中文本背景：主题色 35% 透明度
pub(super) fn figma_text_input_style(
    theme: &Theme,
    status: iced::widget::text_input::Status,
) -> iced::widget::text_input::Style {
    settings_input_style(theme, status)
}

/// Figma 风格文本编辑器样式
///
/// 定义类似 Figma 设计工具的多行文本编辑器样式，
/// 与 `figma_text_input_style` 保持视觉一致性。
///
/// # 参数
///
/// - `theme`: 主题引用
/// - `_status`: 文本编辑器的当前状态（当前未使用）
///
/// # 返回值
///
/// 返回对应的文本编辑器样式配置
pub(super) fn figma_text_editor_style(
    theme: &Theme,
    status: iced::widget::text_editor::Status,
) -> iced::widget::text_editor::Style {
    settings_editor_style(theme, status)
}

/// 构建应用磁贴卡片组件
///
/// 创建一个包含图标、标题和操作按钮的磁贴卡片，
/// 用于在应用列表或网格中展示各个应用。
///
/// # 参数
///
/// - `icon`: 磁贴中显示的图标
/// - `title`: 磁贴的标题文本
/// - `accent`: 强调色，用于图标背景和前景色
/// - `actions`: 操作按钮列表，每个元素为 (标签, 消息) 元组
/// - `primary_action`: 点击磁贴主区域时发送的消息
///
/// # 返回值
///
/// 返回一个完整的 `Element` 组件，包含：
/// - 顶部图标（圆形背景，使用强调色）
/// - 中间标题（居中对齐，支持换行）
/// - 底部操作按钮行（带工具提示）
/// - 整体可点击，使用 `tile_button_style`
///
/// # 布局规格
///
/// - 磁贴宽度：160px
/// - 图标容器：36x36px，图标 18px
/// - 操作按钮：32x32px，图标 14px
/// - 内边距：14px
/// - 元素间距：12px
pub(super) fn tile(
    icon: Icon,
    title: String,
    accent: Color,
    actions: Vec<(&'static str, Message)>,
    primary_action: Message,
) -> Element<'static, Message> {
    // 图标背景色和前景色（基于强调色）
    let icon_bg = accent.scale_alpha(0.14);
    let icon_fg = accent;

    // 构建图标元素：圆形背景容器 + SVG 图标
    let icon_el = container(
        icon_svg(icon, 18.0)
            .style(move |_t: &Theme, _| iced::widget::svg::Style { color: Some(icon_fg) }),
    )
    .width(Length::Fixed(40.0))
    .height(Length::Fixed(40.0))
    .align_x(iced::alignment::Horizontal::Center)
    .align_y(iced::alignment::Vertical::Center)
    .style(move |_theme: &Theme| iced::widget::container::Style {
        background: Some(Background::Color(icon_bg)),
        border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 20.0.into() },
        ..Default::default()
    });

    // 构建标题元素：支持自动换行的居中文本
    let title_el = container(
        text(title.clone())
            .size(14)
            .wrapping(iced::widget::text::Wrapping::Word)
            .align_x(iced::alignment::Horizontal::Center)
            .width(Length::Fill),
    )
    .width(Length::Fill);

    // 构建操作按钮行
    let mut actions_row = row![].spacing(10).align_y(iced::Alignment::Center);

    for (label, msg) in actions {
        let icon = action_icon(label);
        let btn_size = 34.0;

        // 创建圆形图标按钮
        let btn = button(
            container(icon_svg(icon, 14.0).style(move |_t: &Theme, _| iced::widget::svg::Style {
                color: Some(Color::WHITE),
            }))
            .width(Length::Fixed(btn_size))
            .height(Length::Fixed(btn_size))
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center),
        )
        .on_press(msg)
        .style(cool_icon_button_style)
        .padding(0)
        .width(Length::Fixed(btn_size))
        .height(Length::Fixed(btn_size));

        // 为按钮添加顶部工具提示
        let btn = Tooltip::new(btn, tooltip_bubble(label), TooltipPosition::Top).gap(8.0);
        actions_row = actions_row.push(btn);
    }

    // 组装磁贴内容：垂直布局，包含图标、标题和操作按钮
    let content = column![icon_el, title_el, actions_row]
        .spacing(14)
        .width(Length::Fixed(168.0))
        .padding([18.0, 16.0])
        .align_x(iced::alignment::Horizontal::Center);

    // 将整个磁贴包装为可点击按钮
    button(content).on_press(primary_action).style(tile_button_style).padding(0).into()
}

#[cfg(test)]
#[path = "ui_tests.rs"]
mod ui_tests;
