//! An application-owned foreground Gateway; never a service or a user instance.
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use llm_runtime::acp::model_source::gateway::*;
