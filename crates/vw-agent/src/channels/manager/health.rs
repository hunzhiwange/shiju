//! 已配置通道的健康检查、超时分类与汇总输出。
//!
//! 初始化失败与运行期探测结果会合并统计；任一通道不健康时整体检查返回错误。

use super::*;
pub(crate) use channel_runtime::manager::health::*;
pub async fn doctor_channels(config: Config) -> Result<()> { channel_runtime::manager::health::doctor_channels(config, crate::session::ui_store::current_tenant_scope()).await }
#[cfg(test)]
#[path = "../../../../libs/channel-runtime/src/manager/health_tests.rs"]
mod health_tests;
