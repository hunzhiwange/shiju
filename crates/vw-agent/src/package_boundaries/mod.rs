#[cfg(feature = "memory-mariadb")]
mod memory_mariadb_tests;
#[cfg(not(target_arch = "wasm32"))]
mod memory_lucid_tests;
