//! Native Coding Plan quota queries on the authorized execution host.

#[cfg(not(target_arch = "wasm32"))]
pub mod command_code;
#[cfg(not(target_arch = "wasm32"))]
pub mod kimi;
#[cfg(not(target_arch = "wasm32"))]
pub mod minimax;
#[cfg(not(target_arch = "wasm32"))]
pub mod opencode_go;
#[cfg(not(target_arch = "wasm32"))]
pub mod volcengine;
#[cfg(not(target_arch = "wasm32"))]
pub mod volcengine_sign;
#[cfg(not(target_arch = "wasm32"))]
pub mod zenmux;
#[cfg(not(target_arch = "wasm32"))]
pub mod zhipu;
#[cfg(not(target_arch = "wasm32"))]
pub mod zhipu_team;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod volcengine_sign_tests;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod volcengine_tests;

#[cfg(not(target_arch = "wasm32"))]
pub mod zhipu_balance;
