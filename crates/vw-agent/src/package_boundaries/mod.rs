mod git_discovery_tests;
#[cfg(feature = "memory-mariadb")]
mod memory_mariadb_tests;
#[cfg(feature = "memory-postgres")]
mod memory_postgres_tests;
#[cfg(not(target_arch = "wasm32"))]
mod memory_lucid_tests;
mod memory_sqlite_tests;
mod memory_markdown_tests;
mod gateway_pairing_tests;
mod project_storage_paths_tests;
#[cfg(not(target_arch = "wasm32"))]
mod memory_response_cache_tests;
mod memory_types_tests;
