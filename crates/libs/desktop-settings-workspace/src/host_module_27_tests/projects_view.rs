//! 验证项目列表和 Worktree 选项的界面状态。
//!
//! 项目夹具主动清空最近记录与编辑状态，用于隔离缺失路径和确认框边界。

use super::Fixture;
use crate::system_settings_projects::{self as domain, Event, ViewState};
use desktop_i18n::I18n;
use iced::Element;
use iced::{Length, Size};

fn current_project_worktree(app: &Fixture, i18n: I18n) -> Element<'_, Event> {
    static THEME: std::sync::LazyLock<iced::Theme> =
        std::sync::LazyLock::new(|| iced::Theme::Light);
    domain::current_project_worktree(
        &ViewState {
            project_path: &app.project_path,
            project_worktree_enabled: &app.project_worktree_enabled,
            project_tools_menu_path: &None,
            recent_projects: &[],
            recent_projects_edits: &[],
            recent_project_delete_confirm_idx: None,
            recent_project_query: "",
            desktop_locale: app.desktop_locale,
            app_theme: &THEME,
        },
        i18n,
        |event| event,
        help,
    )
}
fn test_app() -> Fixture {
    Fixture::default()
}

#[test]
fn worktree_row_builds_fill_width_help_label_and_control_layout() {
    let app = test_app();
    let element = current_project_worktree(&app, I18n::new(app.desktop_locale));

    assert_eq!(element.as_widget().size(), Size::new(Length::Fill, Length::Shrink));

    let container_children = element.as_widget().children();
    assert_eq!(container_children.len(), 2);
    assert_eq!(container_children[0].children.len(), 2);
    assert!(container_children[1].children.is_empty());
}

fn help<'a>(
    _: I18n,
    _: &iced::Theme,
    label: &'a str,
    description: &'a str,
    width: f32,
    control: Element<'a, Event>,
) -> iced::widget::Container<'a, Event> {
    use desktop_ui_controls::system_settings_common::{
        help::settings_hover_help_label, settings_field_label, settings_flexible_row_with_label,
    };
    let label = settings_hover_help_label(
        settings_field_label(label),
        iced::widget::text(description).into(),
    )
    .width(iced::Length::Fill)
    .wrap();
    settings_flexible_row_with_label(label, width, control)
}
