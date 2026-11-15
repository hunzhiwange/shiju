//! 运行时实现由 agent-runtime 提供，宿主保留跨服务测试入口。

pub mod docker;
pub mod native;
pub mod traits;

pub use agent_runtime::{create_runtime, DockerRuntime, NativeRuntime, RuntimeAdapter};
