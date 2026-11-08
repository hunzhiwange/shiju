use super::system_settings_agent_apps::{refresh_message, status_text};
use crate::app::message::settings::AgentAppsMessage;
use crate::app::state::AgentAppsState;
use crate::app::{Message, message::SettingsMessage};

#[test]
fn cc_base_icon_desktop_list_empty() {
    assert_eq!(status_text(&AgentAppsState::default()), Some("暂无图标，请点击刷新读取图标库。"));
}
#[test]
fn cc_base_icon_desktop_list_loading() {
    let mut state = AgentAppsState::default();
    state.loading = true;
    assert_eq!(status_text(&state), Some("正在加载图标…"));
}
#[test]
fn cc_base_icon_desktop_list_failure_retry() {
    let mut state = AgentAppsState::default();
    state.error = Some("图标服务不可用".into());
    assert_eq!(status_text(&state), Some("图标服务不可用"));
    assert!(matches!(
        refresh_message(),
        Message::Settings(SettingsMessage::AgentApps(AgentAppsMessage::Refresh))
    ));
}
#[test]
fn cc_base_icon_desktop_list_chinese_name() {
    let mut state = AgentAppsState::default();
    state.icons.push(vw_gateway_client::vw_api_types::agent_apps::CustomIconEntry {
        id: "icon".into(),
        name: "工作图标".into(),
        data: String::new(),
        created_at: 0,
    });
    assert_eq!(status_text(&state), None);
    let _: iced::Element<'_, Message> = super::system_settings_agent_apps::view(
        &state,
        crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
        &iced::Theme::Light,
    );
}
