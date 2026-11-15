//! Current configuration belongs to the authorized execution host, not the OS terminal.
use super::request::LaunchMaterial;
use std::{
    path::{Path, PathBuf},
    process::Command,
};

/// Inputs are freshly validated host material and administrator-owned environment paths.
/// No shell evaluates this command; the terminal is only a PTY frontend.
pub fn command(
    material: LaunchMaterial,
    home: &Path,
    search_paths: &[PathBuf],
) -> Result<Command, String> {
    if material.skip_permissions
        || !home.is_absolute()
        || !home.is_dir()
        || home.canonicalize().map_err(|_| "execution-host home unavailable")? != home
        || search_paths.iter().any(|p| !p.is_absolute())
    {
        return Err("validated execution-host environment required".into());
    }
    let paths = search_paths
        .iter()
        .cloned()
        .chain(["/usr/bin", "/bin", "/usr/sbin", "/sbin"].map(PathBuf::from));
    let path = std::env::join_paths(paths).map_err(|_| "invalid execution-host search paths")?;
    let mut command = Command::new(material.program);
    command
        .args(material.args)
        .current_dir(material.cwd)
        .env_clear()
        .env("HOME", home)
        .env("PATH", path)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .env("XDG_CACHE_HOME", home.join(".cache"))
        .env("XDG_DATA_HOME", home.join(".local/share"))
        .env("XDG_STATE_HOME", home.join(".local/state"));
    for name in ["LANG", "LC_ALL", "LC_CTYPE"] {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
    Ok(command)
}
