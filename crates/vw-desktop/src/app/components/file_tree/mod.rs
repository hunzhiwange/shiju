//! # 文件树组件模块
//!
//! 本模块提供了项目文件树的可视化展示功能，用于在侧边栏显示项目目录结构和文件列表。
//!
//! ## 主要功能
//!
//! - **文件树视图**：以树形结构展示项目的目录和文件
//! - **文件管理器视图**：提供更丰富的交互界面，支持切换查看全部文件或仅查看 Git 更改
//! - **查找结果集成**：与查找结果功能集成，在文件管理器中显示搜索结果
//! - **Git 更改追踪**：显示 Git 仓库中的已更改文件列表
//!
//! ## 子模块结构
//!
//! - [`find_results`]：查找结果显示功能
//! - [`icons`]：文件和文件夹图标定义
//! - [`menu`]：右键菜单功能
//! - [`tree_list`]：文件树列表构建逻辑
//! - [`widgets`]：自定义 UI 组件
//!
//! ## 使用示例
//!
//! ```ignore
//! // 在侧边栏中渲染文件树
//! let file_tree_view = file_tree::view(&app);
//! ```

use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::tooltip::{Position as TooltipPosition, Tooltip};
use iced::widget::{Space, button, column, container, row, scrollable, text};
use iced::{Background, Color, Element, Length, Theme};

use crate::app::components::status_animation::spinner_frame;
use crate::app::{
    App, Message,
    assets::Icon,
    message::{self},
};

mod find_results;
mod icons;
mod menu;
pub(crate) mod model;
mod tree_list;
mod widgets;

pub(crate) use tree_list::change_list_expand_keys;

pub(super) const FILE_MANAGER_SEARCH_ICON_SIZE: f32 = 14.0;
const FILE_MANAGER_SEARCH_BUTTON_SIZE: f32 = 28.0;
pub(super) const FILE_MANAGER_SEARCH_TAB_WIDTH: f32 = 36.0;
pub(super) const FILE_MANAGER_MODE_BAR_HEIGHT: f32 = 30.0;

#[cfg(test)]
#[path = "find_results_tests.rs"]
mod find_results_tests;
#[cfg(test)]
#[path = "icons_tests.rs"]
mod icons_tests;
#[cfg(test)]
#[path = "menu_tests.rs"]
mod menu_tests;
#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
#[cfg(test)]
#[path = "model_tests.rs"]
mod model_tests;
#[cfg(test)]
#[path = "tree_list_tests.rs"]
mod tree_list_tests;
#[cfg(test)]
#[path = "widgets_tests.rs"]
mod widgets_tests;

/// 渲染文件树视图
///
/// 创建一个简单的文件树视图，用于在侧边栏显示项目的目录结构。
/// 如果没有打开项目，则显示提示信息。
///
/// # 参数
///
/// - `app`：应用状态的不可变引用，包含项目路径等信息
///
/// # 返回值
///
/// 返回一个 Iced UI 元素，包含：
/// - 未打开项目时：显示"未打开项目"的提示文本
/// - 已打开项目时：显示可滚动的文件树列表
///
/// # 示例
///
/// ```ignore
/// let sidebar_content = file_tree::view(&app);
/// // 将 sidebar_content 添加到侧边栏容器中
/// ```
pub fn view(app: &App) -> Element<'_, Message> {
    if app.project_path.is_none() {
        column![text("未打开项目")].into()
    } else {
        let list_content = tree_list::build_file_tree_list(app);
        let list = column![list_content].spacing(2).width(Length::Fill);
        scrollable(container(list).width(Length::Fill))
            .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
            .height(Length::Fill)
            .into()
    }
}

/// 渲染文件管理器视图
///
/// 创建一个功能更丰富的文件管理器视图，提供文件树展示和 Git 更改追踪功能。
/// 该视图包含一个模式栏，允许用户在"更改"、"全部文件"和搜索之间切换。
///
/// # 参数
///
/// - `app`：应用状态的不可变引用，包含以下相关信息：
///   - `project_path`：项目路径（如果已打开）
///   - `git_changed_files`：Git 已更改的文件列表
///   - `git_changed_files_loading`：Git 更改是否正在加载中
///   - `file_manager_show_changes`：是否显示 Git 更改视图
///   - `find_results_tabs`：查找结果标签页列表
///   - `active_find_results_tab_id`：当前激活的查找结果标签页 ID
///
/// # 返回值
///
/// 返回一个 Iced UI 元素，包含以下组件（从上到下）：
/// 1. **模式栏**：在"更改"、"全部文件"和搜索之间切换
/// 2. **内容区域**：
///    - 如果有活动的查找结果标签页：显示查找结果列表
///    - 如果启用了"显示更改"：显示 Git 更改的文件列表
///    - 否则：显示完整的文件树
///
/// # 视觉样式
///
/// - 切换按钮采用主题色高亮显示活动状态
/// - 悬停和按下状态使用半透明的主题色背景
/// - 所有元素使用统一的间距和圆角边框
///
/// # 示例
///
/// ```ignore
/// // 在主界面中渲染文件管理器
/// let file_manager = file_tree::view_file_manager(&app);
/// sidebar.push(file_manager);
/// ```
pub fn view_file_manager(app: &App) -> Element<'_, Message> {
    // 工作模式直接复用本函数；构建视图只读取文件管理器状态，不改写代码模式面板开关。
    if app.project_path.is_none() {
        return column![text("未打开项目")].into();
    }

    let refresh_content = |refreshing: bool| {
        if refreshing {
            container(text(spinner_frame(app.file_manager_refresh_frame)).size(13))
                .width(Length::Fixed(16.0))
                .align_x(iced::alignment::Horizontal::Center)
        } else {
            container(icons::themed_icon_svg(Icon::ArrowRepeat))
                .width(Length::Fixed(16.0))
                .align_x(iced::alignment::Horizontal::Center)
        }
    };

    let segment_style = |active: bool| {
        move |theme: &Theme| {
            let p = theme.palette().primary;
            let background = if active {
                Some(Background::Color(Color::from_rgba(p.r, p.g, p.b, 0.10)))
            } else {
                None
            };

            iced::widget::container::Style {
                background,
                text_color: Some(theme.palette().text),
                border: iced::Border { width: 0.0, color: Color::TRANSPARENT, radius: 6.0.into() },
                ..Default::default()
            }
        }
    };

    let refresh_button = |message: Message, refreshing: bool| {
        button(
            container(refresh_content(refreshing))
                .width(Length::Fixed(24.0))
                .height(Length::Fixed(24.0))
                .align_x(iced::alignment::Horizontal::Center)
                .align_y(iced::alignment::Vertical::Center),
        )
        .on_press_maybe((!refreshing).then_some(message))
        .padding(0)
        .style(button::text)
    };

    // 构建更改按钮的标签文本
    // 根据更改文件数量和加载状态显示不同的文本
    let changes_count = app.git_changed_files.len();
    let changes_label =
        if changes_count > 0 { format!("{} 更改", changes_count) } else { "更改".to_string() };

    // 创建"更改"切换按钮
    // 点击后切换到显示 Git 更改的文件列表
    let changes_btn = button(
        container(text(changes_label).size(13))
            .width(Length::Shrink)
            .align_x(iced::alignment::Horizontal::Center),
    )
    .on_press(Message::Project(message::ProjectMessage::FileManagerShowChanges(true)))
    .style(button::text)
    .padding([6, 2])
    .width(Length::Shrink);

    let changes_refresh_btn = refresh_button(
        Message::Project(message::ProjectMessage::FileManagerRefreshChanges),
        app.file_manager_changes_refreshing,
    );

    // 创建"全部文件"切换按钮
    // 点击后切换到显示完整的文件树
    let files_btn = button(
        container(text("全部文件").size(13))
            .width(Length::Shrink)
            .align_x(iced::alignment::Horizontal::Center),
    )
    .on_press(Message::Project(message::ProjectMessage::FileManagerShowChanges(false)))
    .style(button::text)
    .padding([6, 2])
    .width(Length::Shrink);

    let files_refresh_btn = refresh_button(
        Message::Project(message::ProjectMessage::FileManagerRefreshFileTree),
        app.file_manager_file_tree_refreshing,
    );

    let searching = app.active_find_results_tab_id.is_some();
    let search_btn = button(
        container(
            icons::themed_icon_svg(Icon::Search)
                .width(Length::Fixed(FILE_MANAGER_SEARCH_ICON_SIZE))
                .height(Length::Fixed(FILE_MANAGER_SEARCH_ICON_SIZE)),
        )
        .width(Length::Fixed(FILE_MANAGER_SEARCH_BUTTON_SIZE))
        .height(Length::Fixed(FILE_MANAGER_SEARCH_BUTTON_SIZE))
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center),
    )
    .on_press(Message::Project(file_manager_search_message(app)))
    .padding(0)
    .style(button::text);
    let search_btn = Tooltip::new(
        search_btn,
        container(text("搜索").size(12)).padding([5, 8]),
        TooltipPosition::Bottom,
    )
    .gap(6);

    // 组装单层模式栏，避免文件和搜索再形成第二层标签页。
    let header = row![
        container(
            row![changes_btn, changes_refresh_btn]
                .spacing(0)
                .width(Length::Shrink)
                .align_y(iced::Alignment::Center)
        )
        .width(Length::FillPortion(1))
        .center_x(Length::Fill)
        .padding([0, 8])
        .style(segment_style(app.file_manager_show_changes && !searching)),
        container(
            row![files_btn, files_refresh_btn]
                .spacing(0)
                .width(Length::Shrink)
                .align_y(iced::Alignment::Center)
        )
        .width(Length::FillPortion(1))
        .center_x(Length::Fill)
        .padding([0, 8])
        .style(segment_style(!app.file_manager_show_changes && !searching)),
        container(search_btn)
            .width(Length::Fixed(FILE_MANAGER_SEARCH_TAB_WIDTH))
            .height(Length::Fixed(FILE_MANAGER_MODE_BAR_HEIGHT))
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center)
            .style(segment_style(searching))
    ]
    .spacing(4)
    .width(Length::Fill)
    .height(Length::Fixed(FILE_MANAGER_MODE_BAR_HEIGHT))
    .align_y(iced::Alignment::Center)
    .padding([0, 5]);

    // 根据当前状态选择要显示的内容
    // 优先级：查找结果 > Git 更改 > 完整文件树
    let list_content = if app.active_find_results_tab_id.is_some() {
        // 如果有活动的查找结果标签页，显示查找结果列表
        find_results::build_find_results_list(app)
    } else if app.file_manager_show_changes {
        // 如果启用了"显示更改"，显示 Git 更改的文件列表
        scrollable(container(tree_list::build_changes_list(app)).width(Length::Fill))
            .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
            .height(Length::Fill)
            .into()
    } else {
        // 否则显示完整的文件树
        scrollable(container(tree_list::build_file_tree_list(app)).width(Length::Fill))
            .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
            .height(Length::Fill)
            .into()
    };

    let content = container(list_content).width(Length::Fill).height(Length::Fill);

    let results_tabs: Element<'_, Message> = if app.find_results_tabs.is_empty() {
        Space::new().height(Length::Fixed(0.0)).into()
    } else {
        find_results::build_find_results_tabs(app)
    };

    // 组装完整的文件管理器界面
    // 布局顺序：模式栏 -> 搜索结果标签页 -> 内容区域
    let base: Element<'_, Message> = column![header, results_tabs, content]
        .spacing(2)
        .width(Length::Fill)
        .height(Length::Fill)
        .into();

    base
}

fn file_manager_search_message(app: &App) -> message::ProjectMessage {
    app.active_find_results_tab_id
        .as_ref()
        .filter(|active_id| {
            app.find_results_tabs.iter().any(|tab| tab.id.as_str() == active_id.as_str())
        })
        .cloned()
        .or_else(|| app.find_results_tabs.last().map(|tab| tab.id.clone()))
        .map(message::ProjectMessage::FileTreeFindTabSelected)
        .unwrap_or(message::ProjectMessage::FileTreeFindInProject)
}
