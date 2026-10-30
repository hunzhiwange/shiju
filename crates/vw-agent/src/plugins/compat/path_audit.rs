//! 所有插件格式共享的组件路径词法边界；文件系统审计仍由 plugins::audit 完成。

use anyhow::{Result, bail};

pub(crate) fn validate_component_path(field: &str, raw: &str) -> Result<()> {
    let Some(relative) = raw.strip_prefix("./") else {
        bail!("Plugin manifest field {field} path must start with './': {raw}");
    };
    if relative.starts_with('/') || relative.contains('\\') {
        bail!("Plugin manifest field {field} path must stay within plugin root using '/': {raw}");
    }
    let mut resource = None;
    for component in relative.split('/') {
        match component {
            "" | "." => {}
            ".." => {
                bail!(
                    "Plugin manifest field {field} path escapes plugin root or contains a parent component: {raw}"
                );
            }
            component => {
                resource.get_or_insert(component);
            }
        }
    }
    let Some(first) = resource else {
        bail!("Plugin manifest field {field} path must name a resource below plugin root: {raw}");
    };
    // Apply the same admission rule on every host: a Windows drive is never a POSIX resource name.
    if matches!(first.as_bytes(), [drive, b':', ..] if drive.is_ascii_alphabetic()) {
        bail!("Plugin manifest field {field} path must not use a Windows drive: {raw}");
    }
    Ok(())
}
