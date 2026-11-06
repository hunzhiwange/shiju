//! Byte patch primitives; file authorization, locking and commit belong to callers.
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
