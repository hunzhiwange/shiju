use config_ui::DesktopLocale;

#[tokio::test(flavor = "multi_thread")]
async fn desktop_gettext_area_14_hub_title_updates_after_locale_change() {
    let (mut app, _) = crate::app::App::new();
    crate::app::message::skill_hub::open_expert_hub_tab(&mut app);
    for (locale, expected) in [
        (DesktopLocale::SimplifiedChinese, "专家·技能·连接器"),
        (DesktopLocale::TraditionalChinese, "專家·技能·連接器"),
        (DesktopLocale::English, "Experts · Skills · Connectors"),
    ] {
        app.desktop_locale = locale;
        super::apps::refresh_hub_titles(&mut app);
        let tab = app
            .navigation_tabs.open_tabs
            .iter()
            .find(|tab| tab.id == crate::app::message::skill_hub::EXPERT_HUB_TAB_ID)
            .unwrap();
        assert_eq!(tab.title, expected);
    }
}
