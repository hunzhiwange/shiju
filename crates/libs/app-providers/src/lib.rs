//! Provider mode, native catalog and publication services.
pub mod providers;
pub mod aggregation;
pub mod models {
    #[cfg(test)]
    mod codex_template_tests;
    #[cfg(test)]
    mod defaults_tests;
    #[cfg(test)]
    mod fill_hermes_tests;
    #[cfg(test)]
    mod fill_opencode_tests;
    #[cfg(test)]
    mod preset_omp_tests;
    #[cfg(test)]
    mod fill_kimi_tests;
    pub use app_models::*;
    pub use app_model_catalog::*;
    #[cfg(test)]
    mod preset_catalog_tests;
    #[cfg(all(test, not(target_arch = "wasm32")))]
    mod fill_pi_tests;
}
pub mod desktop_presence;
#[cfg(all(test, unix))]
mod desktop_presence_tests;

pub mod target_model;
#[cfg(test)]
mod target_model_tests;
