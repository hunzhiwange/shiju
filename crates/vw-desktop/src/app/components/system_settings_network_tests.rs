//! 验证网络类设置页切换时保存错误提示与当前标签保持一致。
//!
//! 同一组错误状态会轮换所有网络标签，确认横幅只读取当前页面的数据。

use super::*;
use iced::advanced::widget::Tree;

#[test]
fn network_save_error_banner_follows_tabs_for_each_network_page() {
    let mut app = App::new().0;
    app.system_settings_tab = SystemTab::WebSearch;
    assert_eq!(active_save_error(&app), None);
    assert_eq!(view(&app).as_widget().children().len(), 3);

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
        assert_eq!(active_save_error(&app), Some(expected));
        let element = view(&app);
        let tree = Tree::new(element.as_widget());
        assert_eq!(tree.children.len(), 4);
        assert_eq!(tree.children[1].children.len(), NETWORK_TABS.len());
    }
}
