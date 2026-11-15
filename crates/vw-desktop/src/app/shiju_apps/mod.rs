//! 识矩应用领域状态导出与桌面辅助程序路径适配。
pub(crate) use desktop_app_launch::{
    ShijuAppDetailTarget, ShijuAppLaunchState, ShijuAppsLoadState, ShijuAppsState, ShijuAppsTab,
};

pub(crate) fn launch_shiju_app(
    target: &vw_gateway_client::ShijuAppLaunchDto,
    expected_installation_id: &str,
) -> Result<std::process::Child, crate::app::skill_hub::SkillHubError> {
    desktop_app_launch::launch_shiju_app(
        &crate::app::webview_runtime::webview_helper_candidates(),
        target,
        expected_installation_id,
    )
}
