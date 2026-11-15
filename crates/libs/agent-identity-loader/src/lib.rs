//! AIEOS 身份配置选择、文件读取与加载错误处理。

mod load;

pub use load::{is_aieos_configured, load_aieos_identity};

#[cfg(test)]
mod tests;
