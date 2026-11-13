//! 预览面板组件模块
//!
//! 本模块提供用于显示和交互预览内容的 UI 组件集合。预览面板是一个多功能界面，
//! 支持多种内容类型的渲染、LSP（语言服务器协议）集成、样式定制以及标签页管理。
//!
//! # 模块结构
//!
//! - [`lsp_overlay`] - LSP 覆盖层组件，提供代码智能提示、诊断信息叠加显示等功能
//! - [`styles`] - 预览面板的样式定义，包括颜色、布局、字体等视觉配置
//! - [`tabs`] - 标签页管理组件，支持多文档/多视图的标签式切换
//! - [`view`] - 主视图组件，是预览面板的核心渲染入口点
//! - [`widgets`] - 可复用的 UI 小部件集合，如按钮、输入框、状态指示器等
//!
//! # 使用示例
//!
//! ```ignore
//! use app::components::preview_panel::view;
//!
//! // 在应用中渲染预览面板视图
//! let panel = view(ctx, props);
//! ```

mod lsp_overlay;
mod styles;
pub(super) mod tabs;
mod view;
mod widgets;

#[cfg(test)]
#[path = "lsp_overlay_tests.rs"]
mod lsp_overlay_tests;
#[cfg(test)]
#[path = "styles_tests.rs"]
mod styles_tests;
#[cfg(test)]
#[path = "tabs_tests.rs"]
mod tabs_tests;
#[cfg(test)]
#[path = "view_tests.rs"]
mod view_tests;
#[cfg(test)]
#[path = "widgets_tests.rs"]
mod widgets_tests;

pub(crate) use view::build_settings_overlay;
pub use view::view;
