//! CLI 终端调色板、坐标计算与绘制基础。

// 保留现有绘制原语与主题令牌；暂无生产调用的成员不扩大为公开 API。
#[allow(dead_code)]
pub mod theme;
#[cfg(test)]
#[path = "theme_tests.rs"]
mod theme_tests;

#[allow(dead_code)]
pub(crate) mod tui_utils;
#[cfg(test)]
#[path = "tui_utils_tests.rs"]
mod tui_utils_tests;

#[allow(dead_code)]
pub(crate) mod render;
#[cfg(test)]
#[path = "render_tests.rs"]
mod render_tests;

#[allow(dead_code)]
pub(crate) mod logo;
#[cfg(test)]
#[path = "logo_tests.rs"]
mod logo_tests;
