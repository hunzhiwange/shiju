use super::settings_runtime::{Text, text};
use vw_config_types::ui::DesktopLocale;
#[test]
fn desktop_i18n_settings_runtime_switches_visible_text() {
    assert_eq!(text(DesktopLocale::SimplifiedChinese, Text::RuntimeKind), "运行时类型");
    assert_eq!(text(DesktopLocale::TraditionalChinese, Text::RuntimeKind), "執行環境類型");
    assert_eq!(text(DesktopLocale::English, Text::RuntimeKind), "Runtime type");
}

#[test]
fn desktop_i18n_settings_runtime_memory_storage_and_observability() {
    for (key, zh, tw, en) in [
        (Text::MemoryConfig, "记忆系统配置", "記憶系統設定", "Memory system configuration"),
        (Text::StorageKind, "存储类型", "儲存類型", "Storage type"),
        (Text::ObservabilityConfig, "可观测性配置", "可觀測性設定", "Observability configuration"),
    ] {
        assert_eq!(text(DesktopLocale::SimplifiedChinese, key), zh);
        assert_eq!(text(DesktopLocale::TraditionalChinese, key), tw);
        assert_eq!(text(DesktopLocale::English, key), en);
    }
}
