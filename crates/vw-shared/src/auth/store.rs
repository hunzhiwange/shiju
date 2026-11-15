use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde_json::Value;

use super::Info;

/// 解析鉴权文件路径，优先兼容旧目录，其次使用当前数据目录。
pub fn resolve_filepath(home: &Path, data: &Path) -> PathBuf {
    let legacy = config_runtime_paths::home_config_dir(home).join("auth.json");
    if legacy.exists() {
        return legacy;
    }

    let primary = data.join("auth.json");
    if primary.exists() {
        return primary;
    }

    legacy
}

/// 从指定文件读取全部提供商鉴权信息。
pub fn all_from(path: &Path) -> HashMap<String, Info> {
    let Ok(content) = std::fs::read_to_string(path) else {
        return HashMap::new();
    };

    let Ok(value) = serde_json::from_str::<Value>(&content) else {
        return HashMap::new();
    };

    let Some(obj) = value.as_object() else {
        return HashMap::new();
    };

    let mut out = HashMap::new();
    for (k, v) in obj {
        if let Ok(info) = serde_json::from_value::<Info>(v.clone()) {
            out.insert(k.clone(), info);
        }
    }
    out
}

/// 读取单个提供商的鉴权信息。
pub fn get_from(path: &Path, provider_id: &str) -> Option<Info> {
    all_from(path).get(provider_id).cloned()
}

/// 写入或覆盖单个提供商的鉴权信息。
pub fn set_to(path: &Path, key: &str, info: &Info) -> Result<(), std::io::Error> {
    let mut data = all_from(path);
    data.insert(key.to_string(), info.clone());
    write_auth_file(path, &data)
}

/// 删除单个提供商的鉴权信息。
pub fn remove_from(path: &Path, key: &str) -> Result<(), std::io::Error> {
    let mut data = all_from(path);
    data.remove(key);
    write_auth_file(path, &data)
}

fn write_auth_file(path: &Path, data: &HashMap<String, Info>) -> Result<(), std::io::Error> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let content = serde_json::to_string_pretty(data).map_err(std::io::Error::other)?;
    std::fs::write(path, content)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
    }

    Ok(())
}

#[cfg(test)]
#[path = "store_tests.rs"]
mod store_tests;
