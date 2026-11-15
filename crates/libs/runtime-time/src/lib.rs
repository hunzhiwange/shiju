//! 跨平台时间读取与活动区间计时。

mod time;

pub use time::{ActiveTimer, now, now_ms};
