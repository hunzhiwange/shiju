//! 本地密钥加密存储。

pub mod secrets;

#[cfg(not(target_arch = "wasm32"))]
pub mod gateway_skey_store;
