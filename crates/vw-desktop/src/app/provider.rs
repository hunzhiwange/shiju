//! 封装模型提供方相关状态与选择逻辑。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

/// provider 子模块，拆分当前领域的局部职责。
pub mod provider {
    /// 对外暴露当前模块需要复用的能力。
    pub use vw_provider_resolver::provider::{
        default_model, get_model, get_provider, invalidate_cache, list, list_for_settings,
    };
    /// 对外暴露当前模块需要复用的能力。
    pub use vw_shared::provider::types::{
        ApiInfo, Capabilities, CapabilityIO, Info, InterleavedCapability, Model, ModelCost,
        ModelCostCache, ModelCostOver200k, ModelLimit, ModelNotFoundError, ParsedModelRef,
        ProviderSource, default_adapter,
    };
    /// 对外暴露当前模块需要复用的能力。
    pub use vw_shared::provider::types::{parse_model, sort};
}

/// provider_models 子模块，拆分当前领域的局部职责。
pub mod provider_models {
    /// 对外暴露当前模块需要复用的能力。
    pub use vw_provider_resolver::models::{get, invalidate_cache, refresh};
}

#[cfg(test)]
#[path = "provider_tests.rs"]
mod provider_tests;
