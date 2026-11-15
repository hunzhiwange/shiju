//! Pure configuration patches; callers own authorization, files and transactions.
pub mod patch_dotenv;
#[cfg(test)]
mod patch_dotenv_tests;
pub mod patch_json;
#[cfg(test)]
mod patch_json_tests;
pub mod patch_toml;
#[cfg(test)]
mod patch_toml_tests;
pub mod patch_yaml;
#[cfg(test)]
mod patch_yaml_tests;

#[cfg(target_os = "macos")]
pub mod patch_json5;
#[cfg(all(test, target_os = "macos"))]
mod patch_json5_tests;
