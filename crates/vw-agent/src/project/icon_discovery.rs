//! 项目图标自动发现。
//!
//! 该模块在 Git 项目中查找常见 favicon 文件，并把最短路径的候选图标编码为
//! data URL 写回项目元数据。用户手动设置过图标或外部 URL 时会直接跳过，避免
//! 自动发现覆盖显式选择。

use super::{Error, IconUpdate, Info, UpdateInput, Vcs, update};
use base64::Engine;
use std::path::PathBuf;
use vw_api_types::id::ProjectScope;

#[cfg(target_arch = "wasm32")]
/// wasm 目标下的图标发现占位实现。
///
/// # 错误
///
/// 当前实现不会失败，保留 `Result` 是为了与原生目标签名一致。
pub async fn discover(_scope: &ProjectScope, _input: &Info) -> Result<(), Error> {
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
/// 为项目自动发现并保存图标。
///
/// 只处理 Git 项目，且不会覆盖已有的用户覆盖图标或图标 URL。发现成功后会通过
/// `update` 写回项目记录。
///
/// # 错误
///
/// 读取候选图标失败或项目更新失败时返回错误；无候选、非 Git 项目、目录不存在
/// 等不需要处理的情况返回 `Ok(())`。
pub async fn discover(scope: &ProjectScope, input: &Info) -> Result<(), Error> {
    if input.vcs != Some(Vcs::Git) {
        return Ok(());
    }
    if input.icon.as_ref().is_some_and(|i| i.override_icon.is_some()) {
        return Ok(());
    }
    if input.icon.as_ref().is_some_and(|i| i.url.is_some()) {
        return Ok(());
    }

    let root = PathBuf::from(&input.worktree);
    if !root.is_dir() {
        return Ok(());
    }

    let candidates = tokio::task::spawn_blocking(move || {
        let mut found: Vec<PathBuf> = Vec::new();
        // 遍历可能较大的仓库树，放到阻塞线程，避免卡住 async runtime 工作线程。
        for entry in walkdir::WalkDir::new(&root)
            .follow_links(false)
            .into_iter()
            .flatten()
            .filter(|e| e.file_type().is_file())
        {
            let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
            if name == "favicon.ico"
                || name == "favicon.png"
                || name == "favicon.svg"
                || name == "favicon.jpg"
                || name == "favicon.jpeg"
                || name == "favicon.webp"
            {
                found.push(entry.path().to_path_buf());
            }
        }
        // 更短路径通常更接近项目根，优先选它能减少误选深层依赖资源的概率。
        found.sort_by_key(|p| p.to_string_lossy().len());
        found
    })
    .await
    .unwrap_or_default();

    let Some(shortest) = candidates.into_iter().next() else {
        return Ok(());
    };

    let bytes = tokio::fs::read(&shortest).await?;
    let ext = shortest.extension().and_then(|s| s.to_str()).unwrap_or("png").to_ascii_lowercase();

    let mime = match ext.as_str() {
        "ico" => "image/x-icon",
        "svg" => "image/svg+xml",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "png" => "image/png",
        _ => "image/png",
    };

    let base64 = base64::engine::general_purpose::STANDARD.encode(bytes);
    let url = format!("data:{};base64,{}", mime, base64);

    let _ = update(
        scope,
        UpdateInput {
            project_id: input.id.clone(),
            name: None,
            icon: Some(IconUpdate { url: Some(Some(url)), override_icon: None, color: None }),
            commands: None,
        },
    )
    .await?;
    Ok(())
}

#[cfg(test)]
#[path = "icon_discovery_tests.rs"]
mod icon_discovery_tests;
