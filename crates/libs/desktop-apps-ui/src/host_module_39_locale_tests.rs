use desktop_i18n::I18n;
use config_ui::DesktopLocale;

#[test]
fn desktop_gettext_area_14_launch_error_metadata_rejects_stale_generation() {
    use desktop_app_launch::{ShijuAppLaunchState, ShijuAppsState};
    let source = "当前账户没有已启用的 IAM API 密钥";
    let mut state = ShijuAppsState::default();
    state.launch = ShijuAppLaunchState::Error {
        generation: 7,
        installation_id: "raw/%n".into(),
        message: source.into(),
    };
    state.launch_feedback =
        Some((7, source.into(), desktop_skill_hub::SkillHubError::localized(source, vec![])));
    assert_eq!(
        state.render_launch_error(source, I18n::new(DesktopLocale::English)),
        "No enabled IAM API keys for this account"
    );
    assert_eq!(
        state.render_launch_error(source, I18n::new(DesktopLocale::TraditionalChinese)),
        "目前帳戶沒有已啟用的 IAM API 金鑰"
    );
    state.launch_feedback =
        Some((6, source.into(), desktop_skill_hub::SkillHubError::localized(source, vec![])));
    assert_eq!(state.render_launch_error(source, I18n::new(DesktopLocale::English)), source);
    let raw = "服务错误/%n/%s/中文";
    assert_eq!(state.render_launch_error(raw, I18n::new(DesktopLocale::English)), raw);
}
