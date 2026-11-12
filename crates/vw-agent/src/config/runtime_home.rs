//! One home boundary for configuration stores and runtime-owned credentials.
use anyhow::{Result, ensure};
use std::path::PathBuf;

pub(super) fn config_home() -> Result<PathBuf> {
    let home = crate::global::paths().home.clone();
    ensure!(home.is_absolute(), "Could not resolve an absolute runtime home directory");
    Ok(home)
}
