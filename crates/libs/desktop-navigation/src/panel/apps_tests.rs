use super::*;

#[test]
fn third_party_agents_open_as_dedicated_tab_without_duplicates() {
    let mut tabs = vec![AppTab {
        id: "apps".into(),
        title: "应用".into(),
        screen: Screen::Apps,
        project_path: None,
    }];
    let mut active = Some("apps".into());
    let mut screen = Screen::Apps;
    let mut query = String::new();
    for _ in 0..2 {
        let effect = update(
            Context {
                open_tabs: &mut tabs,
                active_tab_id: &mut active,
                screen: &mut screen,
                apps_search_query: &mut query,
            },
            "workflow",
            None,
            Message::OpenThirdPartyAgents,
        );
        assert!(matches!(effect, Some(Effect::OpenThirdPartyAgents)));
        assert_eq!(screen, Screen::ThirdPartyAgents);
        assert_eq!(active.as_deref(), Some("third_party_agents"));
        assert_eq!(tabs.len(), 1);
        assert_eq!(tabs[0].screen, Screen::ThirdPartyAgents);
    }
}
