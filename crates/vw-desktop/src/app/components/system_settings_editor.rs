//! 系统设置编辑器组件
//!
//! 本模块提供编辑器相关设置的用户界面组件，用于配置编辑器的显示和行为参数。
//! 主要功能包括：
//! - 主题选择：支持跟随系统主题或自定义编辑器主题
//! - 字体大小调整：提供滑块控制编辑器字体大小
//! - 行高配置：支持手动调整行高或自动根据字体大小调整
//!
//! # 示例
//!
//! ```ignore
//! use crate::app::components::system_settings_editor;
//! let settings_view = system_settings_editor::view(&app);
//! ```

use crate::app::components::system_settings::general_fields::{self, GeneralSettingField};
use crate::app::components::system_settings_common::{
    settings_checkbox_style, settings_divider, settings_flexible_row_with_help,
    settings_group_card, settings_group_heading_with_help, settings_page_heading_with_help,
    settings_row_layout, settings_select, settings_value_badge,
};
use crate::app::i18n::I18n;
use crate::app::message::editor::{EditorMessage, metrics};
use crate::app::{App, Message, message};
use iced::widget::{Slider, checkbox, column, container, responsive, row, slider};
use iced::{Alignment, Element, Length};
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
    let fields = responsive(move |size| editor_fields(app, (size.width * 0.40).min(250.0)))
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

fn editor_fields(app: &App, control_width: f32) -> Element<'_, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let field_row =
        |field: &GeneralSettingField, description, control: Element<'static, Message>| {
            settings_flexible_row_with_help(
                i18n,
                &app.app_theme,
                i18n.__(field.label),
                description,
                control_width,
                control,
            )
            .id(field.id)
        };
    let theme_pick = settings_select(
        iced::Theme::ALL.iter().cloned(),
        Some(app.effective_editor_theme()),
        |theme| Message::Editor(EditorMessage::ThemeChanged(theme)),
    )
    .disabled(app.editor_follow_system_theme)
    .width(Length::Fill);

    let follow_system_theme = field_row(
        &general_fields::EDITOR_FOLLOW_THEME,
        i18n.__("启用后沿用应用当前主题。"),
        checkbox(app.editor_follow_system_theme)
            .label(i18n.__("启用"))
            .on_toggle(|v| Message::Editor(message::EditorMessage::ToggleFollowSystemTheme(v)))
            .style(settings_checkbox_style)
            .into(),
    );

    let editor_theme_row = settings_row_layout(
        i18n.__(general_fields::EDITOR_THEME.label),
        i18n.__("关闭跟随系统主题时使用的编辑器配色。"),
        container(theme_pick).align_right(control_width),
    )
    .id(general_fields::EDITOR_THEME.id);

    let displayed = metrics::displayed(app);
    let size_slider = row![
        metrics_slider(10.0..=30.0, displayed.font_size, |v| {
            Message::Editor(message::EditorMessage::FontSizePreview(v))
        })
        .width(Length::Fill),
        settings_value_badge(format!("{:.0}px", displayed.font_size))
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    let size_row = field_row(
        &general_fields::EDITOR_FONT_SIZE,
        i18n.__("调整编辑器主文本字号。"),
        size_slider.into(),
    );

    let line_height_slider = row![
        metrics_slider(10.0..=60.0, displayed.line_height, |v| {
            Message::Editor(message::EditorMessage::LineHeightPreview(v))
        })
        .width(Length::Fill),
        settings_value_badge(format!("{:.1}px", displayed.line_height))
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    let line_height_row = field_row(
        &general_fields::EDITOR_LINE_HEIGHT,
        i18n.__("调整每一行的垂直高度。"),
        line_height_slider.into(),
    );

    let auto_line_height = field_row(
        &general_fields::EDITOR_AUTO_LINE_HEIGHT,
        i18n.__("根据字体大小自动推导更合适的行高。"),
        checkbox(displayed.auto_line_height)
            .label(i18n.__("根据字体大小自动调整"))
            .on_toggle(|v| Message::Editor(message::EditorMessage::ToggleAutoLineHeight(v)))
            .style(settings_checkbox_style)
            .into(),
    );

    #[cfg(not(target_arch = "wasm32"))]
    let lsp_enabled = settings_row_layout(
        i18n.__(general_fields::EDITOR_LSP.label),
        i18n.__("启用代码补全、悬停提示与跳转定义；需要对应语言服务器可用。"),
        container(
            checkbox(!app.lsp_disabled)
                .label(i18n.__("启用"))
                .on_toggle(|enabled| {
                    Message::Preview(message::PreviewMessage::LspToggleDisabled(!enabled))
                })
                .style(settings_checkbox_style),
        )
        .align_right(control_width),
    )
    .id(general_fields::EDITOR_LSP.id);

    let mut fields = column![
        follow_system_theme,
        settings_divider(),
        editor_theme_row,
        settings_divider(),
        size_row,
        settings_divider(),
        line_height_row,
        settings_divider(),
        auto_line_height,
    ]
    .spacing(0);

    #[cfg(not(target_arch = "wasm32"))]
    {
        fields = fields.push(settings_divider()).push(lsp_enabled);
    }

    settings_group_card(fields).into()
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
