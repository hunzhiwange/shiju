//! 应用自更新：发布源查询、下载与二进制替换。

#[cfg(not(target_arch = "wasm32"))]
mod binary;
mod update;

pub use update::{
    UpdateProgress, check_for_update, current_version, desktop_self_update,
    desktop_self_update_with_progress, fetch_latest_version, self_update,
};
