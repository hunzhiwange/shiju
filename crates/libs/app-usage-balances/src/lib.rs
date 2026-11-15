//! Authorized provider wallet HTTP adapters.

#[cfg(not(target_arch = "wasm32"))]
pub mod deepseek;
#[cfg(not(target_arch = "wasm32"))]
pub mod novita;
#[cfg(not(target_arch = "wasm32"))]
pub mod openrouter;
#[cfg(not(target_arch = "wasm32"))]
pub mod siliconflow;
#[cfg(not(target_arch = "wasm32"))]
pub mod stepfun;
