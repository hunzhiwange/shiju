// 验证设置标签切换、帮助面板与嵌套搜索目标导航。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use super::*;
use crate::app::{
    SettingsTab,
    components::system_settings_search::SystemSettingsSearchTarget,
    state::{AgentConfigTab, GeneralSettingsTab, McpSettingsTab, ModelConfigTab},
};

fn app() -> App {
    App::new().0
}

fn has_scheduled_action(task: Task<Message>) -> bool {
    task.units() > 0
}

async fn collect_task_messages(task: Task<Message>) -> Vec<Message> {
    use iced_runtime::futures::futures::StreamExt;

    let Some(mut stream) = iced_runtime::task::into_stream(task.collect()) else {
        return Vec::new();
    };
    while let Some(action) = stream.next().await {
        if let iced_runtime::Action::Output(messages) = action {
            return messages;
        }
    }
    Vec::new()
}

// 状态转换：覆盖交互前后的字段同步、选择关系与清理规则。
// 连续操作仍需保持模块不变量，并清除已经失效的瞬态反馈。
#[test]
fn top_level_tab_selection_updates_tab_and_closes_help() {
    let mut app = app();
    app.system_settings_help_tab = Some(SystemTab::General);

    let task = update(&mut app, SettingsMessage::TabSelected(SettingsTab::Files));

    assert_eq!(app.settings_tab, SettingsTab::Files);
    assert_eq!(app.system_settings_help_tab, None);
    assert!(!has_scheduled_action(task));
}

#[test]
fn query_help_and_sidebar_messages_update_only_their_ui_state() {
    let mut app = app();
    let original_sidebar_state = app.settings_sidebar_collapsed;

    assert!(!has_scheduled_action(update(
        &mut app,
        SettingsMessage::SystemTabQueryChanged(" runtime ".to_string()),
    )));
    assert_eq!(app.system_settings_query, " runtime ");

    assert!(!has_scheduled_action(update(
        &mut app,
        SettingsMessage::SystemHelpOpen(SystemTab::Runtime),
    )));
    assert_eq!(app.system_settings_help_tab, Some(SystemTab::Runtime));

    assert!(!has_scheduled_action(update(&mut app, SettingsMessage::SystemHelpClose)));
    assert_eq!(app.system_settings_help_tab, None);

    assert!(!has_scheduled_action(update(&mut app, SettingsMessage::ToggleSettingsSidebar,)));
    assert_eq!(app.settings_sidebar_collapsed, !original_sidebar_state);
}

#[tokio::test(flavor = "multi_thread")]
async fn system_settings_tabs_search_target_selection_opens_nested_tab_and_reuses_refresh() {
    let mut app = app();
    app.system_settings_query = "stdio".to_string();
    app.system_settings_help_tab = Some(SystemTab::General);
    app.system_settings_tab = SystemTab::General;
    app.agents_settings.active_config_tab = AgentConfigTab::Heartbeat;
    app.mcp_settings.active_tab = McpSettingsTab::Runtime;
    app.mcp_settings.loading = false;

    let messages = collect_task_messages(update(
        &mut app,
        SettingsMessage::SystemSearchTargetSelected(SystemSettingsSearchTarget::AgentMcp(
            McpSettingsTab::Servers,
        )),
    ))
    .await;

    assert_eq!(app.system_settings_tab, SystemTab::Agents);
    assert_eq!(app.agents_settings.active_config_tab, AgentConfigTab::Mcp);
    assert_eq!(app.mcp_settings.active_tab, McpSettingsTab::Servers);
    assert!(app.mcp_settings.loading);
    assert_eq!(app.system_settings_help_tab, None);
    assert_eq!(app.system_settings_query, "stdio");
    assert!(messages.iter().any(|message| matches!(
        message,
        Message::Settings(SettingsMessage::Mcp(McpMessage::Refresh))
    )));
}

#[test]
fn ordinary_system_tab_selection_changes_tab_closes_help_and_stays_idle() {
    let mut app = app();
    app.system_settings_help_tab = Some(SystemTab::Runtime);

    let task = update(&mut app, SettingsMessage::SystemTabSelected(SystemTab::Projects));

    assert_eq!(app.system_settings_tab, SystemTab::Projects);
    assert_eq!(app.system_settings_help_tab, None);
    assert!(!has_scheduled_action(task));
}

#[test]
fn legacy_editor_tab_selection_opens_general_editor_tab() {
    let mut app = app();

    let task = update(&mut app, SettingsMessage::SystemTabSelected(SystemTab::Editor));

    assert_eq!(app.system_settings_tab, SystemTab::General);
    assert_eq!(app.general_settings_tab, GeneralSettingsTab::Editor);
    assert!(!has_scheduled_action(task));
}

#[test]
fn legacy_themes_tab_selection_opens_general_themes_tab_and_refreshes() {
    let mut app = app();

    let task = update(&mut app, SettingsMessage::SystemTabSelected(SystemTab::Themes));

    assert_eq!(app.system_settings_tab, SystemTab::General);
    assert_eq!(app.general_settings_tab, GeneralSettingsTab::Themes);
    assert!(has_scheduled_action(task));
}

// 任务与结果：覆盖任务构造、调度或结果回写对公开状态的影响。
// 完成、失败与过期响应均按当前场景的既定契约处理。
#[test]
fn providers_and_models_selection_prepare_loading_before_refresh() {
    let mut app = app();
    app.provider_settings.loading = false;
    app.provider_settings.connect_error = Some("connect".to_string());
    app.provider_settings.save_error = Some("save".to_string());

    let providers_task = update(&mut app, SettingsMessage::SystemTabSelected(SystemTab::Providers));

    assert!(app.provider_settings.loading);
    assert_eq!(app.provider_settings.connect_error, None);
    assert_eq!(app.provider_settings.save_error, None);
    assert!(has_scheduled_action(providers_task));

    app.model_settings.loading = false;
    app.model_settings.save_error = Some("save".to_string());
    let models_task = update(&mut app, SettingsMessage::SystemTabSelected(SystemTab::Models));

    assert!(app.model_settings.loading);
    assert_eq!(app.model_settings.save_error, None);
    assert!(has_scheduled_action(models_task));
}

#[test]
fn multimodal_model_tab_refreshes_model_catalog() {
    let mut app = app();
    app.model_settings.loading = false;
    app.model_settings.save_error = Some("stale catalog error".to_string());
    app.multimodal_settings.save_error = Some("stale OCR error".to_string());
    app.multimodal_settings.ocr_model = "openai/vision".to_string();

    let task = update(
        &mut app,
        SettingsMessage::ModelConfigTab(ModelConfigTabMessage::Selected(
            ModelConfigTab::Multimodal,
        )),
    );

    assert_eq!(app.model_settings.active_tab, ModelConfigTab::Multimodal);
    assert!(app.model_settings.loading);
    assert_eq!(app.model_settings.save_error, None);
    assert_eq!(app.multimodal_settings.save_error, None);
    assert_eq!(app.multimodal_settings.ocr_model, "openai/vision");
    assert!(has_scheduled_action(task));
}

#[tokio::test(flavor = "multi_thread")]
async fn direct_refresh_tabs_emit_their_expected_messages() {
    let mut app = app();
    app.runtime_settings.save_error = Some("runtime".to_string());
    app.browser_settings.save_error = Some("browser".to_string());
    app.web_search_settings.save_error = Some("web_search".to_string());
    app.http_request_settings.save_error = Some("http_request".to_string());
    app.memory_settings.save_error = Some("memory".to_string());
    app.channels_settings.save_error = Some("channels".to_string());
    app.gateway_settings.save_error = Some("gateway".to_string());

    let cases = [
        (SystemTab::Runtime, "runtime"),
        (SystemTab::Browser, "browser"),
        (SystemTab::WebSearch, "web_search"),
        (SystemTab::HttpRequest, "http_request"),
        (SystemTab::Memory, "memory"),
        (SystemTab::Channels, "channels"),
        (SystemTab::Gateway, "gateway"),
    ];

    for (tab, expected) in cases {
        let messages =
            collect_task_messages(update(&mut app, SettingsMessage::SystemTabSelected(tab))).await;

        assert_eq!(messages.len(), 1, "unexpected message count for {expected}");
        let matches_expected = matches!(
            (expected, &messages[0]),
            ("runtime", Message::Settings(SettingsMessage::Runtime(RuntimeMessage::Refresh)))
                | ("browser", Message::Settings(SettingsMessage::Browser(BrowserMessage::Refresh)))
                | (
                    "web_search",
                    Message::Settings(SettingsMessage::WebSearch(WebSearchMessage::Refresh))
                )
                | (
                    "http_request",
                    Message::Settings(SettingsMessage::HttpRequest(HttpRequestMessage::Refresh))
                )
                | ("memory", Message::Settings(SettingsMessage::Memory(MemoryMessage::Refresh)))
                | (
                    "channels",
                    Message::Settings(SettingsMessage::Channels(ChannelsMessage::Refresh))
                )
                | ("gateway", Message::Settings(SettingsMessage::Gateway(GatewayMessage::Refresh)))
        );
        assert!(matches_expected, "unexpected refresh message for {expected}");
    }

    assert_eq!(app.runtime_settings.save_error, None);
    assert_eq!(app.browser_settings.save_error, None);
    assert_eq!(app.web_search_settings.save_error, None);
    assert_eq!(app.http_request_settings.save_error, None);
    assert_eq!(app.memory_settings.save_error, None);
    assert_eq!(app.channels_settings.save_error, None);
    assert_eq!(app.gateway_settings.save_error, None);
}

#[tokio::test(flavor = "multi_thread")]
async fn model_config_tab_selection_switches_internal_tab_and_refreshes_its_data() {
    let mut app = app();

    let routes = collect_task_messages(update(
        &mut app,
        SettingsMessage::ModelConfigTab(ModelConfigTabMessage::Selected(ModelConfigTab::ModelRoutes)),
    ))
    .await;
    assert_eq!(app.model_settings.active_tab, ModelConfigTab::ModelRoutes);
    assert!(matches!(
        routes.as_slice(),
        [Message::Settings(SettingsMessage::ModelRoutes(ModelRoutesMessage::Refresh))]
    ));

    let classification = collect_task_messages(update(
        &mut app,
        SettingsMessage::ModelConfigTab(ModelConfigTabMessage::Selected(
            ModelConfigTab::QueryClassification,
        )),
    ))
    .await;
    assert_eq!(app.model_settings.active_tab, ModelConfigTab::QueryClassification);
    assert!(matches!(
        classification.as_slice(),
        [Message::Settings(SettingsMessage::ModelRoutes(ModelRoutesMessage::Refresh))]
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn agent_config_tab_selection_updates_state_and_refreshes_dialogue_flow() {
    let mut app = app();

    app.channels_settings.save_error = Some("channels".to_string());
    let channel_messages = collect_task_messages(update(
        &mut app,
        SettingsMessage::AgentConfigTab(AgentConfigTabMessage::Selected(
            AgentConfigTab::Channels,
        )),
    ))
    .await;
    assert_eq!(app.agents_settings.active_config_tab, AgentConfigTab::Channels);
    assert_eq!(app.channels_settings.save_error, None);
    assert!(matches!(
        channel_messages.as_slice(),
        [Message::Settings(SettingsMessage::Channels(ChannelsMessage::Refresh))]
    ));

    app.acp_settings.loading = false;
    app.acp_settings.save_error = Some("acp".to_string());
    let acp_messages = collect_task_messages(update(
        &mut app,
        SettingsMessage::AgentConfigTab(AgentConfigTabMessage::Selected(AgentConfigTab::Acp)),
    ))
    .await;
    assert!(app.acp_settings.loading);
    assert_eq!(app.acp_settings.save_error, None);
    assert!(matches!(
        acp_messages.as_slice(),
        [Message::Settings(SettingsMessage::Acp(AcpMessage::Refresh))]
    ));

    app.mcp_settings.loading = false;
    app.mcp_settings.save_error = Some("mcp".to_string());
    let mcp_messages = collect_task_messages(update(
        &mut app,
        SettingsMessage::AgentConfigTab(AgentConfigTabMessage::Selected(AgentConfigTab::Mcp)),
    ))
    .await;
    assert!(app.mcp_settings.loading);
    assert_eq!(app.mcp_settings.save_error, None);
    assert!(matches!(
        mcp_messages.as_slice(),
        [Message::Settings(SettingsMessage::Mcp(McpMessage::Refresh))]
    ));

    app.skills_settings.loading = false;
    app.skills_settings.save_error = Some("skills".to_string());
    let skills_messages = collect_task_messages(update(
        &mut app,
        SettingsMessage::AgentConfigTab(AgentConfigTabMessage::Selected(AgentConfigTab::Skills)),
    ))
    .await;
    assert!(app.skills_settings.loading);
    assert_eq!(app.skills_settings.save_error, None);
    assert!(matches!(
        skills_messages.as_slice(),
        [Message::Settings(SettingsMessage::SkillsRefresh)]
    ));

    app.skills_settings.plugin_loading = false;
    let plugin_messages = collect_task_messages(update(
        &mut app,
        SettingsMessage::AgentConfigTab(AgentConfigTabMessage::Selected(AgentConfigTab::Plugins)),
    ))
    .await;
    assert_eq!(app.agents_settings.active_config_tab, AgentConfigTab::Plugins);
    assert!(app.skills_settings.plugin_loading);
    assert!(matches!(
        plugin_messages.as_slice(),
        [Message::Settings(SettingsMessage::PluginsRefresh)]
    ));

    app.runtime_settings.save_error = Some("runtime".to_string());
    let runtime_messages = collect_task_messages(update(
        &mut app,
        SettingsMessage::AgentConfigTab(AgentConfigTabMessage::Selected(
            AgentConfigTab::Runtime,
        )),
    ))
    .await;
    assert_eq!(app.agents_settings.active_config_tab, AgentConfigTab::Runtime);
    assert_eq!(app.runtime_settings.save_error, None);
    assert!(matches!(
        runtime_messages.as_slice(),
        [Message::Settings(SettingsMessage::Runtime(RuntimeMessage::Refresh))]
    ));

    app.reliability_settings.save_error = Some("reliability".to_string());
    let reliability_task = update(
        &mut app,
        SettingsMessage::AgentConfigTab(AgentConfigTabMessage::Selected(
            AgentConfigTab::Reliability,
        )),
    );
    assert_eq!(app.reliability_settings.save_error.as_deref(), Some("reliability"));
    assert!(!has_scheduled_action(reliability_task));

    let messages = collect_task_messages(update(
        &mut app,
        SettingsMessage::AgentConfigTab(AgentConfigTabMessage::Selected(
            AgentConfigTab::DialogueFlow,
        )),
    ))
    .await;

    assert_eq!(app.agents_settings.active_config_tab, AgentConfigTab::DialogueFlow);
    assert!(matches!(
        messages.as_slice(),
        [Message::Settings(SettingsMessage::DialogueFlowPermissionRefresh)]
    ));

    app.heartbeat_settings.save_error = Some("stale".to_string());
    let heartbeat_task = update(
        &mut app,
        SettingsMessage::AgentConfigTab(AgentConfigTabMessage::Selected(
            AgentConfigTab::Heartbeat,
        )),
    );
    assert_eq!(app.agents_settings.active_config_tab, AgentConfigTab::Heartbeat);
    assert_eq!(app.heartbeat_settings.save_error, None);
    assert!(!has_scheduled_action(heartbeat_task));
}

#[tokio::test(flavor = "multi_thread")]
async fn legacy_channels_system_tab_redirects_to_agent_channels_tab() {
    let mut app = app();

    let messages = collect_task_messages(update(
        &mut app,
        SettingsMessage::SystemTabSelected(SystemTab::Channels),
    ))
    .await;

    assert_eq!(app.system_settings_tab, SystemTab::Agents);
    assert_eq!(app.agents_settings.active_config_tab, AgentConfigTab::Channels);
    assert!(matches!(
        messages.as_slice(),
        [Message::Settings(SettingsMessage::Channels(ChannelsMessage::Refresh))]
    ));
}

// 行为契约：使用代表性场景固定该分支的可观察结果。
// 断言聚焦输入、状态与返回任务，不复述内部控制流程。
#[tokio::test(flavor = "multi_thread")]
async fn legacy_runtime_system_tab_redirects_to_agent_runtime_tab() {
    let mut app = app();

    let messages = collect_task_messages(update(
        &mut app,
        SettingsMessage::SystemTabSelected(SystemTab::Runtime),
    ))
    .await;

    assert_eq!(app.system_settings_tab, SystemTab::Agents);
    assert_eq!(app.agents_settings.active_config_tab, AgentConfigTab::Runtime);
    assert!(matches!(
        messages.as_slice(),
        [Message::Settings(SettingsMessage::Runtime(RuntimeMessage::Refresh))]
    ));
}

#[test]
fn state_only_system_tabs_clear_stale_feedback_without_scheduling_work() {
    let mut app = app();
    app.multimodal_settings.save_error = Some("multimodal".to_string());
    app.tunnel_settings.save_error = Some("tunnel".to_string());

    assert!(!has_scheduled_action(update(
        &mut app,
        SettingsMessage::SystemTabSelected(SystemTab::Tunnel),
    )));

    assert_eq!(app.tunnel_settings.save_error, None);
}

#[tokio::test(flavor = "multi_thread")]
async fn cron_selection_clears_error_and_requests_jobs_and_agents() {
    let mut app = app();
    app.cron_settings.save_error = Some("cron".to_string());
    app.agents_settings.loading = false;

    let messages = collect_task_messages(update(
        &mut app,
        SettingsMessage::SystemTabSelected(SystemTab::Cron),
    ))
    .await;

    assert_eq!(app.cron_settings.save_error, None);
    assert!(app.agents_settings.loading);
    assert_eq!(messages.len(), 2);
    assert!(
        messages
            .iter()
            .any(|message| matches!(message, Message::Settings(SettingsMessage::CronJobsRefresh)))
    );
    assert!(messages.iter().any(|message| matches!(
        message,
        Message::Settings(SettingsMessage::Agents(AgentsMessage::Refresh))
    )));
}

#[tokio::test(flavor = "multi_thread")]
async fn agents_selection_prepares_state_and_requests_agents_and_skills() {
    let mut app = app();
    app.agents_settings.loading = false;
    app.agents_settings.save_error = Some("agents".to_string());
    app.skills_settings.loading = false;
    app.agents_settings.active_config_tab = AgentConfigTab::Heartbeat;

    let messages = collect_task_messages(update(
        &mut app,
        SettingsMessage::SystemTabSelected(SystemTab::Agents),
    ))
    .await;

    assert!(app.agents_settings.loading);
    assert_eq!(app.agents_settings.active_config_tab, AgentConfigTab::Agents);
    assert_eq!(app.agents_settings.save_error, None);
    assert!(app.skills_settings.loading);
    assert_eq!(messages.len(), 2);
    assert!(messages.iter().any(|message| matches!(
        message,
        Message::Settings(SettingsMessage::Agents(AgentsMessage::Refresh))
    )));
    assert!(
        messages
            .iter()
            .any(|message| matches!(message, Message::Settings(SettingsMessage::SkillsRefresh)))
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn staff_selection_resets_feedback_and_requests_all_dependencies() {
    let mut app = app();
    app.staff_settings.loading = false;
    app.staff_settings.save_error = Some("staff".to_string());
    app.staff_settings.save_success = true;
    app.agents_settings.loading = false;
    app.skills_settings.loading = false;

    let messages = collect_task_messages(update(
        &mut app,
        SettingsMessage::SystemTabSelected(SystemTab::Staff),
    ))
    .await;

    assert!(app.staff_settings.loading);
    assert_eq!(app.staff_settings.save_error, None);
    assert!(!app.staff_settings.save_success);
    assert!(app.agents_settings.loading);
    assert!(app.skills_settings.loading);
    assert_eq!(messages.len(), 4);
    assert!(messages.iter().any(|message| matches!(
        message,
        Message::Settings(SettingsMessage::Staff(StaffMessage::Refresh))
    )));
    assert!(messages.iter().any(|message| matches!(
        message,
        Message::Settings(SettingsMessage::Staff(StaffMessage::Resources(
            StaffResourceMessage::Refresh
        )))
    )));
    assert!(messages.iter().any(|message| matches!(
        message,
        Message::Settings(SettingsMessage::Agents(AgentsMessage::Refresh))
    )));
    assert!(
        messages
            .iter()
            .any(|message| matches!(message, Message::Settings(SettingsMessage::SkillsRefresh)))
    );
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn unhandled_settings_message_is_rejected_without_state_change_or_task() {
    let mut app = app();
    app.system_settings_query = "unchanged".to_string();

    let task = update(&mut app, SettingsMessage::SkillsRefresh);

    assert_eq!(app.system_settings_query, "unchanged");
    assert!(!has_scheduled_action(task));
}
