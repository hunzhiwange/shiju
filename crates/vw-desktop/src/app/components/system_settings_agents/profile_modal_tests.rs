//! Host contracts: settings update dispatch, persisted/draft projection, ACP directory refresh and actual modal rendering.
use crate::app::{
    App,
    message::settings::{AgentsMessage, SettingsMessage},
};

fn dispatch(app: &mut App, message: AgentsMessage) {
    let _ = crate::app::message::settings::update(app, SettingsMessage::Agents(message));
}

fn runtime_creation_draft() -> App {
    let mut app = App::new().0;
    app.agents_settings.config_ready = true;
    app.acp_agents = vec!["codex".into(), "claude".into()];
    dispatch(&mut app, AgentsMessage::AddAgentKeyChanged("runtime-custom".into()));
    dispatch(&mut app, AgentsMessage::AddAgentRequested);
    app
}

#[test]
fn agent_runtime_creation_empty_directory_refreshes_and_failure_preserves_native() {
    use crate::app::message::settings::AcpMessage;
    let mut app = App::new().0;
    app.agents_settings.config_ready = true;
    app.acp_agents.clear();
    app.acp_settings.loading = false;
    dispatch(&mut app, AgentsMessage::AddAgentKeyChanged("empty-catalog".into()));
    dispatch(&mut app, AgentsMessage::AddAgentRequested);
    assert!(app.acp_settings.loading);
    let _ = crate::app::message::settings::update(
        &mut app,
        SettingsMessage::Acp(AcpMessage::Loaded(Err("目录加载失败".into()))),
    );
    assert!(!app.acp_settings.loading);
    assert!(app.acp_settings.save_error.is_some());
    dispatch(&mut app, AgentsMessage::ProfileRuntimeChanged("codex".into()));
    assert_eq!(
        app.agents_settings.profile_modal.as_ref().unwrap().runtime,
        config_agent_profile::AgentRuntime::Native
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn settings_tabs_capabilities_appearance_profile() {
    use crate::app::Message;
    use crate::app::components::system_settings_common::tab_appearance_tests;
    use crate::app::i18n::I18n;
    use crate::app::state::{
        AGENT_DETAIL_BASIC_TAB, AGENT_DETAIL_IDENTITY_TAB, AGENT_PROMPT_SYSTEM_TAB,
        WORKSPACE_IDENTITY_FILES,
    };

    fn view(app: &App) -> iced::Element<'_, Message> {
        // A uniform surface keeps the underlying page from tinting the translucent card.
        let base = iced::widget::Space::new().width(iced::Length::Fill).height(iced::Length::Fill);
        super::profile_modal::overlay(app, base.into())
    }

    tab_appearance_tests::run(
        concat!(module_path!(), "::settings_tabs_capabilities_appearance_profile"),
        runtime_creation_draft,
        |scene, app| {
            let locale = app.desktop_locale;
            let tr = |label| super::tr(locale, label);
            let draft_name = "暂存智能体 · Profile draft";
            dispatch(app, AgentsMessage::ProfileNameChanged(draft_name.into()));

            let native_tabs = ["基础参数", "身份", "工具", "技能", "MCP"].map(tr);
            let identity = scene.check(app, view(app), &native_tabs, 1);
            assert!(
                matches!(&identity, SettingsMessage::Agents(AgentsMessage::DetailTabSelected(key))
                if key == AGENT_DETAIL_IDENTITY_TAB)
            );
            let _ = app.update(Message::Settings(identity));
            assert_eq!(app.agents_settings.active_detail_tab, AGENT_DETAIL_IDENTITY_TAB);
            app.agents_settings.active_prompt_tab = AGENT_PROMPT_SYSTEM_TAB.into();

            let prompt_tabs: Vec<_> = std::iter::once(tr("系统提示词"))
                .chain(
                    WORKSPACE_IDENTITY_FILES.iter().map(|(_, label)| I18n::new(locale).__(label)),
                )
                .collect();
            let prompt = scene.check(app, view(app), &prompt_tabs, 1);
            let file = WORKSPACE_IDENTITY_FILES[0].0;
            assert!(
                matches!(&prompt, SettingsMessage::Agents(AgentsMessage::PromptTabSelected(key))
                if key == file)
            );
            let _ = app.update(Message::Settings(prompt));
            assert_eq!(app.agents_settings.active_prompt_tab, file);
            assert!(scene.labels(view(app)).contains(tr("恢复默认")));
            scene.check_modal_scroll(view(app), tr("智能体配置"), tr("保存配置"));

            let basic = scene.click(view(app), tr("基础参数"));
            let _ = app.update(Message::Settings(basic));
            assert_eq!(app.agents_settings.active_detail_tab, AGENT_DETAIL_BASIC_TAB);
            assert_eq!(app.agents_settings.profile_modal.as_ref().unwrap().label, draft_name);

            dispatch(app, AgentsMessage::ProfileRuntimeChanged("codex".into()));
            let acp_tabs = ["基础参数", "身份", "技能", "MCP"].map(tr);
            assert!(!scene.labels(view(app)).contains(tr("工具")));
            let skills = scene.check(app, view(app), &acp_tabs, 2);
            let _ = app.update(Message::Settings(skills));
            assert_eq!(
                app.agents_settings.active_detail_tab,
                crate::app::state::AGENT_DETAIL_SKILLS_TAB
            );
            assert_eq!(app.agents_settings.profile_modal.as_ref().unwrap().label, draft_name);
        },
    )
    .await;
}
