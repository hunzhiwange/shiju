//! 验证相对时间按分钟至年份分桶并生成对应本地化标签。
//! 零值、未来时间和各单位切换点用于锁定边界归属而非只检查区间中值。

use super::time::{
    format_chat_time_label, project_last_modified_ms, relative_modified_label,
    relative_time_bucket, relative_time_label, relative_time_label_for_bucket,
};
use crate::app::App;

// ── relative_time_bucket ──

#[test]
fn relative_time_bucket_zero_input_returns_just_now() {
    assert_eq!(relative_time_bucket(0, 1_000_000), (0, 0));
    assert_eq!(relative_time_bucket(0, 0), (0, 0));
}

#[test]
fn relative_time_bucket_at_or_after_now_returns_just_now() {
    assert_eq!(relative_time_bucket(1000, 1000), (0, 0));
    assert_eq!(relative_time_bucket(2000, 1000), (0, 0));
}

#[test]
fn relative_time_bucket_under_one_minute_returns_just_now() {
    assert_eq!(relative_time_bucket(1000, 31_000), (0, 0));
    assert_eq!(relative_time_bucket(1000, 60_000), (0, 0));
}

#[test]
fn relative_time_bucket_minutes_range() {
    assert_eq!(relative_time_bucket(1000, 121_000), (1, 2));
    assert_eq!(relative_time_bucket(1000, 3_541_000), (1, 59));
}

#[test]
fn relative_time_bucket_hours_range() {
    assert_eq!(relative_time_bucket(1000, 7_201_000), (2, 2));
    assert_eq!(relative_time_bucket(1000, 82_801_000), (2, 23));
}

#[test]
fn relative_time_bucket_days_range() {
    assert_eq!(relative_time_bucket(1000, 172_801_000), (3, 2));
    assert_eq!(relative_time_bucket(1000, 2_505_601_000), (3, 29));
}

#[test]
fn relative_time_bucket_months_range() {
    assert_eq!(relative_time_bucket(1000, 5_184_001_000), (4, 2));
    assert_eq!(relative_time_bucket(1000, 28_512_001_000), (4, 11));
}

#[test]
fn relative_time_bucket_years_range() {
    assert_eq!(relative_time_bucket(1000, 63_072_001_000), (5, 2));
}

// ── relative_time_label_for_bucket ──

#[test]
fn relative_time_label_for_bucket_just_now() {
    assert_eq!(relative_time_label_for_bucket((0, 0)), "刚刚");
    assert_eq!(relative_time_label_for_bucket((0, 99)), "刚刚");
}

#[test]
fn relative_time_label_for_bucket_minutes() {
    assert_eq!(relative_time_label_for_bucket((1, 1)), "1 分钟前");
    assert_eq!(relative_time_label_for_bucket((1, 30)), "30 分钟前");
    assert_eq!(relative_time_label_for_bucket((1, 59)), "59 分钟前");
}

#[test]
fn relative_time_label_for_bucket_hours() {
    assert_eq!(relative_time_label_for_bucket((2, 1)), "1 小时前");
    assert_eq!(relative_time_label_for_bucket((2, 12)), "12 小时前");
    assert_eq!(relative_time_label_for_bucket((2, 23)), "23 小时前");
}

#[test]
fn relative_time_label_for_bucket_days() {
    assert_eq!(relative_time_label_for_bucket((3, 1)), "1 天前");
    assert_eq!(relative_time_label_for_bucket((3, 7)), "7 天前");
    assert_eq!(relative_time_label_for_bucket((3, 29)), "29 天前");
}

#[test]
fn relative_time_label_for_bucket_months() {
    assert_eq!(relative_time_label_for_bucket((4, 1)), "1 个月前");
    assert_eq!(relative_time_label_for_bucket((4, 6)), "6 个月前");
    assert_eq!(relative_time_label_for_bucket((4, 11)), "11 个月前");
}

#[test]
fn relative_time_label_for_bucket_years() {
    assert_eq!(relative_time_label_for_bucket((5, 1)), "1 年前");
    assert_eq!(relative_time_label_for_bucket((5, 10)), "10 年前");
}

#[test]
fn relative_time_label_for_bucket_unknown_bucket_falls_back() {
    assert_eq!(relative_time_label_for_bucket((6, 0)), "刚刚");
    assert_eq!(relative_time_label_for_bucket((255, 999)), "刚刚");
}

// ── format_chat_time_label ──

#[test]
fn format_chat_time_label_zero_returns_just_now() {
    assert_eq!(format_chat_time_label(0), "刚刚");
}

#[test]
fn format_chat_time_label_valid_timestamp() {
    // 2024-01-01 00:00:00 UTC in milliseconds
    let result = format_chat_time_label(1704067200000);
    assert_eq!(result, "2024-01-01 00:00");
}

#[test]
fn format_chat_time_label_out_of_range_timestamp_falls_back_to_just_now() {
    assert_eq!(format_chat_time_label(u64::MAX), "刚刚");
}

// ── relative_time_label ──

#[test]
fn relative_time_label_zero_returns_just_now() {
    assert_eq!(relative_time_label(0), "刚刚");
}

#[test]
fn relative_time_label_recent_offset_returns_just_now() {
    let now = crate::app::time::now_ms();
    assert_eq!(relative_time_label(now.saturating_sub(5_000)), "刚刚");
}

// ── project_last_modified_ms ──

#[test]
fn project_last_modified_ms_with_positive_updated_at_returns_it() {
    let mut app = App::new().0;
    app.project_updated_at_ms = Some(42);
    assert_eq!(project_last_modified_ms(&app), Some(42));
}

#[test]
fn project_last_modified_ms_with_zero_updated_at_falls_through() {
    let mut app = App::new().0;
    app.project_updated_at_ms = Some(0);
    assert_eq!(project_last_modified_ms(&app), None);
}

#[test]
fn project_last_modified_ms_with_no_path_and_no_updated_at_returns_none() {
    let app = App::new().0;
    assert_eq!(project_last_modified_ms(&app), None);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn project_last_modified_ms_with_missing_path_returns_none() {
    let root = tempfile::tempdir().expect("temporary root should be created");
    let mut app = App::new().0;
    app.project_path = Some(root.path().join("missing").to_string_lossy().into_owned());

    assert_eq!(project_last_modified_ms(&app), None);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn project_last_modified_ms_with_directory_uses_filesystem_modified_time() {
    let project = tempfile::tempdir().expect("temporary project directory should be created");
    let mut app = App::new().0;
    app.project_path = Some(project.path().to_string_lossy().into_owned());

    assert!(project_last_modified_ms(&app).is_some());
}

// ── relative_modified_label ──

#[test]
fn relative_modified_label_no_modified_shows_fallback() {
    let app = App::new().0;
    assert_eq!(relative_modified_label(&app), "最后修改 暂无");
}

#[test]
fn relative_modified_label_recent_modified_time_shows_minutes() {
    let mut app = App::new().0;
    app.project_updated_at_ms = Some(crate::app::time::now_ms().saturating_sub(120_000));

    assert_eq!(relative_modified_label(&app), "最后修改 2 分钟前");
}
