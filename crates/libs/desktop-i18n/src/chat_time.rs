use once_cell::sync::Lazy;
use time::format_description::FormatItem;
pub fn format_chat_time_label(ts_ms: u64) -> String {
    static FMT: Lazy<Vec<FormatItem<'static>>> = Lazy::new(|| {
        // time 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        time::format_description::parse_borrowed::<2>("[year]-[month]-[day] [hour]:[minute]")
            .unwrap_or_default()
    });

    if ts_ms == 0 {
        return "刚刚".to_string();
    }

    let nanos = (ts_ms as i128).saturating_mul(1_000_000);
    time::OffsetDateTime::from_unix_timestamp_nanos(nanos)
        .ok()
        .and_then(|dt| dt.format(&FMT).ok())
        .unwrap_or_else(|| "刚刚".to_string())
}
