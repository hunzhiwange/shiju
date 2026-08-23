//! 验证系统设置搜索对中文、拼音、首字母和子标签目录的匹配。
//!
//! 搜索样例同时包含中文、全拼、分词拼音、首字母和大小写 ASCII 查询。

use super::system_settings::SystemTab;
use super::system_settings_search::{
    SystemSettingsSearchEntry, SystemSettingsSearchTarget, catalog,
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
    assert_target_count!(SystemSettingsSearchTarget::General(_), 3);
    assert_target_count!(SystemSettingsSearchTarget::Models(_), 4);
    assert_target_count!(SystemSettingsSearchTarget::Cron(_), 4);
    assert_target_count!(SystemSettingsSearchTarget::Agents(_), 11);
    assert_target_count!(SystemSettingsSearchTarget::Staff(_), 4);
    assert_target_count!(
        SystemSettingsSearchTarget::Channels(_),
        if cfg!(target_arch = "wasm32") { 20 } else { 21 }
    );
    assert_target_count!(SystemSettingsSearchTarget::Memory(_), 5);
    assert_target_count!(SystemSettingsSearchTarget::Autonomy(_), 4);
    assert_target_count!(SystemSettingsSearchTarget::Security(_), 6);
    assert_target_count!(SystemSettingsSearchTarget::Gateway(_), 2);
    assert_target_count!(SystemSettingsSearchTarget::Network(_), 3);
    assert_target_count!(SystemSettingsSearchTarget::AgentMcp(_), 3);
    assert_target_count!(SystemSettingsSearchTarget::AgentSkills(_), 4);

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
