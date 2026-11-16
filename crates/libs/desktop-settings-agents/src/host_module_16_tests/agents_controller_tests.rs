//! 处理系统设置页面中对应功能区的消息、校验和配置持久化。

use super::agents_controller::{AgentsControllerFixture, handle_agents_config_saved, update};
use crate::agents_update::build_agents_settings_patch;
use crate::agents_update_message::{AgentsMessage, Message, SettingsMessage};
use crate::state::{DelegateAgentSettingsEntry, SkillsCatalogItem};
use config_agent::DelegateCapabilityPolicy;
use desktop_settings_skills::types::{SkillsCatalogKind, SkillsDirectoryScope};
use serde_json::Value;
use std::sync::Arc;

fn test_app() -> AgentsControllerFixture {
    AgentsControllerFixture {
        agents_settings: crate::agents_state::AgentsSettingsState {
            loading: false,
            config_ready: true,
            ..Default::default()
        },
        endpoint: gateway_endpoint::GatewayEndpoint::new("127.0.0.1", 1),
        project_path: None,
        effects: Vec::new(),
    }
}
fn entry<'a>(app: &'a AgentsControllerFixture, key: &str) -> &'a DelegateAgentSettingsEntry {
    app.agents_settings
        .entries
        .iter()
        .find(|entry| entry.key == key)
        .expect("missing agent settings entry")
}

fn entry_mut<'a>(
    app: &'a mut AgentsControllerFixture,
    key: &str,
) -> &'a mut DelegateAgentSettingsEntry {
    app.agents_settings
        .entries
        .iter_mut()
        .find(|entry| entry.key == key)
        .expect("missing agent settings entry")
}

fn skill_item(id: &str, source: &str, enabled: bool) -> SkillsCatalogItem {
    SkillsCatalogItem {
        id: id.to_string(),
        title: id.to_string(),
        description: String::new(),
        kind: SkillsCatalogKind::Personal,
        resource_count: 0,
        installed: true,
        enabled,
        source: source.to_string(),
        source_path: None,
    }
}

#[test]
fn agent_config_cleanup_skill_directory_preserves_global_draft() {
    let mut app = test_app();
    app.project_path = Some("/workspace".into());
    let _ = update(&mut app, SettingsMessage::Agents(AgentsMessage::ProfileOpen("main".into())));
    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::ProfileNameChanged("保留草稿".into())),
    );
    for path in ["/directory-a", "/directory-b"] {
        let _ =
            update(&mut app, SettingsMessage::Agents(AgentsMessage::ProjectSelected(path.into())));
        assert_eq!(app.project_path.as_deref(), Some("/workspace"));
        assert_eq!(app.agents_settings.profile_modal.as_ref().unwrap().label, "保留草稿");
        assert!(app.agents_settings.config_ready);
    }
}

static AGENT_CONFIG_CLEANUP_HTTP_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

struct AgentConfigCleanupGateway {
    _directory: tempfile::TempDir,
    endpoint: gateway_endpoint::GatewayEndpoint,
    requests: Arc<std::sync::Mutex<Vec<(String, String, Value)>>>,
    server: tokio::task::JoinHandle<()>,
}

impl AgentConfigCleanupGateway {
    async fn start() -> Self {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let directory = tempfile::tempdir().unwrap();
        let endpoint = gateway_endpoint::GatewayEndpoint::new("127.0.0.1", port);
        let requests = Arc::new(std::sync::Mutex::new(Vec::new()));
        let recorded = requests.clone();
        let server = tokio::spawn(async move {
            while let Ok((mut stream, _)) = listener.accept().await {
                let recorded = recorded.clone();
                tokio::spawn(async move {
                    let mut bytes = Vec::new();
                    let (header_end, length) = loop {
                        let mut buffer = [0; 4096];
                        let count = stream.read(&mut buffer).await.unwrap();
                        if count == 0 {
                            return;
                        }
                        bytes.extend_from_slice(&buffer[..count]);
                        if let Some(index) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                            let header = String::from_utf8_lossy(&bytes[..index]);
                            let length = header
                                .lines()
                                .find_map(|line| {
                                    let (name, value) = line.split_once(':')?;
                                    name.eq_ignore_ascii_case("content-length")
                                        .then(|| value.trim().parse::<usize>().unwrap())
                                })
                                .unwrap_or(0);
                            break (index + 4, length);
                        }
                    };
                    while bytes.len() < header_end + length {
                        let mut buffer = [0; 4096];
                        let count = stream.read(&mut buffer).await.unwrap();
                        if count == 0 {
                            return;
                        }
                        bytes.extend_from_slice(&buffer[..count]);
                    }
                    let header = String::from_utf8_lossy(&bytes[..header_end]);
                    let mut parts = header.split_whitespace();
                    let method = parts.next().unwrap().to_string();
                    let path = parts.next().unwrap().to_string();
                    let body = if length == 0 {
                        Value::Null
                    } else {
                        serde_json::from_slice(&bytes[header_end..header_end + length]).unwrap()
                    };
                    recorded.lock().unwrap().push((method.clone(), path.clone(), body));
                    if path.contains("directory-a") {
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    }
                    let body = if path.starts_with("/v1/skills") {
                        let id = if path.contains("directory-a") { "skill-a" } else { "skill-b" };
                        serde_json::json!([{"id":id,"title":"中文技能", "description":"目录候选", "kind":"personal", "resource_count":0,"installed":true,"enabled":true,"source":"workspace","source_path":null}])
                    } else if path == "/v1/tools" {
                        serde_json::json!({"items": [
                            {"id":"file_read", "identity":{"namespace":null,"name":"file_read"}, "source":"builtin", "exposure":"direct", "display_name":"Read", "description":"Read a file", "input_schema":{"type":"object"}}
                        ]})
                    } else if path.contains("/copy") {
                        {
                            let mut value =
                                serde_json::to_value(config_agent::DelegateAgentConfig::default())
                                    .unwrap();
                            value["key"] = "copy-agent".into();
                            value["label"] = "复制智能体".into();
                            value
                        }
                    } else if path.starts_with("/v1/agent") {
                        serde_json::json!([])
                    } else {
                        serde_json::json!({})
                    };
                    let body = body.to_string();
                    let response = format!(
                        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    stream.write_all(response.as_bytes()).await.unwrap();
                });
            }
        });
        Self { _directory: directory, endpoint, requests, server }
    }
}

impl Drop for AgentConfigCleanupGateway {
    fn drop(&mut self) {
        self.server.abort();
    }
}

async fn agent_config_cleanup_messages(task: iced::Task<Message>) -> Vec<Message> {
    use iced::futures::StreamExt;
    let Some(mut stream) = iced_runtime::task::into_stream(task.collect()) else {
        return vec![];
    };
    while let Some(action) = stream.next().await {
        if let iced_runtime::Action::Output(messages) = action {
            return messages;
        }
    }
    vec![]
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn agent_config_cleanup_global_mutations_do_not_use_skill_directory() {
    let _lock = AGENT_CONFIG_CLEANUP_HTTP_LOCK.lock().await;
    let mut app = test_app();
    let fixture = AgentConfigCleanupGateway::start().await;
    app.endpoint = fixture.endpoint.clone();
    app.project_path = Some("/workspace".into());
    app.agents_settings.skill_directory = Some("/directory-b".into());
    let mut custom = DelegateAgentSettingsEntry::from_config("custom-agent", None);
    custom.label = "智能体".into();
    app.agents_settings.entries.push(custom);
    for message in [
        AgentsMessage::CatalogToggle("custom-agent".into(), false),
        AgentsMessage::CatalogCopy("main".into()),
    ] {
        let task = update(&mut app, SettingsMessage::Agents(message));
        for result in agent_config_cleanup_messages(task).await {
            if let Message::Settings(message) = result {
                let _ = update(&mut app, message);
            }
        }
        assert!(!app.agents_settings.catalog_busy);
    }
    assert!(app.agents_settings.entries.iter().any(|entry| entry.key == "copy-agent"));
    for key in ["new-agent", "custom-agent", "copy-agent"] {
        if key == "new-agent" {
            app.agents_settings.new_agent_key_input = key.into();
            let _ = update(&mut app, SettingsMessage::Agents(AgentsMessage::AddAgentRequested));
        } else {
            let _ =
                update(&mut app, SettingsMessage::Agents(AgentsMessage::ProfileOpen(key.into())));
        }
        let _ = update(
            &mut app,
            SettingsMessage::Agents(AgentsMessage::ProfileNameChanged("保存草稿".into())),
        );
        for message in [AgentsMessage::ProfileSave, AgentsMessage::ProfileDelete] {
            let task = update(&mut app, SettingsMessage::Agents(message));
            assert!(app.agents_settings.profile_saving);
            for result in agent_config_cleanup_messages(task).await {
                if let Message::Settings(message) = result {
                    let _ = update(&mut app, message);
                }
            }
            // Reopen the saved draft to exercise deletion through the same transaction.
            if app.agents_settings.profile_modal.is_none() {
                app.agents_settings.profile_modal =
                    app.agents_settings.entries.iter().find(|e| e.key == key).cloned();
            }
        }
    }
    let recorded = fixture.requests.lock().unwrap();
    let requests = recorded
        .iter()
        .filter(|(method, _, _)| method == "PATCH" || method == "POST")
        .collect::<Vec<_>>();
    assert_eq!(requests.len(), 8, "create/edit/delete/toggle/copy must all reach isolated HTTP");
    assert!(
        requests
            .iter()
            .all(|(_, path, _)| !path.contains("directory") && !path.contains("project_path"))
    );
    assert_eq!(
        requests
            .iter()
            .filter(|(method, path, _)| method == "PATCH" && path == "/v1/global/config")
            .count(),
        7
    );
    assert!(
        requests.iter().any(|(method, path, _)| method == "POST" && path == "/v1/agent/main/copy")
    );
    assert!(requests.iter().any(|(_, _, body)| body["agents"]["new-agent"].is_object()));
    assert!(
        requests
            .iter()
            .any(|(_, _, body)| body["agents"].get("custom-agent").is_some_and(Value::is_null))
    );
    assert_eq!(app.project_path.as_deref(), Some("/workspace"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn agent_config_cleanup_stale_skill_catalog_is_ignored() {
    let _lock = AGENT_CONFIG_CLEANUP_HTTP_LOCK.lock().await;
    let mut app = test_app();
    let fixture = AgentConfigCleanupGateway::start().await;
    app.endpoint = fixture.endpoint.clone();
    let outer_catalog = vec![skill_item("outer-library", "global", true)];
    let outer_loading = true;
    let task_a = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::SkillDirectorySelected(Some("/directory-a".into()))),
    );
    let task_b = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::SkillDirectorySelected(Some("/directory-b".into()))),
    );
    let (a, b) =
        tokio::join!(agent_config_cleanup_messages(task_a), agent_config_cleanup_messages(task_b));
    for result in b.into_iter().chain(a) {
        if let Message::Settings(message) = result {
            let _ = update(&mut app, message);
        }
    }
    assert_eq!(app.agents_settings.skill_catalog[0].id, "skill-b");
    assert!(!app.agents_settings.skill_catalog_loading);
    assert_eq!(outer_catalog[0].id, "outer-library");
    assert!(outer_loading);
    assert_eq!(
        fixture
            .requests
            .lock()
            .unwrap()
            .iter()
            .filter(|(_, path, _)| path.starts_with("/v1/skills"))
            .count(),
        2
    );
    // Returning to the same directory still rejects a response from an earlier generation.
    let _ = update(&mut app, SettingsMessage::Agents(AgentsMessage::SkillCatalogRefresh));
    let generation = app.agents_settings.skill_catalog_generation;
    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::SkillCatalogLoaded {
            directory: Some("/directory-b".into()),
            generation: generation - 1,
            result: Err("旧请求错误".into()),
        }),
    );
    assert!(app.agents_settings.skill_catalog_error.is_none());
    assert!(app.agents_settings.skill_catalog_loading);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn agent_config_cleanup_profile_http_roundtrips_multiline_and_runtime() {
    let _lock = AGENT_CONFIG_CLEANUP_HTTP_LOCK.lock().await;
    let mut app = test_app();
    let fixture = AgentConfigCleanupGateway::start().await;
    app.endpoint = fixture.endpoint.clone();
    app.agents_settings.entries.push(DelegateAgentSettingsEntry::from_config("textarea", None));
    for message in [
        AgentsMessage::ProfileOpen("textarea".into()),
        AgentsMessage::ProfileNameChanged("多行智能体".into()),
        AgentsMessage::TemperatureChanged("textarea".into(), 1.2),
        AgentsMessage::ProfileDescriptionAction(iced::widget::text_editor::Action::SelectAll),
        AgentsMessage::ProfileDescriptionAction(iced::widget::text_editor::Action::Edit(
            iced::widget::text_editor::Edit::Paste(Arc::new("中文描述\n第二行\n".into())),
        )),
    ] {
        let _ = update(&mut app, SettingsMessage::Agents(message));
    }
    let task = update(&mut app, SettingsMessage::Agents(AgentsMessage::ProfileSave));
    assert!(app.agents_settings.profile_saving);
    for result in agent_config_cleanup_messages(task).await {
        if let Message::Settings(message) = result {
            let _ = update(&mut app, message);
        }
    }
    assert!(app.agents_settings.profile_modal.is_none());
    let mut payload = {
        let requests = fixture.requests.lock().unwrap();
        let (_, path, body) = requests.iter().find(|(method, _, _)| method == "PATCH").unwrap();
        assert_eq!(path, "/v1/global/config");
        body["agents"]["textarea"].clone()
    };
    assert!(payload.get("description_editor").is_none());
    // The HTTP fixture captures a Merge Patch: null deletes the ACP backend field.
    assert!(payload["runtime"]["backend"].is_null());
    payload["runtime"].as_object_mut().unwrap().remove("backend");
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("saved.json");
    std::fs::write(&path, serde_json::to_vec(&payload).unwrap()).unwrap();
    let config = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let restored = DelegateAgentSettingsEntry::from_config("textarea", Some(config));
    assert_eq!(restored.label, "多行智能体");
    assert_eq!(restored.description, "中文描述\n第二行\n");
    assert_eq!(restored.description_editor.text(), restored.description);
    assert_eq!(restored.temperature, 1.2);
}

#[test]
fn agent_config_cleanup_skill_selection_writes_only_one_default() {
    use config_agent_profile::AgentDefault;
    let mut app = test_app();
    app.agents_settings.skill_catalog = vec![skill_item("allowed", "global", true)];
    let agent = entry_mut(&mut app, "main");
    agent.allowed_skills = vec!["allowed".into()];
    agent.defaults.skills = AgentDefault::Fixed(vec![]);
    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::AllowedSkillToggled(
            "main".into(),
            "allowed".into(),
            true,
        )),
    );
    let agent = entry(&app, "main");
    assert_eq!(agent.allowed_skills, vec!["allowed"]);
    assert_eq!(agent.defaults.skills, AgentDefault::Fixed(vec!["allowed".into()]));
    let patch = crate::defaults::patch(&agent.defaults);
    assert_eq!(patch["skills"]["value"], serde_json::json!(["allowed"]));
}

#[test]
fn agent_config_cleanup_skill_selection_preserves_inherit_and_rejects_unavailable() {
    use config_agent_profile::AgentDefault;
    let mut app = test_app();
    app.agents_settings.skill_catalog = vec![
        skill_item("allowed", "global", true),
        skill_item("outside", "global", true),
        skill_item("disabled", "global", false),
    ];
    let agent = entry_mut(&mut app, "main");
    agent.allowed_skills = vec!["allowed".into()];
    agent.skill_policy = DelegateCapabilityPolicy::Strict;
    agent.defaults.skills = AgentDefault::Inherit;
    let toggle = |id: &str| {
        SettingsMessage::Agents(AgentsMessage::AllowedSkillToggled("main".into(), id.into(), true))
    };
    let _ = update(&mut app, toggle("allowed"));
    assert_eq!(entry(&app, "main").defaults.skills, AgentDefault::Inherit);
    entry_mut(&mut app, "main").defaults.skills = AgentDefault::Fixed(vec![]);
    for id in ["outside", "disabled", "unknown"] {
        let _ = update(&mut app, toggle(id));
        assert_eq!(entry(&app, "main").defaults.skills, AgentDefault::Fixed(vec![]));
        assert!(
            app.effects
                .iter()
                .any(|effect| matches!(effect, crate::agents_update_message::Effect::Toast(..)))
        );
    }
    let _ = update(
        &mut app,
        SettingsMessage::Agents(AgentsMessage::AllowedSkillsSelectAll(
            "main".into(),
            SkillsDirectoryScope::All,
        )),
    );
    assert_eq!(entry(&app, "main").defaults.skills, AgentDefault::Fixed(vec!["allowed".into()]));
    assert_eq!(entry(&app, "main").allowed_skills, vec!["allowed"]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn agent_tool_catalog_async_survives_provider_failure() {
    let _lock = AGENT_CONFIG_CLEANUP_HTTP_LOCK.lock().await;
    let mut app = test_app();
    let fixture = AgentConfigCleanupGateway::start().await;
    app.endpoint = fixture.endpoint.clone();
    app.agents_settings.available_tools.clear();
    let messages = agent_config_cleanup_messages(crate::agents_update::refresh_task(
        desktop_config::ConfigAccess::new(app.endpoint.clone(), None),
        vw_gateway_client::GatewayClient::new(app.endpoint.clone()),
    ))
    .await;
    assert!(messages.iter().any(|message| matches!(
        message,
        Message::Settings(SettingsMessage::Agents(AgentsMessage::Loaded(Err(_))))
    )));
    for message in messages {
        if let Message::Settings(message) = message {
            let _ = update(&mut app, message);
        }
    }
    assert_eq!(app.agents_settings.available_tools, vec!["file_read"]);
    assert!(fixture.requests.lock().unwrap().iter().any(|(_, path, _)| path == "/v1/tools"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn agent_tool_catalog_async_retries_on_profile_open() {
    let _lock = AGENT_CONFIG_CLEANUP_HTTP_LOCK.lock().await;
    let mut app = test_app();
    let _fixture = AgentConfigCleanupGateway::start().await;
    app.endpoint = _fixture.endpoint.clone();
    app.agents_settings.available_tools.clear();
    let task = update(&mut app, SettingsMessage::Agents(AgentsMessage::ProfileOpen("main".into())));
    for message in agent_config_cleanup_messages(task).await {
        if let Message::Settings(message) = message {
            let _ = update(&mut app, message);
        }
    }
    assert!(app.agents_settings.profile_modal.is_some());
    assert_eq!(app.agents_settings.available_tools, vec!["file_read"]);
}

#[test]
fn saved_config_stays_clean_after_discard_refresh_in_either_response_order() {
    for project_path in [None, Some("/project-a".to_string())] {
        for catalog_first in [false, true] {
            let mut app = test_app();
            app.project_path = project_path.clone();
            app.agents_settings.project_path = project_path.clone();
            let _ = update(
                &mut app,
                SettingsMessage::Agents(AgentsMessage::ModelChanged(
                    "main".into(),
                    "saved-model".into(),
                )),
            );
            let patch = build_agents_settings_patch(&app.agents_settings.entries).unwrap();
            let mut saved_config = config_aggregate::Config::default();
            for (key, value) in patch["agents"].as_object().unwrap() {
                if !value.is_null() {
                    saved_config
                        .agents
                        .insert(key.clone(), serde_json::from_value(value.clone()).unwrap());
                }
            }
            let _ = update(&mut app, SettingsMessage::Agents(AgentsMessage::Save));
            let _ = handle_agents_config_saved(&mut app, Ok(()));
            assert!(!app.agents_settings.dirty);

            for _ in 0..2 {
                let _ = update(&mut app, SettingsMessage::Agents(AgentsMessage::DiscardChanges));
                let catalog = AgentsMessage::Loaded(Ok((vec![], vec![], vec!["read".into()])));
                let config = AgentsMessage::ConfigLoaded {
                    project_path: None,
                    result: Ok(Box::new(saved_config.clone())),
                };
                let responses = if catalog_first { [catalog, config] } else { [config, catalog] };
                for response in responses {
                    let _ = update(&mut app, SettingsMessage::Agents(response));
                    assert!(!app.agents_settings.dirty);
                    assert!(!app.agents_settings.save_in_flight);
                }
                assert!(app.agents_settings.config_ready);
                assert_eq!(entry(&app, "main").model, "saved-model");
                assert!(entry(&app, "main").allowed_tools.iter().all(|tool| tool == "read"));
            }
            let _ = update(
                &mut app,
                SettingsMessage::Agents(AgentsMessage::ModelChanged("main".into(), "draft".into())),
            );
            let _ = update(
                &mut app,
                SettingsMessage::Agents(AgentsMessage::Loaded(Ok((
                    vec![],
                    vec![],
                    vec!["read".into()],
                )))),
            );
            assert!(app.agents_settings.dirty, "catalog refresh must preserve actual edits");
        }
    }
}
