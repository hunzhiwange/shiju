//! 验证时间格式化输出及 IANA 时区配置的应用。

use super::locale::{
    configured_datetime_at, datetime, parse_timezone, time_short, today_time_or_datetime,
};
use chrono::{TimeZone, Utc};

#[test]
fn formats_known_timestamp_without_empty_output() {
    assert!(time_short(0).contains(':'));
    assert!(datetime(0).contains("1970"));
    assert!(!today_time_or_datetime(0).is_empty());
}

#[test]
fn configured_datetime_uses_iana_timezone() {
    let utc = Utc.with_ymd_and_hms(2026, 1, 15, 0, 30, 0).unwrap();

    assert_eq!(
        configured_datetime_at(utc, chrono_tz::Asia::Shanghai),
        "2026-01-15 08:30:00 +08:00 (Asia/Shanghai)"
    );
    assert_eq!(
        configured_datetime_at(utc, chrono_tz::America::New_York),
        "2026-01-14 19:30:00 -05:00 (America/New_York)"
    );
    assert_eq!(parse_timezone("Asia/Shanghai").unwrap(), chrono_tz::Asia::Shanghai);
    assert!(
        parse_timezone("Mars/Olympus_Mons")
            .unwrap_err()
            .to_string()
            .contains("invalid IANA timezone: Mars/Olympus_Mons")
    );
}
