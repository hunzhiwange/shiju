use super::I18n;
use vw_config_types::ui::DesktopLocale;

#[test]
fn desktop_gettext_area_10_uses_chinese_keys() {
    for (locale, expected) in [
        (DesktopLocale::SimplifiedChinese, "搜索文件 / 项目 / 历史会话"),
        (DesktopLocale::TraditionalChinese, "搜尋檔案 / 專案 / 歷史對話"),
        (DesktopLocale::English, "Search files / projects / conversation history"),
    ] {
        assert_eq!(I18n::new(locale).__("搜索文件 / 项目 / 历史会话"), expected);
    }
}

#[test]
fn desktop_gettext_area_10_catalog_covers_display_paths() {
    let en = super::catalog::parse_catalog(include_str!("locales/en.json")).unwrap();
    let tw = super::catalog::parse_catalog(include_str!("locales/zh-TW.json")).unwrap();
    let literal =
        regex::Regex::new(r#"(?:\.__|\.format|ProjectFeedback::new)\(\s*("(?:\\.|[^"\\])*")"#)
            .unwrap();
    for source in [
        include_str!("../components/file_tree/find_results.rs"),
        include_str!("../components/file_tree/mod.rs"),
        include_str!("../components/file_tree/tree_list.rs"),
        include_str!("../components/search_panel.rs"),
        include_str!("project.rs"),
        include_str!("../message/project/helpers.rs"),
        include_str!("../impls/app_view_modals/project_edit/general_tab.rs"),
        include_str!("../impls/app_view_modals/project_edit/launch_tab.rs"),
        include_str!("../impls/app_view_modals/project_edit/refresh_tab.rs"),
        include_str!("../impls/app_view_modals/project_edit/scheduling_tab.rs"),
        include_str!("../impls/app_view_modals/project_edit/mod.rs"),
        include_str!("../views/project/layout/work_sidebar.rs"),
        include_str!("../views/project/layout/work_resources.rs"),
        include_str!("../views/project/components/projects_list.rs"),
    ] {
        for capture in literal.captures_iter(source) {
            let key: String = serde_json::from_str(&capture[1]).unwrap();
            assert!(en.contains_key(&key), "missing English key: {key}");
            assert!(tw.contains_key(&key), "missing Traditional Chinese key: {key}");
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn desktop_gettext_area_10_dynamic_feedback_retains_raw_arguments() {
    use crate::app::message::project::ProjectMessage;
    let (mut app, _) = crate::app::App::new();
    let raw = "用户目录/%n/%s/原始错误";
    app.file_tree_clipboard = None;
    let _ = app.show_localized_toast(
        crate::app::state::ToastKind::Success,
        "已重命名为 %s",
        vec![raw.into()],
    );
    let message = app.active_toast.as_ref().unwrap().localized_message.as_ref().unwrap();
    for (locale, prefix) in [
        (DesktopLocale::SimplifiedChinese, "已重命名为 "),
        (DesktopLocale::TraditionalChinese, "已重新命名為 "),
        (DesktopLocale::English, "Renamed to "),
    ] {
        assert_eq!(message.render(I18n::new(locale)).unwrap(), format!("{prefix}{raw}"));
    }
    app.file_tree_rename_path = Some("/project/file".into());
    app.file_tree_rename_value = "../invalid".into();
    let _ = crate::app::message::project::update(&mut app, ProjectMessage::FileTreeRenameSave);
    let message = app.active_toast.as_ref().unwrap().localized_message.as_ref().unwrap();
    for (locale, expected) in [
        (DesktopLocale::SimplifiedChinese, "文件名不合法"),
        (DesktopLocale::TraditionalChinese, "檔案名稱無效"),
        (DesktopLocale::English, "Invalid file name"),
    ] {
        assert_eq!(message.render(I18n::new(locale)).unwrap(), expected);
    }
}
