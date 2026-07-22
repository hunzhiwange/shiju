//! 文件树右键菜单组件
//!
//! 该模块提供文件树视图的右键上下文菜单构建功能，为文件和文件夹操作提供统一的用户界面。
//!
//! # 主要功能
//!
//! - 构建文件树上下文菜单的 UI 元素
//! - 支持多种文件操作（打开、复制、粘贴、删除等）
//! - 根据上下文动态显示/隐藏菜单项
//! - 提供统一的视觉样式和交互体验
//!
//! # 菜单项包括
//!
//! - **基础操作**：打开、在 Finder 中显示、在终端中打开
//! - **对话集成**：添加到对话
//! - **搜索功能**：在文件夹中查找（可选）
//! - **剪贴板操作**：剪切、复制、粘贴（根据剪贴板状态动态显示）
//! - **路径操作**：复制路径、复制相对路径
//! - **文件管理**：重命名、删除

use iced::widget::{Space, button, column, container, text};
use iced::{Background, Color, Element, Length, Theme, Vector};

use crate::app::message::project::{FileTreeAction, ProjectMessage};
use crate::app::{App, Message};

pub(super) fn file_tree_menu_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let p = theme.extended_palette();
    let bg = match status {
        iced::widget::button::Status::Hovered => p.background.weak.color,
        iced::widget::button::Status::Pressed => p.background.strong.color,
        _ => Color::TRANSPARENT,
    };

    iced::widget::button::Style {
        background: Some(Background::Color(bg)),
        text_color: theme.palette().text,
        border: iced::Border { radius: 4.0.into(), ..iced::Border::default() },
        ..Default::default()
    }
}

pub(super) fn file_tree_menu_separator_style(theme: &Theme) -> container::Style {
    let p = theme.extended_palette();
    container::Style { background: Some(p.background.strong.color.into()), ..Default::default() }
}

pub(super) fn file_tree_menu_container_style(theme: &Theme) -> container::Style {
    let p = theme.extended_palette();
    container::Style {
        background: Some(p.background.base.color.into()),
        border: iced::Border { color: p.background.strong.color, width: 1.0, radius: 8.0.into() },
        shadow: iced::Shadow {
            color: Color::BLACK.scale_alpha(0.1),
            offset: Vector::new(0.0, 4.0),
            blur_radius: 12.0,
        },
        ..Default::default()
    }
}

/// 构建文件树右键菜单
///
/// 根据当前应用状态和配置创建一个包含多种文件操作的上下文菜单。
/// 菜单项会根据条件动态显示或隐藏，例如粘贴操作仅在剪贴板有内容时显示。
///
/// # 参数
///
/// * `app` - 应用程序状态的不可变引用，用于访问剪贴板状态等信息
/// * `include_find_in_folder` - 是否在菜单中包含"在文件夹中查找"选项
///
/// # 返回值
///
/// 返回一个 `Element<Message>` 类型的 UI 元素，表示完整的右键菜单
///
/// # 示例
///
/// ```ignore
/// // 在文件树视图的右键事件处理中
/// let menu = build_file_tree_menu(&app, true);
/// // 将菜单显示在鼠标右键位置
/// ```
///
/// # 菜单结构
///
/// 菜单按功能分组，使用分隔线进行视觉区分：
/// 1. 打开操作组
/// 2. 集成操作组
/// 3. 剪贴板操作组（根据剪贴板状态调整）
/// 4. 路径操作组
/// 5. 文件管理操作组
pub fn build_file_tree_menu<'a>(
    app: &'a App,
    include_find_in_folder: bool,
) -> Element<'a, Message> {
    // 创建菜单按钮的辅助闭包
    // 封装按钮的创建逻辑，包括样式、布局和交互行为
    let menu_btn = |label: &str, action: FileTreeAction| -> Element<'a, Message> {
        let label = label.to_string();
        button(container(text(label).size(13)).width(Length::Fill).padding([2, 8]))
            .on_press(Message::Project(ProjectMessage::FileTreeAction(action)))
            .style(file_tree_menu_button_style)
            .width(Length::Fill)
            .into()
    };

    // 创建菜单分隔线的辅助闭包
    // 使用空容器实现，高度为 1px，背景色为主题强背景色
    let separator = || -> Element<'a, Message> {
        container(Space::new())
            .width(Length::Fill)
            .height(Length::Fixed(1.0))
            .style(file_tree_menu_separator_style)
            .into()
    };

    // 构建菜单内容
    // 基础菜单项：打开、在 Finder 中显示、在终端中打开、添加到对话
    let mut content = column![
        menu_btn("打开", FileTreeAction::Open),
        separator(),
        menu_btn("在 Finder 中显示", FileTreeAction::RevealInFinder),
        menu_btn("在集成终端中打开", FileTreeAction::OpenInTerminal),
        separator(),
        menu_btn("添加到对话", FileTreeAction::AddToChat),
    ];

    // 条件添加：在文件夹中查找（仅当参数为 true 时显示）
    if include_find_in_folder {
        content = content.push(menu_btn("在文件夹中查找", FileTreeAction::FindInFolder));
    }

    // 添加剪贴板操作：剪切、复制
    content = content
        .push(separator())
        .push(menu_btn("剪切", FileTreeAction::Cut))
        .push(menu_btn("复制", FileTreeAction::Copy));

    // 条件添加：粘贴（仅当剪贴板有内容时显示）
    if app.file_tree_clipboard.is_some() {
        content = content.push(menu_btn("粘贴", FileTreeAction::Paste));
    }

    // 完成菜单构建：路径操作、重命名、删除
    let content = content
        .push(separator())
        .push(menu_btn("复制路径", FileTreeAction::CopyPath))
        .push(menu_btn("复制相对路径", FileTreeAction::CopyRelativePath))
        .push(separator())
        .push(menu_btn("重命名", FileTreeAction::Rename))
        .push(menu_btn("删除", FileTreeAction::Delete))
        .width(Length::Fixed(180.0)) // 固定菜单宽度
        .padding(2) // 内边距
        .spacing(1); // 项间距

    // 包装菜单内容，应用容器样式
    // 设置背景、边框和阴影效果
    container(content).style(file_tree_menu_container_style).into()
}
