//! PDF 工具的组合层重导出。
pub use tool_files::pdf_read::PdfReadTool;

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "render_tests.rs"]
mod tests;
