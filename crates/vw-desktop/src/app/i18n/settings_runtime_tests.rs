use super::settings_runtime::text;
use vw_config_types::ui::DesktopLocale;
#[test]
fn desktop_i18n_settings_runtime_switches_visible_text() {
    assert_eq!(text(DesktopLocale::SimplifiedChinese, "运行时类型"), "运行时类型");
    assert_eq!(text(DesktopLocale::TraditionalChinese, "运行时类型"), "執行環境類型");
    assert_eq!(text(DesktopLocale::English, "运行时类型"), "Runtime type");
}

#[test]
fn desktop_i18n_settings_runtime_memory_storage_and_observability() {
    for (key, zh, tw, en) in [
        ("记忆系统配置", "记忆系统配置", "記憶系統設定", "Memory system configuration"),
        ("存储类型", "存储类型", "儲存類型", "Storage type"),
        ("可观测性配置", "可观测性配置", "可觀測性設定", "Observability configuration"),
    ] {
        assert_eq!(text(DesktopLocale::SimplifiedChinese, key), zh);
        assert_eq!(text(DesktopLocale::TraditionalChinese, key), tw);
        assert_eq!(text(DesktopLocale::English, key), en);
    }
}

#[test]
fn desktop_i18n_settings_runtime_dynamic_errors_preserve_raw_content() {
    use super::settings_runtime::validation_error;
    use super::settings_security::text as security_text;
    let raw = "用户/admin.example.com:外部错误😀";
    for locale in [
        DesktopLocale::SimplifiedChinese,
        DesktopLocale::TraditionalChinese,
        DesktopLocale::English,
    ] {
        assert_eq!(validation_error(locale, raw), raw);
        let duplicate = validation_error(locale, &format!("列表项已存在：{raw}"));
        assert!(duplicate.ends_with(raw));
        assert!(!security_text(locale, "Bash 安全检查").is_empty());
    }
    assert_eq!(
        validation_error(DesktopLocale::English, "Docker CPU 限制 必须大于 0"),
        "Docker CPU limit must be greater than 0"
    );
    assert_eq!(
        validation_error(DesktopLocale::TraditionalChinese, "列表项不能为空"),
        "列表項目不可為空"
    );
}

#[test]
fn desktop_i18n_settings_runtime_views_and_search_follow_locale() {
    use crate::app::components::{
        system_settings_memory as memory, system_settings_observability as observability,
        system_settings_runtime as runtime, system_settings_search as search,
        system_settings_security as security, system_settings_storage as storage,
    };
    use crate::app::{
        App,
        state::{MemorySettingsTab, SecuritySettingsTab},
    };
    use iced::advanced::widget::Tree;
    let mut app = App::new().0;
    app.memory_settings.backend = "qdrant".into();
    for locale in [
        DesktopLocale::SimplifiedChinese,
        DesktopLocale::TraditionalChinese,
        DesktopLocale::English,
    ] {
        app.desktop_locale = locale;
        for tab in [
            MemorySettingsTab::General,
            MemorySettingsTab::RetentionAndCache,
            MemorySettingsTab::Embedding,
            MemorySettingsTab::EmbeddingRoutes,
            MemorySettingsTab::Storage,
            MemorySettingsTab::Qdrant,
        ] {
            app.memory_settings.active_tab = tab;
            let element = memory::view(&app);
            std::hint::black_box(Tree::new(element.as_widget()));
        }
        for tab in [
            SecuritySettingsTab::Bash,
            SecuritySettingsTab::SandboxAndResources,
            SecuritySettingsTab::Audit,
            SecuritySettingsTab::OtpAndEstop,
            SecuritySettingsTab::Syscall,
            SecuritySettingsTab::Guard,
        ] {
            app.security_settings.active_tab = tab;
            let element = security::view(&app);
            std::hint::black_box(Tree::new(element.as_widget()));
        }
        for element in
            [runtime::view(&app), storage::embedded_view(&app), observability::view(&app)]
        {
            std::hint::black_box(Tree::new(element.as_widget()));
        }
        let entries = search::catalog(&app);
        let entry = entries
            .iter()
            .find(|entry| {
                entry.target
                    == search::SystemSettingsSearchTarget::Memory(
                        MemorySettingsTab::RetentionAndCache,
                    )
            })
            .unwrap();
        assert!(entry.matches(text(locale, "保留与缓存")));
        assert!(entry.matches("保留与缓存"));
        assert!(super::settings_security::help(locale).contains("\"security\""));
        assert!(super::settings_runtime::observability_help(locale).contains("\"observability\""));
    }
}
