//! Presentation of observations returned by the SDK; no local discovery.
use vw_gateway_client::{AgentAppsReadError, GatewayClient, vw_api_types::agent_apps::*};

#[derive(Debug, Clone)]
pub struct CatalogRow {
    pub entry: AppCatalogEntry,
    pub observation: Result<AppReadResponse, AgentAppsReadError>,
}

pub(crate) async fn load(
    client: GatewayClient,
    host: String,
    token: String,
) -> Result<Vec<CatalogRow>, AgentAppsReadError> {
    let catalog = client.agent_apps_catalog_authorized(&host, &token).await?;
    let mut rows = Vec::new();
    for entry in catalog.apps {
        let observation = client.agent_apps_installations_authorized(&host, entry.id, &token).await;
        rows.push(CatalogRow { entry, observation });
    }
    Ok(rows)
}

pub(crate) fn error_label(error: &AgentAppsReadError) -> &'static str {
    match error {
        AgentAppsReadError::Http { status: 503, .. } => "远端离线或服务不可用",
        AgentAppsReadError::Http { status: 501, .. } => "不支持",
        AgentAppsReadError::Http { status: 401 | 403, .. } => "未授权，请检查主机与应用管理令牌",
        AgentAppsReadError::Decode(_) => "响应无效，未验证",
        _ => "加载失败，请重试",
    }
}

pub(crate) fn runtime_label(kind: AppRuntimeKind) -> &'static str {
    match kind {
        AppRuntimeKind::NativeCli => "原生 CLI",
        AppRuntimeKind::AcpAdapter => "ACP 适配器",
        AppRuntimeKind::ReviewRuntime => "审查运行时",
        AppRuntimeKind::DesktopApp => "桌面应用",
        AppRuntimeKind::EditorIntegration => "编辑器集成",
    }
}

pub(crate) fn runtime_status(row: &CatalogRow, kind: AppRuntimeKind) -> String {
    // Native observations do not prove an adapter or editor runtime works.
    match kind {
        AppRuntimeKind::AcpAdapter => return "未验证（请在 ACP 页管理）".into(),
        AppRuntimeKind::ReviewRuntime | AppRuntimeKind::EditorIntegration => {
            return "未验证".into();
        }
        _ => {}
    }
    let observation = match &row.observation {
        Ok(value) => value,
        Err(error) => return error_label(error).into(),
    };
    if kind == AppRuntimeKind::DesktopApp {
        return match &observation.desktop {
            Some(desktop) if !desktop.supported => "不支持",
            Some(desktop) => match &desktop.app_bundles {
                Some(paths) if paths.is_empty() => "未安装",
                Some(_) => "已发现，运行未验证",
                None => "未验证",
            },
            None => "未验证",
        }
        .into();
    }
    if observation.installations.is_empty() {
        return "未安装".into();
    }
    observation
        .installations
        .iter()
        .map(|installation| match &installation.runtime_status {
            AppInstallationRuntimeStatus::Runnable => "可运行",
            AppInstallationRuntimeStatus::ExitedWithError { .. } => "运行失败",
            AppInstallationRuntimeStatus::ProbeFailed { reason } => match reason {
                AppInstallationProbeFailure::DamagedInstallation => "损坏",
                AppInstallationProbeFailure::Missing => "文件缺失",
                AppInstallationProbeFailure::PermissionDenied => "权限不足",
                AppInstallationProbeFailure::TimedOut => "探测超时，未验证",
                AppInstallationProbeFailure::Cancelled => "探测取消，未验证",
                _ => "探测失败，未验证",
            },
        })
        .collect::<Vec<_>>()
        .join(" / ")
}

#[cfg(test)]
#[path = "catalog_ui_refresh_06_tests.rs"]
mod ui_refresh_06_tests;
