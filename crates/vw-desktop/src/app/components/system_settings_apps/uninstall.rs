//! Uninstall selection stays bound to a discovered instance; execution belongs to the gateway.
use vw_gateway_client::vw_api_types::agent_apps::*;

pub(crate) fn selectable(
    request: &AppActionPreviewRequest,
    installation: &AppInstallation,
) -> bool {
    request.app_id == installation.app_id
        && installation.source != AppInstallationSource::Unknown
        && matches!(&request.target, AppActionTarget::Installation { installation_id } if installation_id == &installation.id)
}

pub(crate) fn guidance(app: AppId) -> &'static str {
    match app {
        AppId::GrokBuild => "Grok 警告：npm 模式更新器可能重建已卸载的包副本；本次只处理所选实例。",
        AppId::Hermes => "Hermes 提示：配置、会话和用户数据将保留；卸载后可能仍有残留。",
        _ => "只卸载本次预览选定的实例；其他实例与用户数据不会由桌面端清理。",
    }
}

pub(crate) const MANUAL_GUIDANCE: &str = "来源未知：请先向管理员核实真实路径和安装来源，再按原安装渠道人工卸载。本界面不执行命令或删除文件。";
