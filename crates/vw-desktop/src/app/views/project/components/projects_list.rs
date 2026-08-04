//! 项目列表组件模块
//!
//! 本模块提供项目列表视图的核心 UI 组件，用于在侧边栏中显示最近打开的项目徽章按钮列表。
//! 每个项目以圆形徽章形式展示，支持自定义图标、颜色和选中/悬停状态高亮。
//!
//! # 主要功能
//!
//! - 渲染项目徽章按钮，显示项目的首字母缩写或自定义图标
//! - 支持项目选中状态、悬停状态和注意力提示（绿点）的可视化
//! - 提供"打开新项目"按钮，用于添加新的项目到列表
//! - 根据项目路径自动生成主题色和徽章标签
//!
//! # 组件层次
//!
//! ```text
//! projects_list (可滚动列表)
//! ├── project_badge_button (项目徽章按钮) × N
//! │   ├── 图标容器（图片或文字）
//! │   └── 注意力提示点（可选）
//! └── open_project_badge_button (添加项目按钮)
//! ```

use iced::widget::image::Handle as ImageHandle;
use iced::widget::tooltip::{Position as TooltipPosition, Tooltip};
use iced::widget::{Image, Space, button, column, container, mouse_area, scrollable, text};
use iced::{Background, Color, ContentFit, Element, Length, Theme};

use crate::app::components::input_panel::styles::tooltip_dark_style;
use crate::app::{Message, message};
use vw_shared::session::info as session;

use super::super::styles::project_item_button_style;
use super::super::utils::{
    contrast_text_color, lighten_color, mix_color, project_accent_color, project_badge_label,
};

/// 解析十六进制颜色字符串为 Iced Color
///
/// 将格式为 `#RRGGBB` 或 `RRGGBB` 的十六进制字符串转换为 `Color` 枚举值。
/// 该函数用于解析用户配置的项目图标颜色。
///
/// # 参数
///
/// - `input`: 十六进制颜色字符串，可带或不带 `#` 前缀
///
/// # 返回值
///
/// - `Some(Color)`: 解析成功时返回对应的颜色
/// - `None`: 输入格式无效时返回 None
///
/// # 示例
///
/// ```ignore
/// let red = parse_hex_color("#FF0000");    // Some(Color { r: 1.0, g: 0.0, b: 0.0, ... })
/// let green = parse_hex_color("00FF00");   // Some(Color { r: 0.0, g: 1.0, b: 0.0, ... })
/// let invalid = parse_hex_color("GGGGGG"); // None
/// ```
fn parse_hex_color(input: &str) -> Option<Color> {
    // 移除前后空白字符和 # 前缀
    let hex = input.trim().trim_start_matches('#');
    // 十六进制颜色必须为 6 个字符（RRGGBB）
    if hex.len() != 6 {
        return None;
    }
    // 分别解析 R、G、B 三个通道的十六进制值
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(Color::from_rgb8(r, g, b))
}

/// 根据图标路径创建图像句柄
///
/// 解析图标字符串并尝试创建 `ImageHandle`。支持 `file:///` 和 `file://` 前缀的本地文件路径，
/// 也可以接受直接的文件系统路径。
///
/// # 参数
///
/// - `icon`: 图标路径字符串，可以是以下格式：
///   - `file:///absolute/path/to/icon.png`
///   - `file://relative/path/to/icon.png`
///   - `/absolute/path/to/icon.png`
///   - `relative/path/to/icon.png`
///
/// # 返回值
///
/// - `Some(ImageHandle)`: 路径有效且文件存在时返回图像句柄
/// - `None`: 路径为空或文件不存在时返回 None
///
/// # 备注
///
/// 此函数仅检查文件是否存在，不验证文件是否为有效图像格式。
/// 图像格式的有效性由 Iced 框架在渲染时处理。
fn icon_image_handle(icon: &str) -> Option<ImageHandle> {
    let raw = icon.trim();
    // 空字符串直接返回 None
    if raw.is_empty() {
        return None;
    }
    // 尝试移除 file:/// 或 file:// 前缀，获取实际文件路径
    let path_str = raw.strip_prefix("file://").unwrap_or(raw);
    let path_str = path_str
        .strip_prefix("//")
        .map(|rest| format!("/{rest}"))
        .unwrap_or_else(|| path_str.to_string());
    let path = std::path::Path::new(&path_str);
    // 仅当文件存在时才创建句柄
    if path.exists() { Some(ImageHandle::from_path(path)) } else { None }
}

/// 创建项目徽章按钮元素
///
/// 渲染单个项目的徽章按钮，显示在项目列表中。徽章可以是：
/// - 自定义图片图标（如果配置且文件存在）
/// - 单个文字字符（来自自定义图标字符串的首字符，如果无法作为图片加载）
/// - 项目名称首字母缩写（默认行为）
///
/// # 参数
///
/// - `path`: 项目的文件系统路径，用作唯一标识符
/// - `title`: 项目的显示标题，优先使用用户编辑的名称
/// - `selected`: 是否为当前选中的项目（影响边框高亮）
/// - `has_attention`: 是否需要显示注意力提示（绿点）
/// - `is_hovered`: 鼠标是否悬停在该项目上（影响视觉反馈）
/// - `custom_icon`: 可选的自定义图标路径
/// - `custom_color`: 可选的自定义主题色
///
/// # 返回值
///
/// 返回一个 `Element<Message>`，可嵌入到 Iced 布局中
///
/// # 视觉特性
///
/// - 徽章尺寸：32×32 像素（内容），按钮总尺寸 36×36 像素
/// - 圆角半径：13 像素
/// - 选中状态：使用强调色边框
/// - 注意力提示：右上角显示绿色圆点
///
/// # 交互行为
///
/// 点击按钮会触发 `Message::Project(message::ProjectMessage::OpenRecentPressed)` 消息
pub fn project_badge_button<'a>(
    path: String,
    title: String,
    selected: bool,
    has_attention: bool,
    is_hovered: bool,
    custom_icon: Option<String>,
    custom_color: Option<Color>,
) -> Element<'a, Message> {
    // 清理自定义图标字符串，去除首尾空白
    let custom_icon_trimmed = custom_icon.as_deref().map(str::trim).filter(|v| !v.is_empty());
    // 如果自定义图标无法作为图片加载，则取首字符作为文字图标
    let text_icon = custom_icon_trimmed
        .filter(|v| icon_image_handle(v).is_none())
        .map(|v| v.chars().next().unwrap_or_default().to_string());
    // 确定徽章显示的文字标签：优先使用自定义文字图标，否则使用项目名称缩写
    let label = text_icon.unwrap_or_else(|| project_badge_label(&title));
    // 确定主题色：优先使用自定义颜色，否则根据路径自动生成
    let accent = custom_color.unwrap_or_else(|| project_accent_color(&path));
    // 生成浅色背景（用于非自定义背景场景）
    let light_bg = lighten_color(accent);
    // 定义徽章和按钮的尺寸常量
    let badge_size = 36.0;
    let button_size = 42.0;
    let badge_radius = 16.0;

    // 判断是否存在有效的图片图标
    let has_image_icon = custom_icon.as_deref().and_then(icon_image_handle).is_some();
    // 决定是否使用自定义背景色（仅当有自定义颜色但无图标时）
    let use_custom_bg = custom_color.is_some() && custom_icon_trimmed.is_none() && !has_image_icon;
    // 根据上述决策选择背景色
    let badge_bg = if use_custom_bg { accent } else { light_bg };
    // 选择文字颜色：自定义背景时使用对比色，否则使用强调色
    let badge_text = if use_custom_bg { contrast_text_color(accent) } else { accent };

    let badge_font = if label.is_ascii() {
        iced::Font { weight: iced::font::Weight::Bold, ..Default::default() }
    } else {
        iced::Font::with_name("Noto Sans CJK SC")
    };

    // 构建徽章内容：图片或文字
    let badge_content: Element<'a, Message> =
        if let Some(handle) = custom_icon.as_deref().and_then(icon_image_handle) {
            // 使用图片作为徽章内容，填充整个徽章区域
            Image::new(handle)
                .content_fit(ContentFit::Fill)
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        } else {
            // 使用文字作为徽章内容，居中显示
            container(
                text(label).size(17).font(badge_font).style(move |_theme: &Theme| {
                    iced::widget::text::Style { color: Some(badge_text) }
                }),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
        };

    // 构建徽章容器，应用样式
    let badge = container(badge_content)
        .width(Length::Fixed(badge_size))
        .height(Length::Fixed(badge_size))
        .clip(has_image_icon) // 图片图标需要裁剪以适应圆角
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center)
        .style(move |theme: &Theme| {
            let p = theme.extended_palette();
            let is_dark = super::super::styles::is_dark_theme(theme);
            let border_accent = selected || is_hovered;
            let border_width = if selected {
                2.0
            } else if is_hovered {
                1.5
            } else {
                0.0
            };
            let background = if has_image_icon {
                None
            } else if use_custom_bg {
                let highlighted_bg = if selected {
                    mix_color(accent, Color::WHITE, 0.18)
                } else if is_hovered {
                    mix_color(accent, Color::WHITE, 0.10)
                } else {
                    badge_bg
                };
                Some(Background::Color(highlighted_bg))
            } else {
                let highlighted_bg = if selected {
                    mix_color(badge_bg, accent, 0.26)
                } else if is_hovered {
                    mix_color(badge_bg, accent, 0.16)
                } else {
                    badge_bg
                };
                Some(Background::Color(highlighted_bg))
            };
            // 根据是否有图片图标选择不同的边框颜色策略
            let border_color = if has_image_icon {
                // 图片图标的边框：混合强调色或背景色
                if selected {
                    mix_color(accent, p.background.base.color, 0.22)
                } else if is_hovered {
                    mix_color(accent, p.background.base.color, 0.32)
                } else {
                    mix_color(p.background.strong.color, p.background.base.color, 0.55)
                }
            } else {
                // 文字图标的边框：使用亮色版本
                if selected {
                    mix_color(accent, Color::WHITE, 0.12)
                } else if is_hovered {
                    mix_color(accent, Color::WHITE, 0.22)
                } else {
                    lighten_color(p.background.strong.color)
                }
            };
            iced::widget::container::Style {
                background,
                border: iced::Border {
                    width: border_width,
                    color: border_color,
                    radius: badge_radius.into(),
                },
                text_color: Some(badge_text),
                shadow: if border_accent {
                    iced::Shadow {
                        color: if has_image_icon {
                            Color::BLACK.scale_alpha(if is_dark { 0.24 } else { 0.10 })
                        } else {
                            accent.scale_alpha(if selected {
                                if is_dark { 0.28 } else { 0.16 }
                            } else if is_dark {
                                0.20
                            } else {
                                0.10
                            })
                        },
                        offset: iced::Vector::new(0.0, if selected { 8.0 } else { 6.0 }),
                        blur_radius: if selected { 18.0 } else { 14.0 },
                    }
                } else {
                    iced::Shadow::default()
                },
                ..Default::default()
            }
        });

    // 如果需要显示注意力提示，在徽章右上角添加绿色圆点
    let badge_content: Element<'a, Message> = if has_attention {
        // 创建绿色注意力提示点
        let dot = container(Space::new())
            .width(Length::Fixed(8.0))
            .height(Length::Fixed(8.0))
            .style(|theme: &Theme| {
                let bg = theme.extended_palette().background.base.color;
                container::Style {
                    background: Some(Background::Color(Color::from_rgb8(34, 197, 94))),
                    border: iced::Border { width: 1.0, color: bg, radius: 999.0.into() },
                    ..Default::default()
                }
            });

        // 将提示点定位到右上角
        let dot_layer = container(dot)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::alignment::Horizontal::Right)
            .align_y(iced::alignment::Vertical::Top);

        // 使用 stack 叠加徽章和提示点
        iced::widget::stack![badge, dot_layer].into()
    } else {
        badge.into()
    };

    // 包装为按钮，绑定点击事件
    button(badge_content)
        .on_press(Message::Project(message::ProjectMessage::OpenRecentPressed(path)))
        .padding(0)
        .width(Length::Fixed(button_size))
        .height(Length::Fixed(button_size))
        .style(move |theme: &Theme, status| {
            // 悬停状态覆盖：确保视觉反馈与 is_hovered 参数一致
            let effective_status =
                if is_hovered { iced::widget::button::Status::Hovered } else { status };
            project_item_button_style(theme, selected, accent, effective_status)
        })
        .into()
}

/// 创建侧栏导航徽章按钮元素。
///
/// 渲染一个带文字标识和提示文案的按钮，用于触发代码模式侧栏导航。
/// 导航按钮显示在最近项目列表之后。
///
/// # 返回值
///
/// 返回一个 `Element<Message>`，可嵌入到 Iced 布局中
///
/// # 视觉特性
///
/// - 徽章尺寸：32×32 像素（内容），按钮总尺寸 36×36 像素
/// - 圆角半径：13 像素
/// - 显示单字符标识，字体大小 20，粗体
/// - 悬停时背景变亮，边框使用主题色（深色模式）或背景强调色（浅色模式）
/// - 悬停时在右侧显示导航目标
///
/// # 交互行为
///
/// 点击按钮会触发调用方提供的导航消息
fn navigation_badge_button<'a>(
    label: &'static str,
    tooltip: &'static str,
    message: Message,
) -> Element<'a, Message> {
    // 定义徽章和按钮的尺寸常量
    let badge_size = 36.0;
    let button_size = 42.0;

    // 构建徽章容器，显示单字符导航标识
    let badge = container(
        text(label)
            .size(20)
            .font(iced::Font { weight: iced::font::Weight::Bold, ..Default::default() }),
    )
    .width(Length::Fixed(badge_size))
    .height(Length::Fixed(badge_size))
    .align_x(iced::alignment::Horizontal::Center)
    .align_y(iced::alignment::Vertical::Center);

    // 包装为按钮，应用样式和点击事件
    let button = button(badge)
        .on_press(message)
        .padding(0)
        .width(Length::Fixed(button_size))
        .height(Length::Fixed(button_size))
        .style(|theme: &Theme, status| {
            let p = theme.extended_palette();
            let is_dark = super::super::styles::is_dark_theme(theme);
            // 根据主题计算悬停背景色
            let hover_bg = if is_dark {
                p.background.weak.color
            } else {
                mix_color(p.background.base.color, p.background.weak.color, 0.35)
            };
            // 计算悬停文字颜色
            let hover_text =
                if is_dark { p.primary.base.color } else { Color::from_rgb8(20, 20, 20) };
            // 根据按钮状态选择颜色方案
            let (bg, btn_border_color, btn_text_color) = match status {
                iced::widget::button::Status::Hovered => {
                    // 悬停状态：使用悬停背景色和主题边框
                    let border =
                        if is_dark { p.primary.base.color } else { p.background.strong.color };
                    (Some(hover_bg), border, hover_text)
                }
                iced::widget::button::Status::Pressed => {
                    // 按下状态：使用半透明背景
                    (
                        Some(p.background.strong.color.scale_alpha(0.5)),
                        p.background.strong.color.scale_alpha(0.85),
                        p.background.strong.color.scale_alpha(0.85),
                    )
                }
                _ => {
                    // 默认状态：无背景，使用亮色边框
                    (
                        Some(if is_dark {
                            p.background.base.color.scale_alpha(0.48)
                        } else {
                            Color::WHITE.scale_alpha(0.66)
                        }),
                        lighten_color(p.background.strong.color),
                        if is_dark {
                            lighten_color(p.background.strong.color)
                        } else {
                            Color::from_rgb8(20, 20, 20)
                        },
                    )
                }
            };
            iced::widget::button::Style {
                background: bg.map(Background::Color),
                text_color: btn_text_color,
                border: iced::Border { width: 1.0, color: btn_border_color, radius: 16.0.into() },
                shadow: iced::Shadow {
                    color: Color::BLACK.scale_alpha(if is_dark { 0.16 } else { 0.08 }),
                    offset: iced::Vector::new(0.0, 10.0),
                    blur_radius: 20.0,
                },
                ..Default::default()
            }
        });
    let tooltip_content =
        container(text(tooltip).size(12)).padding([6, 10]).style(tooltip_dark_style);

    Tooltip::new(button, tooltip_content, TooltipPosition::Right).gap(8.0).into()
}

pub fn open_project_badge_button<'a>() -> Element<'a, Message> {
    navigation_badge_button(
        "+",
        "打开项目",
        Message::Project(message::ProjectMessage::OpenFolderPressed),
    )
}

#[cfg(not(target_arch = "wasm32"))]
pub fn expert_hub_badge_button<'a>() -> Element<'a, Message> {
    navigation_badge_button(
        "技",
        "专家 · 技能 · 连接器",
        Message::SkillHub(crate::app::message::SkillHubMessage::Open),
    )
}

#[cfg(not(target_arch = "wasm32"))]
pub fn shiju_skills_badge_button<'a>() -> Element<'a, Message> {
    navigation_badge_button(
        "识",
        "识矩技能",
        Message::SkillHub(crate::app::message::SkillHubMessage::OpenShijuSkills),
    )
}

#[cfg(not(target_arch = "wasm32"))]
pub fn shiju_apps_badge_button<'a>() -> Element<'a, Message> {
    navigation_badge_button(
        "应",
        "识矩应用",
        Message::ShijuApps(crate::app::message::ShijuAppsMessage::Open),
    )
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProjectListAction {
    OpenProject,
    ExpertHub,
    ShijuSkills,
    ShijuApps,
}

#[cfg(not(target_arch = "wasm32"))]
const PROJECT_LIST_ACTIONS: [ProjectListAction; 4] = [
    ProjectListAction::OpenProject,
    ProjectListAction::ExpertHub,
    ProjectListAction::ShijuSkills,
    ProjectListAction::ShijuApps,
];

#[cfg(all(test, not(target_arch = "wasm32")))]
pub(crate) fn project_list_action_order() -> [&'static str; 4] {
    PROJECT_LIST_ACTIONS.map(|action| match action {
        ProjectListAction::OpenProject => "open_project",
        ProjectListAction::ExpertHub => "expert_hub",
        ProjectListAction::ShijuSkills => "shiju_skills",
        ProjectListAction::ShijuApps => "shiju_apps",
    })
}

/// 渲染完整的项目列表组件
///
/// 构建包含所有最近项目徽章按钮和"打开新项目"按钮的可滚动列表。
/// 列表垂直排列，居中对齐，支持悬停状态跟踪。
///
/// # 参数
///
/// - `app`: 应用状态引用，包含项目列表、会话状态和悬停信息
/// - `collapsed_hover_mode`: 是否为折叠悬停模式
///   - `true`: 项目悬停时不发送退出消息（适用于侧边栏折叠时的展开预览）
///   - `false`: 鼠标离开项目时发送退出悬停消息
///
/// # 返回值
///
/// 返回一个 `Element<Message>`，可嵌入到 Iced 布局中
///
/// # 内部逻辑
///
/// 1. 遍历 `app.recent_projects_edits`（用户编辑的项目名称）
/// 2. 从 `app.recent_projects` 获取对应路径
/// 3. 从 `app.recent_projects_meta` 获取自定义图标和颜色
/// 4. 检查项目是否选中（当前打开的项目）
/// 5. 检查项目是否有注意力提示（有正在进行的请求或未读成功消息）
/// 6. 检查项目是否悬停（用于高亮显示）
/// 7. 为每个项目创建徽章按钮，添加悬停事件处理
/// 8. 在列表末尾添加"打开新项目"按钮
///
/// # 注意力提示逻辑
///
/// 项目显示注意力绿点当且仅当该项目下存在会话满足以下任一条件：
/// - `is_requesting`: 正在请求中
/// - `queue` 非空：有待处理的队列
/// - `has_unseen_success`: 有未查看的成功消息
pub fn projects_list<'a>(
    app: &crate::app::App,
    collapsed_hover_mode: bool,
) -> Element<'a, Message> {
    /// 检查会话列表是否有注意力状态
    ///
    /// 遍历会话列表，检查是否存在正在请求、队列非空或有未查看成功的会话。
    ///
    /// # 参数
    ///
    /// - `app`: 应用状态引用，用于查询会话运行时状态
    /// - `sessions`: 要检查的会话信息列表
    ///
    /// # 返回值
    ///
    /// - `true`: 至少有一个会话处于注意力状态
    /// - `false`: 所有会话都无注意力状态
    fn has_attention_badge(app: &crate::app::App, sessions: &[session::Info]) -> bool {
        sessions.iter().any(|s| {
            app.session_runtime_states.get(&s.id).is_some_and(|runtime| {
                runtime.is_requesting || !runtime.queue.is_empty() || runtime.has_unseen_success
            })
        })
    }

    // 创建垂直列布局，项目间距 6 像素，水平居中
    let mut projects_col = column![].spacing(8).align_x(iced::alignment::Horizontal::Center);
    // 遍历所有最近项目
    for (i, p) in app.recent_projects_edits.iter().enumerate() {
        // 获取项目路径
        let path = app.recent_projects.get(i).cloned().unwrap_or_default();
        // 确定显示标题：优先使用编辑后的名称，否则使用路径
        let title = if p.trim().is_empty() { path.clone() } else { p.as_str().to_owned() };
        // 查找项目的元数据（图标、颜色等）
        let project_meta = app.recent_projects_meta.iter().find(|meta| meta.path == path);
        let custom_icon = project_meta.and_then(|meta| meta.icon.clone());
        let custom_color =
            project_meta.and_then(|meta| meta.icon_color.as_deref()).and_then(parse_hex_color);
        // 判断是否为当前选中的项目
        let selected = app.project_path.as_ref().is_some_and(|pp| pp == &path);
        // 判断项目是否有注意力提示
        let has_attention = app
            .project_sessions
            .get(&path)
            .map(|sessions| has_attention_badge(app, sessions))
            .unwrap_or_else(|| {
                // 如果项目没有专门的会话列表，检查当前会话列表（针对当前打开的项目）
                app.project_path.as_ref() == Some(&path) && has_attention_badge(app, &app.sessions)
            });
        // 判断鼠标是否悬停在该项目上
        let is_hovered = app.hovered_recent_project.as_ref() == Some(&path);
        // 创建项目徽章按钮
        let btn = project_badge_button(
            path.clone(),
            title.clone(),
            selected,
            has_attention,
            is_hovered,
            custom_icon,
            custom_color,
        );

        // 添加鼠标悬停事件处理
        let btn_with_hover = mouse_area(btn)
            .on_enter(Message::Project(message::ProjectMessage::RecentHovered(Some(path.clone()))))
            .on_move({
                let path = path.clone();
                move |_| {
                    Message::Project(message::ProjectMessage::RecentHovered(Some(path.clone())))
                }
            });
        // 根据模式决定是否添加退出悬停事件
        if collapsed_hover_mode {
            // 折叠悬停模式：不发送退出消息，避免频繁切换
            projects_col = projects_col.push(btn_with_hover);
        } else {
            // 正常模式：鼠标离开时清除悬停状态
            projects_col = projects_col.push(
                btn_with_hover
                    .on_exit(Message::Project(message::ProjectMessage::RecentHovered(None))),
            );
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    for action in PROJECT_LIST_ACTIONS {
        projects_col = projects_col.push(match action {
            ProjectListAction::OpenProject => open_project_badge_button(),
            ProjectListAction::ExpertHub => expert_hub_badge_button(),
            ProjectListAction::ShijuSkills => shiju_skills_badge_button(),
            ProjectListAction::ShijuApps => shiju_apps_badge_button(),
        });
    }
    #[cfg(target_arch = "wasm32")]
    {
        projects_col = projects_col.push(open_project_badge_button());
    }

    // 包装为可滚动容器，填充可用空间
    scrollable(
        container(projects_col)
            .width(Length::Fill)
            .padding([6u16, 0u16])
            .align_x(iced::alignment::Horizontal::Center),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
#[cfg(test)]
#[path = "projects_list_tests.rs"]
mod projects_list_tests;
