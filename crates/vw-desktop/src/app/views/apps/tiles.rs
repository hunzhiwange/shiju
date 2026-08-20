//! # 应用瓦片模块
//!
//! 本模块负责渲染应用中心页面的瓦片网格和页面头部。
//!
//! ## 主要功能
//!
//! - `render_tiles_grid`: 渲染应用瓦片网格，包括内置工具和用户书签
//! - `render_header`: 渲染应用页面的头部，包含标题和搜索框
//!
//! ## 瓦片类型
//!
//! 模块支持以下类型的瓦片：
//! - 项目管理
//! - 网址书签（用户自定义）

use super::ui;
use crate::app::assets::Icon;
use crate::app::components::system_settings::SystemTab;
use crate::app::message::{ProjectMessage, ViewMessage};
use crate::app::{App, Message};
use iced::widget::{Space, button, column, container, row, stack, text, text_input};
use iced::{Color, Element, Length, Theme};

/// 计算瓦片网格的列数
///
/// 根据窗口宽度和瓦片尺寸自动计算可以容纳的列数，
/// 确保瓦片在不同窗口尺寸下都能正确排列。
///
/// # 参数
///
/// - `app`: 应用状态引用，用于获取窗口尺寸
/// - `tile_width`: 单个瓦片的宽度（像素）
/// - `gap`: 瓦片之间的间距（像素）
///
/// # 返回值
///
/// 返回计算得出的列数，最小值为 1
///
/// # 计算逻辑
///
/// 1. 计算可用宽度（窗口宽度减去边距 48 像素）
/// 2. 使用公式：`floor((可用宽度 + 间距) / (瓦片宽度 + 间距))`
/// 3. 确保至少返回 1 列
fn compute_tile_columns(app: &App, tile_width: f32, gap: f32) -> usize {
    // 计算可用宽度：窗口宽度减去左右边距（共 48 像素）
    let available_width = (app.window_size.0 - 48.0).max(tile_width);

    // 计算列数：考虑间距的情况下能容纳多少个瓦片
    let mut cols = ((available_width + gap) / (tile_width + gap)).floor() as usize;

    // 确保至少有一列
    if cols < 1 {
        cols = 1;
    }
    cols
}

#[cfg(test)]
fn builtin_tile_titles() -> Vec<&'static str> {
    vec!["项目管理", "添加网址", "知识库", "工作流", "任务编排"]
}

/// 渲染应用瓦片网格
///
/// 根据搜索条件过滤并渲染所有应用瓦片，包括内置工具和用户自定义的网址书签。
/// 瓦片按照计算出的列数自动排列成网格布局。
///
/// # 参数
///
/// - `app`: 应用状态引用，包含窗口尺寸、搜索查询和书签数据
/// - `blocked`: 是否处于阻塞状态。如果为 true，所有消息将被替换为 `Message::None`
///
/// # 返回值
///
/// 返回渲染后的瓦片网格 `Element`
///
/// # 瓦片搜索
///
/// 用户输入的搜索查询会与瓦片标题进行不区分大小写的匹配，
/// 只有匹配的瓦片才会显示在网格中。
///
/// # 示例
///
/// ```ignore
/// let grid = render_tiles_grid(&app, false);
/// // 返回包含所有匹配搜索条件的瓦片网格
/// ```
pub(super) fn render_tiles_grid(app: &App, blocked: bool) -> Element<'_, Message> {
    // 获取搜索查询并转为小写，用于不区分大小写的匹配
    let q = app.apps_search_query.trim().to_lowercase();

    // 定义匹配函数：如果查询为空或字符串包含查询内容则匹配
    let matches = |s: &str| q.is_empty() || s.to_lowercase().contains(&q);

    // 如果处于阻塞状态，将所有消息替换为 None
    let msg = |m: Message| if blocked { Message::None } else { m };

    // 存储所有匹配的瓦片元素
    let mut tiles: Vec<Element<'_, Message>> = Vec::new();

    // 项目管理瓦片
    if matches("项目管理") {
        tiles.push(ui::tile(
            Icon::FolderOpen,
            "项目管理".to_string(),
            Color::from_rgb8(0x94, 0x5B, 0xFF), // 紫色主题色
            vec![
                ("打开最近", msg(Message::View(ViewMessage::AppsOpenMostRecent))),
                ("打开文件夹", msg(Message::Project(ProjectMessage::OpenFolderPressed))),
                (
                    "项目管理",
                    msg(Message::View(ViewMessage::OpenSystemSettingsTab(SystemTab::Projects))),
                ),
            ],
            msg(Message::View(ViewMessage::AppsOpenMostRecent)),
        ));
    }

    // 添加网址瓦片
    if matches("添加网址") {
        tiles.push(ui::tile(
            Icon::LayoutTextWindow,
            "添加网址".to_string(),
            Color::from_rgb8(0xFF, 0x72, 0xB6), // 粉色主题色
            vec![("添加", msg(Message::View(ViewMessage::ToggleWebLinksMenu)))],
            msg(Message::View(ViewMessage::ToggleWebLinksMenu)),
        ));
    }

    // 用户自定义的网址书签瓦片
    for (idx, bm) in app.web_bookmarks.iter().enumerate() {
        // 如果标题为空，则使用 URL 作为显示标题
        let title = if bm.title.trim().is_empty() { bm.url.clone() } else { bm.title.clone() };

        // 如果标题和 URL 都不匹配搜索条件，跳过此书签
        if !matches(&title) && !matches(&bm.url) {
            continue;
        }

        // 主要操作：在独立窗口中打开网址
        let primary = msg(Message::View(ViewMessage::OpenWebUrlWithTitleAndSize(
            bm.url.clone(),
            title.clone(),
            bm.width,
            bm.height,
        )));

        // 定义三个操作按钮
        let actions = vec![
            ("独立窗口", primary.clone()),
            ("浏览器", msg(Message::View(ViewMessage::OpenUrlExternal(bm.url.clone())))),
            ("编辑", msg(Message::View(ViewMessage::WebBookmarkEditStart(idx)))),
        ];

        tiles.push(ui::tile(
            Icon::LayoutTextWindow,
            title.clone(),
            Color::from_rgb8(0x0D, 0x99, 0xFF),
            actions,
            primary,
        ));
    }

    if matches("知识库") || matches("Knowledge") {
        tiles.push(ui::tile(
            Icon::Journals,
            "知识库".to_string(),
            Color::from_rgb8(0x35, 0x85, 0xD6),
            vec![("打开", msg(Message::View(ViewMessage::OpenKnowledge)))],
            msg(Message::View(ViewMessage::OpenKnowledge)),
        ));
    }

    // Dify 工作流瓦片
    if matches("工作流") {
        tiles.push(ui::tile(
            Icon::GitBranch,
            "工作流".to_string(),
            Color::from_rgb8(0x15, 0x8E, 0xA5),
            vec![("打开", msg(Message::View(ViewMessage::OpenWorkflowTool)))],
            msg(Message::View(ViewMessage::OpenWorkflowTool)),
        ));
    }

    if matches("任务编排") {
        tiles.push(ui::tile(
            Icon::GearWideConnected,
            "任务编排".to_string(),
            Color::from_rgb8(0x28, 0x8F, 0x61),
            vec![("打开", msg(Message::View(ViewMessage::OpenWorkflowManager)))],
            msg(Message::View(ViewMessage::OpenWorkflowManager)),
        ));
    }

    // 瓦片网格布局参数
    let tile_width = 168.0; // 单个瓦片宽度
    let gap = 20.0; // 瓦片之间的间距
    let cols = compute_tile_columns(app, tile_width, gap); // 计算列数

    // 构建网格：按行列排列瓦片
    let mut grid = column![].spacing(26); // 行间距
    let mut row_acc = row![].spacing(16).align_y(iced::Alignment::Center); // 当前行累加器
    let mut cur = 0usize; // 当前列计数器

    // 将瓦片按列数分组排列
    for t in tiles.drain(..) {
        row_acc = row_acc.push(t);
        cur += 1;

        // 当前行已满，添加到网格并开始新行
        if cur == cols {
            grid = grid.push(row_acc);
            row_acc = row![].spacing(16).align_y(iced::Alignment::Center);
            cur = 0;
        }
    }

    // 添加最后一行（如果未满）
    if cur > 0 {
        grid = grid.push(row_acc);
    }

    grid.into()
}

/// 渲染应用页面的头部
///
/// 头部包含标题"应用"、搜索框和关闭按钮。
/// 搜索框支持实时过滤瓦片，关闭按钮用于关闭应用页面。
///
/// # 参数
///
/// - `app`: 应用状态引用，用于获取搜索查询内容
/// - `blocked`: 是否处于阻塞状态。如果为 true，输入和按钮将不可用
///
/// # 返回值
///
/// 返回渲染后的头部 `Element`
///
/// # UI 组件
///
/// - 标题："应用"（18px 字号）
/// - 搜索框：240px 宽度，带有清除按钮
/// - 关闭按钮：带有"关闭应用页"提示的图标按钮
///
/// # 示例
///
/// ```ignore
/// let header = render_header(&app, false);
/// // 返回包含标题、搜索框和关闭按钮的头部元素
/// ```
pub(super) fn render_header(app: &App, blocked: bool) -> Element<'static, Message> {
    // 搜索输入框
    let search_input = text_input("搜索工具、网址或动作", &app.apps_search_query)
        .on_input(move |s| {
            // 阻塞状态下不响应输入
            if blocked { Message::None } else { Message::View(ViewMessage::AppsSearchChanged(s)) }
        })
        .width(Length::Fill)
        .padding(iced::Padding { top: 10.0, right: 40.0, bottom: 10.0, left: 14.0 })
        .size(14)
        .style(ui::figma_text_input_style);

    // 清除按钮尺寸
    let clear_btn_size = 26.0;

    // 清除按钮：仅在非阻塞状态且有输入内容时显示
    let clear_btn: Element<'static, Message> = if blocked || app.apps_search_query.trim().is_empty()
    {
        // 隐藏状态：占位空间
        Space::new()
            .width(Length::Fixed(clear_btn_size))
            .height(Length::Fixed(clear_btn_size))
            .into()
    } else {
        // 显示状态：可点击的清除按钮
        button(ui::icon_svg(Icon::X, 14.0).style(|theme: &Theme, _| iced::widget::svg::Style {
            color: Some(theme.palette().text),
        }))
        .on_press(Message::View(ViewMessage::AppsSearchChanged(String::new())))
        .style(ui::icon_button_style)
        .padding(5)
        .width(Length::Fixed(clear_btn_size))
        .height(Length::Fixed(clear_btn_size))
        .into()
    };

    // 清除按钮覆盖层：定位在搜索框右侧
    let clear_overlay: Element<'static, Message> = container(clear_btn)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(iced::alignment::Horizontal::Right)
        .align_y(iced::alignment::Vertical::Center)
        .padding(iced::Padding { top: 0.0, right: 8.0, bottom: 0.0, left: 0.0 })
        .into();

    // 搜索框组合：输入框 + 清除按钮覆盖层
    let search_box: Element<'static, Message> =
        stack(vec![search_input.into(), clear_overlay]).width(Length::Fixed(280.0)).into();

    // 头部布局：标题 | 占位 | 搜索框
    row![
        column![
            text("应用中心")
                .size(20)
                .font(iced::Font { weight: iced::font::Weight::Bold, ..Default::default() }),
            text("统一浏览内置工具、网址书签与常用动作入口。").size(12).style(|theme: &Theme| {
                iced::widget::text::Style { color: Some(theme.palette().text.scale_alpha(0.66)) }
            }),
        ]
        .spacing(4),
        Space::new().width(Length::Fill),
        search_box
    ]
    .align_y(iced::Alignment::Center)
    .into()
}

#[cfg(test)]
#[path = "tiles_tests.rs"]
mod tiles_tests;
