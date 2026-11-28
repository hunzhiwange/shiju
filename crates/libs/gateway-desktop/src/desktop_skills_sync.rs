use axum::{
    Json,
    extract::ConnectInfo,
    http::{HeaderMap, StatusCode, header},
};
use directories::BaseDirs;
use gateway_desktop_settings::{
    DesktopSkillSyncItem, DesktopSkillSyncRequest, DesktopSkillSyncStatus, SkillSyncTarget,
    global_skill_sync_directory,
};
use gateway_types::ApiError;
use std::{collections::HashMap, fs, io::ErrorKind, net::SocketAddr};

use super::{normalize_optional_project_path, reject_plugin_mutation, resolve_local_skill_dir};

pub(super) async fn sync_post(
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(request): Json<DesktopSkillSyncRequest>,
) -> Result<Json<Vec<DesktopSkillSyncItem>>, ApiError> {
    if !peer.ip().is_loopback()
        || headers.contains_key(header::ORIGIN)
        || headers
            .keys()
            .any(|key| key.as_str() == "forwarded" || key.as_str().starts_with("x-forwarded-"))
    {
        return Err(ApiError {
            status: StatusCode::FORBIDDEN,
            message: "技能同步需要本机网关".into(),
        });
    }

    let config = agent_config::runtime::get().await;
    tokio::task::spawn_blocking(move || {
        let project_path = normalize_optional_project_path(request.project_path);
        let directories =
            BaseDirs::new().ok_or_else(|| ApiError::bad_request("无法确定用户主目录"))?;
        let base = directories.home_dir();
        if request.targets.is_empty()
            || request.targets.len() > SkillSyncTarget::THIRD_PARTY_AGENTS.len()
        {
            return Err(ApiError::bad_request("请选择一个或多个安装目标"));
        }
        let mut targets = Vec::with_capacity(request.targets.len());
        for target in request.targets {
            let relative = global_skill_sync_directory(target)
                .ok_or_else(|| ApiError::bad_request("此应用不支持技能目录同步"))?;
            let root = base.join(relative);
            targets.push((target, root));
        }

        let selected = request.skill_id.trim();
        let source = if let Some(bundled) = selected.strip_prefix("bundled:") {
            if super::bundled_skill_dir(bundled).is_none() {
                return Err(ApiError::bad_request("未找到可同步的技能"));
            }
            super::bundled_skills_root()
                .map_err(|_| ApiError::internal("bundled skill directory could not be prepared"))?
                .join(bundled)
        } else {
            let source = resolve_local_skill_dir(project_path.as_deref(), selected)
                .map_err(ApiError::bad_request)?;
            if request.expected_source_path.as_deref() != source.to_str() {
                return Err(ApiError::bad_request("技能来源已变化，请刷新后重试"));
            }
            reject_plugin_mutation(project_path.as_deref(), selected, &config)
                .map_err(ApiError::bad_request)?;
            source
        };
        if !fs::symlink_metadata(source.join("SKILL.md")).is_ok_and(|metadata| metadata.is_file()) {
            return Err(ApiError::bad_request("该技能缺少可同步的 SKILL.md"));
        }
        let name = source.file_name().ok_or_else(|| ApiError::bad_request("无效的技能目录名"))?;
        let mut results = HashMap::new();
        let mut items = Vec::with_capacity(targets.len());
        for (target, root) in targets {
            let status = *results.entry(root.clone()).or_insert_with(|| {
                use DesktopSkillSyncStatus::{AlreadyInstalled, Failed, Installed};
                if agent_skills::local_sources::validate_skill_target_root(base, &root).is_err() {
                    return Failed;
                }
                match fs::symlink_metadata(root.join(name)) {
                    Ok(metadata) if metadata.is_dir() => AlreadyInstalled,
                    Ok(_) => Failed,
                    Err(error) if error.kind() == ErrorKind::NotFound => {
                        if agent_skills::import::distribute_skill_folder(
                            &source,
                            &root,
                            &config.skills.security,
                        )
                        .is_ok()
                        {
                            Installed
                        } else {
                            Failed
                        }
                    }
                    Err(_) => Failed,
                }
            });
            items.push(DesktopSkillSyncItem { target, status });
        }
        Ok(Json(items))
    })
    .await
    .map_err(|_| ApiError::internal("skill sync task failed"))?
}
