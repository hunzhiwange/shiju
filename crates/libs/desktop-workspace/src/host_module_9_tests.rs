//! 项目侧栏的领域行为，使用局部输入而非桌面 App。
#[cfg(not(target_arch = "wasm32"))]
mod projects_grid;
mod sessions_panel;
mod sidebar_model;
mod sidebar_tests;
mod sidebar_view;
