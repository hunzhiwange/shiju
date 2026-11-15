//! 受限远程模型发现与目录合并。
#![cfg(not(target_arch = "wasm32"))]

mod http;
mod merge;

pub use http::{DiscoveryError, DiscoveredModel, is_real_bearer, discover_openai_compatible_models, discover_catalog_models};
pub use merge::{resolve_api_base, merge_remote_models, discover_and_merge_provider};

#[cfg(test)]
mod test_support;
#[cfg(test)]
mod http_tests;
#[cfg(test)]
mod merge_tests;
