//! Provider 认证信息读取入口。
//!
//! 本模块不负责认证流程本身，只负责定位认证存储文件并读取其中内容，
//! 供上层 provider 解析逻辑判断某个 provider 是否已具备可用凭据。

use std::collections::HashMap;
use std::path::PathBuf;

pub use auth_types::Info;

use provider_runtime_paths as global;

/// 解析认证信息存储文件路径。
///
/// 路径由全局 home/data 目录共同决定，以保持桌面端与测试环境行为一致。
fn filepath() -> PathBuf {
    let paths = global::paths();
    vw_shared::auth::store::resolve_filepath(&paths.home, &paths.data)
}

/// 读取指定 provider 的认证信息。
///
/// # 参数
///
/// * `provider_id` - provider 的稳定标识
///
/// # 返回值
///
/// 若存在对应认证记录则返回其信息，否则返回 `None`
pub fn get(provider_id: &str) -> Option<Info> {
    vw_shared::auth::store::get_from(&filepath(), provider_id)
}

/// 读取全部 provider 的认证信息。
///
/// 返回值中的 key 为 provider_id，value 为对应的认证信息。
pub fn all() -> HashMap<String, Info> {
    vw_shared::auth::store::all_from(&filepath())
}

#[cfg(test)]
#[path = "auth_tests.rs"]
mod auth_tests;
