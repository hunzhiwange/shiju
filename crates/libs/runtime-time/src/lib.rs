//! 跨平台时间读取与活动区间计时。

mod time;
#[cfg(feature = "named-time-zones")]
pub mod zones;

pub use time::{ActiveTimer, now, now_ms, unix_millis, unix_seconds};
