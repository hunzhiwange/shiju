use super::settings_runtime::text;
use config_ui::DesktopLocale;
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
        app.general_settings.desktop_locale = locale;
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
