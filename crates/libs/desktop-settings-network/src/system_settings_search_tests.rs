use super::*;
#[test]
fn search_entry_matches_chinese_full_pinyin_initials_and_ascii() {
    let entry = SystemSettingsSearchEntry::new(
        "智能体",
        None,
        &["Agent settings", "API2"],
        (),
    );

    assert!(entry.matches("智能体"));
    assert!(entry.matches("zhinengti"));
    assert!(entry.matches("zhi neng ti"));
    assert!(entry.matches("znt"));
    assert!(entry.matches("AGENT SETTINGS"));
    assert!(entry.matches("api2"));
    assert!(!entry.matches("memory storage"));
}
