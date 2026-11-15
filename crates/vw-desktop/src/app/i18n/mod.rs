//! Desktop adapters for the shared internationalization catalog.
pub use desktop_i18n::*;
#[path = "macro.rs"]
mod macros;
pub mod apps;
pub mod editor;
pub mod other;
pub mod tasks;
#[cfg(test)]
mod settings_models_tests;
#[cfg(test)]
mod gettext_area_16_tests;
#[cfg(test)]
mod apps_tests;
#[cfg(test)]
mod gettext_area_7_tests;
#[cfg(test)]
mod gettext_area_6_tests;
#[cfg(test)]
mod settings_runtime_tests;
#[cfg(test)]
mod gettext_area_10_tests;
#[cfg(test)]
mod gettext_area_11_tests;
#[cfg(test)]
mod project_tests;
#[cfg(test)]
mod gettext_area_9_tests;
#[cfg(test)]
mod gettext_area_8_tests;
#[cfg(test)]
mod tasks_tests;
#[cfg(test)]
mod gettext_area_14_tests;
#[cfg(test)]
mod gettext_area_15_tests;
#[cfg(test)]
mod gettext_area_13_tests;
#[cfg(test)]
mod gettext_area_12_tests;
#[cfg(test)]
mod gettext_area_18_tests;
#[cfg(test)]
mod workflow_tests;
