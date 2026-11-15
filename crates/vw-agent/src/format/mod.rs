//! 格式化领域组合入口。

mod host;
pub use agent_format::{FormatterStatus, State};

pub mod format {
    pub use super::{FormatterStatus, State, init, status};
}

pub fn init() {
    host::install();
    agent_format::init();
}

pub async fn status() -> Vec<FormatterStatus> {
    host::install();
    agent_format::status().await
}
