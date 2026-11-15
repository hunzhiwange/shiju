//! ACP 认证环境变量解析。

use super::*;

/// 根据认证凭据构造注入代理进程的环境变量。
///
/// 空凭据会被忽略；方法 ID 会以原始 key、规范化 key 和 `VWACP_AUTH_` 前缀 key
/// 尝试注入。包含 `=` 或 NUL 的原始 key 不会直接作为环境变量名使用，避免生成
/// 非法或含义不明的环境项。
pub(super) fn build_agent_environment(
    auth_credentials: &HashMap<String, String>,
) -> HashMap<String, String> {
    let mut env = HashMap::new();
    for (method_id, credential) in auth_credentials {
        if credential.trim().is_empty() {
            continue;
        }

        if !method_id.contains('=') && !method_id.contains('\0') {
            env.entry(method_id.clone()).or_insert_with(|| credential.clone());
        }

        if let Some(normalized) = to_env_token(method_id) {
            env.entry(format!("VWACP_AUTH_{normalized}")).or_insert_with(|| credential.clone());
            env.entry(normalized).or_insert_with(|| credential.clone());
        }
    }
    env
}

/// 从环境变量读取指定认证方法的凭据。
///
/// 会尝试原始方法 ID、规范化 token 和 `VWACP_AUTH_` 前缀形式。返回 `None`
/// 表示没有非空凭据。
pub(super) fn read_env_credential(method_id: &str) -> Option<String> {
    auth_env_keys(method_id)
        .into_iter()
        .find_map(|key| std::env::var(&key).ok().filter(|value| !value.trim().is_empty()))
}

fn auth_env_keys(method_id: &str) -> Vec<String> {
    let mut keys = vec![method_id.to_string()];
    if let Some(token) = to_env_token(method_id) {
        keys.push(token.clone());
        keys.push(format!("VWACP_AUTH_{token}"));
    }
    keys
}

/// 将任意认证方法 ID 转换为可用的环境变量 token。
///
/// 非 ASCII 字母数字字符会被替换为下划线，首尾下划线会被裁剪，最终转为大写。
/// 如果结果为空则返回 `None`。
pub(super) fn to_env_token(value: &str) -> Option<String> {
    let token = value
        .trim()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect::<String>()
        .trim_matches('_')
        .to_ascii_uppercase();

    if token.is_empty() { None } else { Some(token) }
}

/// 提取命令 basename 并转为小写 token。
pub(super) fn basename_token(command: &str) -> String {
    Path::new(command)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(command)
        .to_ascii_lowercase()
}
