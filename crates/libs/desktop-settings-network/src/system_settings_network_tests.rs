use super::*;
use iced::advanced::widget::Tree;
#[test]
fn network_save_error_banner_follows_tabs_for_each_network_page() {
    let mut app = Fixture::default();
    app.system_settings_tab = SystemTab::WebSearch;
    assert_eq!(active_save_error(&app.inputs()), None);
    assert_eq!(view(app.inputs()).as_widget().children().len(), 3);

    app.web_search_settings.save_error = Some("web search error".to_string());
    app.http_request_settings.save_error = Some("http request error".to_string());
    app.browser_settings.save_error = Some("browser error".to_string());
    app.proxy_settings.save_error = Some("proxy error".to_string());
    app.tunnel_settings.save_error = Some("tunnel error".to_string());

    for (tab, expected) in [
        (SystemTab::WebSearch, "web search error"),
        (SystemTab::HttpRequest, "http request error"),
        (SystemTab::Browser, "browser error"),
        (SystemTab::Proxy, "proxy error"),
        (SystemTab::Tunnel, "tunnel error"),
    ] {
        app.system_settings_tab = tab;
        assert_eq!(active_save_error(&app.inputs()), Some(expected));
        let element = view(app.inputs());
        let tree = Tree::new(element.as_widget());
        assert_eq!(tree.children.len(), 4);
        assert_eq!(tree.children[1].children.len(), NETWORK_TABS.len());
    }
}


struct Fixture { system_settings_tab: SystemTab, web_search_settings: crate::app_init::state::WebSearchSettingsState,http_request_settings: crate::app_init::state::HttpRequestSettingsState,browser_settings: crate::state::BrowserSettingsState,proxy_settings: crate::app_init::state::ProxySettingsState,tunnel_settings: crate::app_init::state::TunnelSettingsState, }
impl Default for Fixture { fn default() -> Self { Self { system_settings_tab: SystemTab::WebSearch,web_search_settings: Default::default(),http_request_settings: Default::default(),browser_settings: Default::default(),proxy_settings: Default::default(),tunnel_settings: Default::default(),} } }
impl Fixture { fn inputs(&self) -> View<'_> { View { system_settings_tab: self.system_settings_tab,web_search_settings: &self.web_search_settings,http_request_settings: &self.http_request_settings,browser_settings: &self.browser_settings,proxy_settings: &self.proxy_settings,tunnel_settings: &self.tunnel_settings, desktop_locale: config_ui::DesktopLocale::SimplifiedChinese, app_theme: &iced::Theme::Dark, last_copied_code_hash: None, render_help: |title, body, _, _| iced::widget::column![iced::widget::text(title), iced::widget::text(body)].into() } } }
