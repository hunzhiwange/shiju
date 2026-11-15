//! 保存回执在所属领域更新反馈状态；错误文本由配置边界规范化。

pub fn proxy_saved(state: &mut crate::app_init::state::ProxySettingsState, error: Option<String>) {
    state.save_error = error;
}

pub fn browser_saved(state: &mut crate::state::BrowserSettingsState, error: Option<String>) {
    state.save_error = error;
}

pub fn http_request_saved(
    state: &mut crate::app_init::state::HttpRequestSettingsState,
    error: Option<String>,
) {
    state.save_error = error;
}

pub fn web_search_saved(
    state: &mut crate::app_init::state::WebSearchSettingsState,
    error: Option<String>,
) {
    state.save_error = error;
}

pub fn tunnel_saved(
    state: &mut crate::app_init::state::TunnelSettingsState,
    error: Option<String>,
) {
    state.save_error = error;
}

pub fn gateway_saved(state: &mut crate::state::GatewaySettingsState, error: Option<String>) {
    state.save_error = error.map(desktop_i18n::settings_models::messages::Feedback::gateway);
}
