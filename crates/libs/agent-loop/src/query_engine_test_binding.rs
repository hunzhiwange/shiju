//! Host integration tests reuse the production source with the concrete native host.
include!("query_engine.rs");
#[path = "query_engine_tests.rs"]
mod query_engine_tests;
