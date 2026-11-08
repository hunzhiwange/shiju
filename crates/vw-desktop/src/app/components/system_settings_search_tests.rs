//! 验证系统设置搜索对中文、拼音、首字母和子标签目录的匹配。
//!
//! 搜索样例同时包含中文、全拼、分词拼音、首字母和大小写 ASCII 查询。

use super::system_settings::SystemTab;
use super::system_settings_search::{
    SystemSettingsSearchEntry, SystemSettingsSearchTarget, catalog, search_results,
};
use crate::app::App;
use crate::app::state::{MemorySettingsTab, ModelConfigTab};

#[test]
fn search_entry_matches_chinese_full_pinyin_initials_and_ascii() {
    let entry = SystemSettingsSearchEntry::new(
        "智能体",
        None,
        &["Agent settings", "API2"],
        SystemSettingsSearchTarget::TopLevel(SystemTab::Agents),
    );

    assert!(entry.matches("智能体"));
    assert!(entry.matches("zhinengti"));
    assert!(entry.matches("zhi neng ti"));
    assert!(entry.matches("znt"));
    assert!(entry.matches("AGENT SETTINGS"));
    assert!(entry.matches("api2"));
    assert!(!entry.matches("memory storage"));
}

#[test]
fn catalog_extracts_unique_navigable_child_tabs_and_scoped_keywords() {
    let mut app = App::new().0;
    let entries = catalog(&app);
    let child_entries = entries
        .iter()
        .filter(|entry| !matches!(entry.target, SystemSettingsSearchTarget::TopLevel(_)))
        .collect::<Vec<_>>();

    for (index, entry) in child_entries.iter().enumerate() {
        assert!(entry.parent_label.is_some(), "child entry lacks parent path: {entry:?}");
        assert!(
            !child_entries[..index].iter().any(|other| other.target == entry.target),
            "duplicate child target: {:?}",
            entry.target
        );
    }

    macro_rules! assert_target_count {
        ($pattern:pat, $expected:expr) => {
            assert_eq!(
                child_entries.iter().filter(|entry| matches!(entry.target, $pattern)).count(),
                $expected
            );
        };
    }
    assert_target_count!(SystemSettingsSearchTarget::General(_), 4);
    assert_target_count!(SystemSettingsSearchTarget::Models(_), 4);
    assert_target_count!(SystemSettingsSearchTarget::Cron(_), 4);
    assert_target_count!(SystemSettingsSearchTarget::Agents(_), 11);
    assert_target_count!(
        SystemSettingsSearchTarget::Channels(_),
        if cfg!(target_arch = "wasm32") { 7 } else { 8 }
    );
    assert_target_count!(SystemSettingsSearchTarget::Memory(_), 5);
    assert_target_count!(SystemSettingsSearchTarget::Autonomy(_), 4);
    assert_target_count!(SystemSettingsSearchTarget::Security(_), 6);
    assert_target_count!(SystemSettingsSearchTarget::Gateway(_), 2);
    assert_target_count!(SystemSettingsSearchTarget::Network(_), 5);
    assert_target_count!(SystemSettingsSearchTarget::AgentMcp(_), 3);
    assert_target_count!(SystemSettingsSearchTarget::AgentSkills(_), 3);

    let ocr_matches = child_entries
        .iter()
        .filter(|entry| entry.matches("OCR"))
        .map(|entry| entry.target)
        .collect::<Vec<_>>();
    assert_eq!(ocr_matches, vec![SystemSettingsSearchTarget::Models(ModelConfigTab::Multimodal)]);
    assert_eq!(
        child_entries
            .iter()
            .filter(|entry| entry.matches("allowedTools"))
            .map(|entry| entry.target)
            .collect::<Vec<_>>(),
        vec![SystemSettingsSearchTarget::AgentMcp(crate::app::state::McpSettingsTab::Servers)]
    );

    assert!(
        !child_entries
            .iter()
            .any(|entry| entry.target
                == SystemSettingsSearchTarget::Memory(MemorySettingsTab::Qdrant))
    );
    app.memory_settings.backend = "qdrant".to_string();
    assert!(
        catalog(&app)
            .iter()
            .any(|entry| entry.target
                == SystemSettingsSearchTarget::Memory(MemorySettingsTab::Qdrant))
    );

    #[cfg(target_arch = "wasm32")]
    assert!(!child_entries.iter().any(|entry| {
        entry.target
            == SystemSettingsSearchTarget::Channels(crate::app::state::ChannelsSettingsTab::Email)
    }));
}

#[test]
fn search_results_show_secondary_pages_with_parent_paths() {
    use vw_config_types::ui::DesktopLocale;

    let mut app = App::new().0;
    app.desktop_locale = DesktopLocale::English;

    let results = search_results(&app, "browser");
    let browser = results.first().expect("browser child result");
    assert_eq!(browser.target, SystemSettingsSearchTarget::Network(SystemTab::Browser));
    assert_eq!(browser.label, "Browser");
    assert_eq!(browser.parent_label, Some("Network settings"));

    let results = search_results(&app, "网络配置");
    assert_eq!(results[0].target, SystemSettingsSearchTarget::TopLevel(SystemTab::WebSearch));
    for tab in [
        SystemTab::WebSearch,
        SystemTab::HttpRequest,
        SystemTab::Browser,
        SystemTab::Proxy,
        SystemTab::Tunnel,
    ] {
        assert!(results.iter().any(|entry| {
            entry.target == SystemSettingsSearchTarget::Network(tab)
                && entry.parent_label == Some("Network settings")
        }));
    }
}

#[test]
fn search_matches_setting_labels_and_localizes_each_parent() {
    use super::system_settings::general_fields;
    use vw_config_types::ui::DesktopLocale;

    let mut app = App::new().0;
    for (locale, query, label, path) in [
        (DesktopLocale::SimplifiedChinese, "语言", "语言", "界面 > 常规"),
        (DesktopLocale::TraditionalChinese, "語言", "語言", "介面 > 一般"),
        (DesktopLocale::English, "yuyan", "Language", "Interface > General"),
    ] {
        app.desktop_locale = locale;
        let results = search_results(&app, query);
        let first = results.first().expect("language field");
        assert_eq!(
            first.target,
            SystemSettingsSearchTarget::GeneralField(&general_fields::LANGUAGE)
        );
        assert_eq!(first.label, label);
        assert_eq!(first.parent_path().as_deref(), Some(path));
    }

    let results = search_results(&app, "Font size");
    let paths: Vec<_> = results
        .iter()
        .filter(|entry| entry.label == "Font size")
        .filter_map(SystemSettingsSearchEntry::parent_path)
        .collect();
    assert_eq!(paths, ["Terminal > General", "Editor > General"]);
}
