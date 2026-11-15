//! 维护运行时工具选择器状态及其回归测试。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

use super::{SessionToolBucket, SessionToolInventory, SessionToolSelectorState, SessionToolSelectorTab, SkillsDirectoryScope};

#[test]
fn tool_selector_defaults_enable_every_bucket_without_filters() {
    let selector = SessionToolSelectorState::default();

    for bucket in SessionToolBucket::ALL {
        assert!(selector.is_bucket_enabled(bucket));
    }
    assert!(!selector.has_tool_filter());
    assert!(!selector.has_manual_context_selection());
    assert_eq!(selector.active_tab(), SessionToolSelectorTab::Agent);
    assert_eq!(selector.query(), "");
    assert_eq!(selector.skill_directory_scope(), SkillsDirectoryScope::All);

    let inventory = SessionToolInventory::default();
    assert!(inventory.base_tools.is_empty());
    assert!(!inventory.restricts_tools);
}
