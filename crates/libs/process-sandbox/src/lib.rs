//! 可插拔的进程沙箱接口与操作系统隔离后端。

pub mod traits;
pub mod docker;
#[cfg(feature = "sandbox-bubblewrap")]
pub mod bubblewrap;
#[cfg(target_os = "linux")]
pub mod firejail;
#[cfg(feature = "sandbox-landlock")]
pub mod landlock;

pub use traits::{NoopSandbox, Sandbox, SandboxBounds};
