//! 状态动画辅助模块
//!
//! 提供运行中状态共用的帧动画字符与探索摘要数字翻转时长常量。

/// 共用活动动画的刷新间隔，避免请求期间的动画刷新挤占 UI 更新。
pub const ACTIVITY_ANIMATION_INTERVAL_MS: u64 = 90;

/// 与文件管理器刷新按钮一致的帧动画序列。
pub const STATUS_SPINNER_FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// 探索摘要数字翻转动画持续时长（毫秒）。
pub const EXPLORE_SUMMARY_FLIP_DURATION_MS: u64 = 1_100;

/// 根据动画帧序号返回当前应显示的旋转字符。
pub fn spinner_frame(frame: usize) -> &'static str {
    STATUS_SPINNER_FRAMES[frame % STATUS_SPINNER_FRAMES.len()]
}
