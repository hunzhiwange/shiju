//! 桌面应用顶部栏的按钮、菜单与窗口交互控件。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use super::widgets::{icon_button, icon_toggle_button};
use desktop_assets::Icon;
use super::types::WorkspaceMode;
use super::Message;
use crate::ui_types::Screen;
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
pub fn settings_button(i18n: desktop_i18n::I18n) -> Element<'static, Message> {
    icon_button(
        Icon::Gear,
        i18n.__("系统配置"),
        TooltipPosition::Bottom,
        Message::ToggleSystemSettings,
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
pub fn project_view_tools(app: &State) -> Element<'static, Message> {
    if matches!(app.screen, Screen::Project) {
        let mut tools = row![].spacing(2);
        match app.workspace_mode {
            WorkspaceMode::Code => {
                tools = tools
                    .push(icon_toggle_button(
                        Icon::LayoutSidebar,
                        desktop_i18n::I18n::new(app.desktop_locale).__("切换左侧面板"),
                        TooltipPosition::Bottom,
                        app.show_settings,
                        Message::ToggleSettingsPanel,
                    ))
                    .push(icon_toggle_button(
                        Icon::Columns,
                        desktop_i18n::I18n::new(app.desktop_locale).__("切换审查"),
                        TooltipPosition::Bottom,
                        app.show_diff,
                        Message::ToggleDiffPanel,
                    ))
                    .push(icon_toggle_button(
                        Icon::LayoutSidebarReverse,
                        desktop_i18n::I18n::new(app.desktop_locale).__("切换文件树"),
                        TooltipPosition::Bottom,
                        app.show_file_manager,
                        Message::FileManagerPanelVisible(
                            !app.show_file_manager,
                        ),
                    ));
            }
            WorkspaceMode::Work => {
                tools = tools
                    .push(icon_toggle_button(
                        Icon::LayoutSidebar,
                        desktop_i18n::I18n::new(app.desktop_locale).__("切换工作侧栏"),
                        TooltipPosition::Bottom,
                        !app.work_sidebar_collapsed,
                        Message::WorkSidebarToggled,
                    ))
                    .push(icon_toggle_button(
                        Icon::LayoutSidebarReverse,
                        desktop_i18n::I18n::new(app.desktop_locale).__("切换工作空间"),
                        TooltipPosition::Bottom,
                        app.work_resources_visible,
                        Message::WorkResourcesToggled,
                    ));
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        let tools = tools.push(icon_toggle_button(
            Icon::Terminal,
            desktop_i18n::I18n::new(app.desktop_locale).__("切换终端"),
            TooltipPosition::Bottom,
            app.terminal_visible,
            Message::ToggleTerminalPanel,
        ));

        let tools = tools.push(icon_button(
            Icon::Search,
            desktop_i18n::I18n::new(app.desktop_locale).__("搜索"),
            TooltipPosition::Bottom,
            Message::Search,
        ));

        #[cfg(not(target_arch = "wasm32"))]
        let tools = tools.push(icon_toggle_button(
            Icon::Robot,
            desktop_i18n::I18n::new(app.desktop_locale).__("切换小宠物"),
            TooltipPosition::Bottom,
            app.pet_visible,
            Message::TaskPetToggleWindow,
        ));

        tools.into()
    } else {
        Space::new().into()
    }
}

#[derive(Clone, Copy)]
pub struct State {
 pub screen: Screen,
 pub workspace_mode: WorkspaceMode,
 pub desktop_locale: config_ui::DesktopLocale,
 pub show_settings: bool,
 pub show_diff: bool,
 pub show_file_manager: bool,
 pub work_sidebar_collapsed: bool,
 pub work_resources_visible: bool,
 pub terminal_visible: bool,
 pub pet_visible: bool,
}
