//! 桌面应用顶部栏的按钮、菜单与窗口交互控件。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use super::widgets::{icon_button, icon_toggle_button};
use crate::app::assets::Icon;
use crate::app::{App, Message, Screen, message};
use iced::Element;
use iced::widget::tooltip::Position as TooltipPosition;
use iced::widget::{Space, row};

/// 构建或处理 `settings_button` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回可交给 Iced 渲染树使用的 `Element`，其中已绑定必要的消息回调。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub(super) fn settings_button() -> Element<'static, Message> {
    icon_button(
        Icon::Gear,
        "系统配置",
        TooltipPosition::Bottom,
        Message::View(message::ViewMessage::ToggleSystemSettings),
    )
}

/// 构建或处理 `project_view_tools` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回可交给 Iced 渲染树使用的 `Element`，其中已绑定必要的消息回调。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub(super) fn project_view_tools(app: &App) -> Element<'_, Message> {
    if matches!(app.screen, Screen::Project) {
        let tools = row![
            icon_toggle_button(
                Icon::LayoutSidebar,
                "切换左侧面板",
                TooltipPosition::Bottom,
                app.show_settings,
                Message::View(message::ViewMessage::ToggleSettingsPanel),
            ),
            icon_toggle_button(
                Icon::Terminal,
                "切换终端",
                TooltipPosition::Bottom,
                app.terminal.is_visible,
                Message::View(message::ViewMessage::ToggleTerminalPanel),
            ),
            icon_toggle_button(
                Icon::Columns,
                "切换审查",
                TooltipPosition::Bottom,
                app.show_diff,
                Message::View(message::ViewMessage::ToggleDiffPanel),
            ),
            icon_toggle_button(
                Icon::LayoutSidebarReverse,
                "切换文件树",
                TooltipPosition::Bottom,
                app.show_file_manager,
                Message::View(message::ViewMessage::FileManagerPanelVisible(
                    !app.show_file_manager
                )),
            ),
            icon_button(
                Icon::Search,
                "搜索",
                TooltipPosition::Bottom,
                Message::Search(message::SearchMessage::Open),
            ),
        ]
        .spacing(2);

        #[cfg(not(target_arch = "wasm32"))]
        let tools = tools.push(icon_toggle_button(
            Icon::Robot,
            "切换小宠物",
            TooltipPosition::Bottom,
            app.task_pet_window_id.is_some(),
            Message::View(message::ViewMessage::TaskPetToggleWindow),
        ));

        tools.into()
    } else {
        Space::new().into()
    }
}
