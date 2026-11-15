//! 知识文本分块、检索融合与文档导入。

pub mod chunker;
pub mod fusion;
pub mod import;

#[cfg(not(target_arch = "wasm32"))]
pub mod store;
#[cfg(not(target_arch = "wasm32"))]
mod import_error;

#[cfg(test)]
mod import_tests;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;
