//! 执行状态及结果落地的领域行为证据；不启动桌面 App。
mod execution_tests;
mod execution_results_tests;
#[cfg(not(target_arch = "wasm32"))]
mod panel_fixture;
#[cfg(not(target_arch = "wasm32"))]
mod panel_ui_tests;
#[cfg(not(target_arch = "wasm32"))]
mod forms_ui_tests;
