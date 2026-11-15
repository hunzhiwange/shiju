//! 提供商组装与模型查询。

mod query;
mod state;
#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
mod overrides;
#[cfg(target_arch = "wasm32")]
mod gateway;

pub use vw_shared::provider::state::{
    State, as_string_map, builtin_model_limit, default_model_capabilities, from_models_dev_model,
    from_models_dev_provider, normalize_adapter, resolve_model_capabilities, resolve_model_limit,
};
pub use vw_shared::provider::types::{
    ApiInfo, Capabilities, CapabilityIO, Info, InterleavedCapability, Model, ModelCost,
    ModelCostCache, ModelCostOver200k, ModelLimit, ModelNotFoundError, ParsedModelRef,
    ProviderSource, default_adapter,
};
pub use vw_shared::provider::types::{parse_model, sort};

pub use query::{list, list_for_settings, get_provider, get_model, default_model, get_small_model, init};
pub use state::invalidate_cache;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod test_support;
