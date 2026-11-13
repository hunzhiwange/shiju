//! 系统编辑器设置页，复用编辑器字段。
use crate::app::components::system_settings_common::{
    settings_group_heading_with_help, settings_page_heading_with_help,
};
use crate::app::i18n::I18n;
use crate::app::message::editor::EditorMessage;
use crate::app::{App, Message};
use iced::widget::{Slider, column, responsive, slider};
use iced::{Element, Length};
use std::ops::RangeInclusive;

pub(crate) fn metrics_slider(
    range: RangeInclusive<f32>,
    value: f32,
    on_preview: impl Fn(f32) -> Message + 'static,
) -> Slider<'static, f32, Message> {
    slider(range, value, on_preview).on_release(Message::Editor(EditorMessage::CommitMetrics))
}

/// 渲染系统设置编辑器视图
///
/// 构建并返回编辑器设置界面的 UI 元素，包含主题、字体大小和行高等配置选项。
///
/// # 参数
///
/// - `app` - 应用程序状态引用，包含当前编辑器配置信息
///
/// # 返回值
///
/// 返回一个 `Element` 类型的 UI 组件，包含了所有编辑器设置的交互控件
///
/// # 示例
///
/// ```ignore
/// let view = view(&app);
/// // 返回包含主题选择器、字体大小滑块、行高滑块等控件的界面
/// ```
///
/// # UI 组件
///
/// 该视图包含以下设置项：
/// 1. 跟随系统主题 - 复选框，控制是否使用系统主题
/// 2. 编辑器主题 - 下拉列表，选择编辑器配色主题
/// 3. 字体大小 - 滑块控件，调整编辑器字体大小（10-30px）
/// 4. 行高 - 滑块控件，调整编辑器行高（10-60px）
/// 5. 自动行高 - 复选框，控制是否根据字体大小自动调整行高
/// 6. LSP 语言服务 - 复选框，控制代码补全、悬停与跳转定义
pub(crate) fn content(app: &App) -> Element<'_, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let fields = responsive(move |size| {
        super::editor_settings_fields::view(app, (size.width * 0.40).min(250.0))
    })
    .height(Length::Shrink);

    column![
        settings_group_heading_with_help(
            i18n,
            &app.app_theme,
            i18n.__("主题与显示"),
            i18n.__("统一编辑器主题来源及排版参数。")
        ),
        fields,
    ]
    .spacing(16)
    .width(Length::Fill)
    .into()
}

pub fn view(app: &App) -> Element<'_, Message> {
    let i18n = I18n::new(app.desktop_locale);
    column![
        settings_page_heading_with_help(
            i18n,
            &app.app_theme,
            i18n.__("编辑器配置"),
            i18n.__("配置主题、字号与行高等编辑体验参数。")
        ),
        content(app),
    ]
    .spacing(16)
    .into()
}
#[cfg(test)]
#[path = "system_settings_editor_tests.rs"]
mod system_settings_editor_tests;

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "system_settings_editor_ui_refresh_05_tests.rs"]
mod ui_refresh_05_tests;

#[cfg(test)]
use super::editor_settings_fields::view as editor_fields;
