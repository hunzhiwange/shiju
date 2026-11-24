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


#[test]
fn desktop_gettext_apps_visible_copy() {
    use crate::{app_routing::{lifecycle, requests}, diagnostics::model_panel};
    use DesktopLocale::*;
    let state = lifecycle::State::default();
    let status = vw_api_app_routing::RoutingStatus {
        state: vw_api_app_routing::RoutingState::Running,
        bound_address: Some("127.0.0.1".parse().unwrap()),
        bound_port: Some(43123),
        started_at: None,
    };
    let task = "raw/task/%s/中文";
    let correlation = "raw-correlation/%n/繁體";
    for (locale, title, waiting, active, task_title, correlation_title) in [
        (SimplifiedChinese, "路由服务", "等待连接执行主机", "进行中", "任务：", "关联："),
        (English, "Routing service", "Waiting for execution host connection", "In progress", "Task: ", "Correlation: "),
        (TraditionalChinese, "路由服務", "等待連線執行主機", "進行中", "任務：", "關聯："),
    ] {
        let i18n = I18n::new(locale);
        assert_eq!(lifecycle::service_title(i18n), title);
        assert_eq!(lifecycle::service_address(Some(&status)).as_deref(), Some("127.0.0.1:43123"));
        assert_eq!(lifecycle::service_label(i18n, &state, None), waiting);
        assert_eq!(requests::request_label(i18n, vw_api_app_routing::RoutingRequestState::Active), active);
        let label = model_panel::job_label(i18n, task, correlation);
        assert_eq!(label, format!("{task_title}{task} · {correlation_title}{correlation}"));
        // Exercise the real rendering entry without granting a connection or sending a request.
        let _view = lifecycle::view(i18n, &state, None);
    }
}

#[test]
fn desktop_gettext_apps_feedback_keeps_raw_service_data() {
    use desktop_i18n::format::Message;
    let mut notice = None;
    let _: iced::Task<()> = crate::error(&mut notice, "请先连接应用服务");
    let Some(crate::Notice::Error(message)) = notice.take() else { panic!("missing product feedback") };
    assert_eq!(message.render(I18n::new(DesktopLocale::English)).unwrap(), "Connect to the app service first");
    let raw = "服务数据/%s/%n/原样".to_owned();
    let _: iced::Task<()> = crate::error(&mut notice, raw.clone());
    let Some(crate::Notice::Error(Message::Literal(value))) = notice.take() else { panic!("raw data became a template") };
    assert_eq!(value, raw);
}
