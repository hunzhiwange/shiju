use super::I18n;
use vw_config_types::ui::DesktopLocale;

#[test]
fn desktop_gettext_area_8_cached_exploration_counts_switch_without_reparsing() {
    let counts = super::chat::ExploreCounts { reads: 2, searches: 1, listings: 0, others: 0 };
    assert_eq!(counts.render(I18n::new(DesktopLocale::SimplifiedChinese)), "2 次读取，1 次搜索");
    assert_eq!(counts.render(I18n::new(DesktopLocale::TraditionalChinese)), "2 次讀取，1 次搜尋");
    assert_eq!(counts.render(I18n::new(DesktopLocale::English)), "2 reads，1 searches");
}

#[test]
fn desktop_gettext_area_8_cached_message_metadata_switches_zero_timestamp() {
    let source =
        super::chat::MessageMeta::Model { model: "raw/%n/%s".to_string(), time_ms: Some(0) };
    assert_eq!(source.render(I18n::new(DesktopLocale::English)), "raw/%n/%s · Just now");
    assert_eq!(source.render(I18n::new(DesktopLocale::TraditionalChinese)), "raw/%n/%s · 剛剛");
}

#[test]
fn desktop_gettext_area_8_semantic_time_switches_locale() {
    let bucket = (1, 7);
    assert_eq!(
        super::chat::relative_time(I18n::new(DesktopLocale::English), bucket),
        "7 minutes ago"
    );
    assert_eq!(
        super::chat::relative_time(I18n::new(DesktopLocale::TraditionalChinese), bucket),
        "7 分鐘前"
    );
    assert_eq!(super::chat::elapsed(I18n::new(DesktopLocale::English), 125), "2m 5s");
    assert_eq!(super::chat::elapsed(I18n::new(DesktopLocale::SimplifiedChinese), 125), "2分 5秒");
}
