//! 使用统计视图模块
//!
//! 本模块负责构建和渲染应用程序的使用统计界面，展示会话使用情况、
//! 资源消耗等相关信息。
//!
//! # 模块结构
//!
//! - `components`：UI 组件定义
//! - `data`：使用数据结构与转换逻辑
//! - `layout`：视图布局构建
//! - `session_menu`：会话菜单组件
//! - `steps`：步骤展示组件
//! - `styles`：样式定义
//! - `utils`：工具函数

use iced::Element;

use crate::app::{App, Message};

pub mod center;
mod data;
mod layout;
mod raw_modal;
#[cfg(test)]
mod tests;

/// 构建使用统计视图
///
/// 根据应用程序当前状态生成使用统计界面的 Element，该视图会从
/// App 中提取使用数据并构建完整的布局。
///
/// # 参数
///
/// * `app` - 应用程序状态的不可变引用，包含会话和使用统计信息
///
/// # 返回值
///
/// 返回一个 `Element`，代表使用统计视图的完整 UI 结构
///
/// # 示例
///
/// ```ignore
/// let element = view(&app);
/// // 将 element 返回给 iced 框架进行渲染
/// ```
pub fn view(app: &App) -> Element<'_, Message> {
    let usage_data = data::from_app(app);
    let content: Element<'_, Message> = if app.usage_center.open {
        center::view(app)
    } else {
        iced::widget::column![center::view(app), external_usage_entry(), layout::build_usage_view(app, &usage_data)]
            .spacing(8).into()
    };
    if let Some(modal) = raw_modal::view(app) {
        iced::widget::stack![content, modal].into()
    } else {
        content
    }
}

fn external_usage_entry() -> Element<'static, Message> {
    desktop_usage::entry::external_usage_entry().map(|navigation| {
        use crate::app::components::system_settings::SystemTab;
        let tab = match navigation {
            desktop_usage::entry::Navigation::Auth => SystemTab::Auth,
            desktop_usage::entry::Navigation::Apps => SystemTab::Apps,
        };
        Message::View(crate::app::message::ViewMessage::OpenSystemSettingsTab(tab))
    })
}

fn view_input(app: &App) -> desktop_usage::view_types::ViewInput<'_> {
    let runtime = app.current_session_runtime();
    desktop_usage::view_types::ViewInput {
        desktop_locale: app.general_settings.desktop_locale, usage: &app.chat_view.usage,
        usage_model_info: app.usage_panel.usage_model_info.as_ref(),
        auto_model: runtime.auto_model, model: runtime.model,
        active_session_id: app.session_state.active_session_id.as_ref(),
        session_menu_id: app.workspace_state.session_menu_id.as_ref(), session_menu_anchor: app.workspace_state.session_menu_anchor,
        usage_step_expanded: &app.usage_panel.usage_step_expanded,
        usage_step_raw_loading: &app.usage_panel.usage_step_raw_loading,
        usage_step_raw_payloads: &app.usage_panel.usage_step_raw_payloads,
        step_llm_raw: &app.chat_view.active_session_view_state.step_llm_raw,
        usage_step_raw_modal: app.usage_panel.usage_step_raw_modal.as_ref(),
        last_copied_code_hash: app.clipboard_state.last_copied_code_hash,
    }
}
fn map_message(event: desktop_usage::view_types::Message) -> Message {
    use desktop_usage::view_types::Message as E;
    use crate::app::message::{ProjectMessage as P, ViewMessage as V};
    match event {
        E::SessionRightClicked(id, x, y) => Message::Project(P::SessionRightClicked(id, x, y)),
        E::SessionRenamePressed(id) => Message::Project(P::SessionRenamePressed(id)),
        E::SessionCopyPressed(id) => Message::Project(P::SessionCopyPressed(id)),
        E::SessionArchivePressed(id) => Message::Project(P::SessionArchivePressed(id)),
        E::SessionDeletePressed(id) => Message::Project(P::SessionDeletePressed(id)),
        E::SessionMenuClose => Message::Project(P::SessionMenuClose),
        E::CopyCode(value) => Message::CopyCode(value),
        E::UsageTurnStepRawOpen(turn, index) => Message::View(V::UsageTurnStepRawOpen(turn, index)),
        E::UsageTurnStepToggled(turn, index) => Message::View(V::UsageTurnStepToggled(turn, index)),
        E::ReplaySessionFromSnapshot(path) => Message::View(V::ReplaySessionFromSnapshot(path)),
        E::OpenPathInFinder(path) => Message::View(V::OpenPathInFinder(path)),
        E::UsageStepRawClose => Message::View(V::UsageStepRawClose),
    }
}
