//! Provider mode, native catalog and publication services.
pub mod providers;
pub mod aggregation;
pub mod models {
    pub use app_models::*;
    pub use app_model_catalog::codex_projection;
    #[cfg(test)]
    mod preset_catalog_tests;
    #[cfg(all(test, not(target_arch = "wasm32")))]
    mod fill_pi_tests;
}
pub mod desktop_presence;
#[cfg(all(test, unix))]
mod desktop_presence_tests;
