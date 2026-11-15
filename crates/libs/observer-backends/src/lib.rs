//! 日志、组合、空操作及详细输出观测后端。

pub mod log;
pub mod multi;
pub mod noop;
pub mod verbose;

pub use log::LogObserver;
pub use multi::MultiObserver;
pub use noop::NoopObserver;
pub use verbose::VerboseObserver;
