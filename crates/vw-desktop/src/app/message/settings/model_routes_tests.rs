// 验证模型路由行的规范化、必填校验与错误定位。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use super::*;
use crate::app::App;
use crate::app::state::ModelRoute;

fn app() -> App {
    App::new().0
}

fn route(pattern: &str, provider: &str, model: &str, priority: &str) -> ModelRoute {
    ModelRoute {
        pattern: pattern.to_string(),
        provider: provider.to_string(),
        model: model.to_string(),
        priority_input: priority.to_string(),
    }
}

// 数据边界：覆盖规范化、默认值以及相关状态字段的既定规则。
// 这里固定可观察结果，避免依赖处理过程中的临时实现细节。
#[test]
fn validate_routes_skips_blank_rows_and_trims_complete_routes() {
    let mut app = app();
    app.model_routes_settings.routes =
        vec![route("  ", " ", "\t", ""), route("  coding  ", "  openai  ", "  gpt-5  ", "  7  ")];

    let routes = validate_routes(&app).expect("blank and complete routes should be valid");

    assert_eq!(routes.len(), 1);
    assert_eq!(routes[0].hint, "coding");
    assert_eq!(routes[0].provider, "openai");
    assert_eq!(routes[0].model, "gpt-5");
    assert_eq!(routes[0].max_tokens, None);
    assert_eq!(routes[0].api_key, None);
}

#[test]
fn validate_routes_accepts_complete_route_without_priority() {
    let mut app = app();
    app.model_routes_settings.routes =
        vec![route("  analysis  ", "  anthropic  ", "  claude-sonnet  ", "  ")];

    let routes = validate_routes(&app).expect("priority is optional");

    assert_eq!(routes.len(), 1);
    assert_eq!(routes[0].hint, "analysis");
    assert_eq!(routes[0].provider, "anthropic");
    assert_eq!(routes[0].model, "claude-sonnet");
}

// 数据边界：覆盖规范化、默认值以及相关状态字段的既定规则。
// 这里固定可观察结果，避免依赖处理过程中的临时实现细节。
#[test]
fn validate_routes_reports_provider_model_and_priority_errors_with_row_number() {
    let mut app = app();
    app.model_routes_settings.routes =
        vec![ModelRoute::default(), route("chat", " ", "gpt-5", "1")];
    assert_eq!(validate_routes(&app).unwrap_err(), "第 2 条路由缺少 provider");

    app.model_routes_settings.routes[1].provider = "openai".to_string();
    app.model_routes_settings.routes[1].model.clear();
    assert_eq!(validate_routes(&app).unwrap_err(), "第 2 条路由缺少 model");

    app.model_routes_settings.routes[1].model = "gpt-5".to_string();
    app.model_routes_settings.routes[1].priority_input = "-1".to_string();
    assert_eq!(validate_routes(&app).unwrap_err(), "第 2 条路由的 priority 不是有效数字");
}

#[test]
fn model_routes_validate_required_fields() {
    let mut app = app();
    app.model_routes_settings.routes = vec![ModelRoute {
        pattern: "  ".to_string(),
        provider: "openai".to_string(),
        model: "gpt-5".to_string(),
        priority_input: "1".to_string(),
    }];
    assert!(validate_routes(&app).unwrap_err().contains("pattern"));

    app.model_routes_settings.routes[0].pattern = "chat".to_string();
    app.model_routes_settings.routes[0].priority_input = "oops".to_string();
    assert!(validate_routes(&app).unwrap_err().contains("priority"));
}

#[test]
fn model_routes_update_syncs_query_classification_state() {
    let mut app = app();

    let _ = update(&mut app, SettingsMessage::ModelRoutes(ModelRoutesMessage::AddRoute));
    let _ = update(
        &mut app,
        SettingsMessage::ModelRoutes(ModelRoutesMessage::PatternChanged(0, "chat".to_string())),
    );
    let _ = update(
        &mut app,
        SettingsMessage::ModelRoutes(ModelRoutesMessage::ProviderChanged(0, "openai".to_string())),
    );
    let _ = update(
        &mut app,
        SettingsMessage::ModelRoutes(ModelRoutesMessage::ModelChanged(0, "gpt-5".to_string())),
    );
    let _ = update(
        &mut app,
        SettingsMessage::ModelRoutes(ModelRoutesMessage::PriorityChanged(0, "7".to_string())),
    );

    assert!(app.query_classification_settings.enabled);
    assert_eq!(app.query_classification_settings.rules.len(), 1);
    assert_eq!(app.query_classification_settings.rules[0].category, "chat");
    assert_eq!(app.model_routes_settings.routes[0].provider, "openai");
    assert_eq!(app.model_routes_settings.routes[0].model, "gpt-5");

    let _ = update(
        &mut app,
        SettingsMessage::ModelRoutes(ModelRoutesMessage::PriorityChanged(0, "bad".to_string())),
    );
    assert!(app.model_routes_settings.save_error.as_deref().unwrap_or("").contains("priority"));
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn invalid_route_edits_update_fields_and_expose_validation_errors() {
    let mut app = app();
    app.model_routes_settings.routes = vec![ModelRoute::default()];

    let _ = update(
        &mut app,
        SettingsMessage::ModelRoutes(ModelRoutesMessage::ProviderChanged(0, "openai".to_string())),
    );
    assert_eq!(app.model_routes_settings.routes[0].provider, "openai");
    assert_eq!(app.model_routes_settings.save_error.as_deref(), Some("第 1 条路由缺少 pattern"));

    let _ = update(
        &mut app,
        SettingsMessage::ModelRoutes(ModelRoutesMessage::PatternChanged(0, "chat".to_string())),
    );
    assert_eq!(app.model_routes_settings.routes[0].pattern, "chat");
    assert_eq!(app.model_routes_settings.save_error.as_deref(), Some("第 1 条路由缺少 model"));

    app.model_routes_settings.routes[0].provider.clear();
    let _ = update(
        &mut app,
        SettingsMessage::ModelRoutes(ModelRoutesMessage::ModelChanged(0, "gpt-5".to_string())),
    );
    assert_eq!(app.model_routes_settings.routes[0].model, "gpt-5");
    assert_eq!(app.model_routes_settings.save_error.as_deref(), Some("第 1 条路由缺少 provider"));

    app.model_routes_settings.routes[0].provider = "openai".to_string();
    let _ = update(
        &mut app,
        SettingsMessage::ModelRoutes(ModelRoutesMessage::PriorityChanged(0, "high".to_string())),
    );
    assert_eq!(app.model_routes_settings.routes[0].priority_input, "high");
    assert_eq!(
        app.model_routes_settings.save_error.as_deref(),
        Some("第 1 条路由的 priority 不是有效数字")
    );
}

#[test]
fn remove_route_updates_existing_rows_and_ignores_out_of_range_indexes() {
    let mut app = app();
    app.model_routes_settings.routes = vec![
        route("chat", "openai", "gpt-5", "3"),
        route("code", "anthropic", "claude-sonnet", "9"),
    ];
    app.model_routes_settings.save_error = Some(("stale".to_string()).into());

    let _ = update(&mut app, SettingsMessage::ModelRoutes(ModelRoutesMessage::RemoveRoute(9)));

    assert_eq!(app.model_routes_settings.routes.len(), 2);
    assert!(app.query_classification_settings.enabled);

    let _ = update(&mut app, SettingsMessage::ModelRoutes(ModelRoutesMessage::RemoveRoute(0)));

    assert_eq!(app.model_routes_settings.routes.len(), 1);
    assert_eq!(app.model_routes_settings.routes[0].pattern, "code");
    assert_eq!(app.query_classification_settings.rules.len(), 1);
    assert_eq!(app.query_classification_settings.rules[0].category, "code");

    let _ = update(&mut app, SettingsMessage::ModelRoutes(ModelRoutesMessage::RemoveRoute(0)));

    assert!(app.model_routes_settings.routes.is_empty());
    assert!(!app.query_classification_settings.enabled);
    assert!(app.query_classification_settings.rules.is_empty());
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn update_ignores_messages_for_other_settings_sections() {
    let mut app = app();
    app.model_routes_settings.routes = vec![route("chat", "openai", "gpt-5", "3")];
    app.model_routes_settings.save_error = Some(("keep".to_string()).into());

    let _ = update(&mut app, SettingsMessage::ToggleSettingsSidebar);

    assert_eq!(app.model_routes_settings.routes.len(), 1);
    assert_eq!(app.model_routes_settings.routes[0].pattern, "chat");
    assert_eq!(app.model_routes_settings.routes[0].provider, "openai");
    assert_eq!(app.model_routes_settings.routes[0].model, "gpt-5");
    assert_eq!(app.model_routes_settings.routes[0].priority_input, "3");
    assert_eq!(app.model_routes_settings.save_error.as_deref(), Some("keep"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn route_and_classification_edits_coalesce_without_blocking() {
    use super::super::messages::QueryClassificationMessage as Q;
    use super::super::query_classification;
    use iced_runtime::futures::futures::StreamExt;
    use serde_json::{Value, json};
    use std::time::{Duration, Instant};
    const TIMEOUT: Duration = Duration::from_secs(4);

    fn run(
        runtime: &tokio::runtime::Runtime,
        task: Task<Message>,
    ) -> tokio::task::JoinHandle<Message> {
        let mut stream = iced_runtime::task::into_stream(task).expect("background task");
        runtime.spawn(async move {
            while let Some(action) = stream.next().await {
                if let iced_runtime::Action::Output(message) = action {
                    return message;
                }
            }
            panic!("missing task result");
        })
    }
    fn finish(
        runtime: &tokio::runtime::Runtime,
        task: tokio::task::JoinHandle<Message>,
        app: &mut App,
    ) -> Task<Message> {
        let message =
            runtime.block_on(async { tokio::time::timeout(TIMEOUT, task).await }).unwrap().unwrap();
        let Message::Settings(message) = message else {
            panic!("settings result");
        };
        update(app, message)
    }
    fn patch(request: String) -> Value {
        assert!(request.starts_with("PATCH /v1/global/config "));
        serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap()
    }
    crate::app::config::with_editor_settings_gateway(|mut app, requests, responses| {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap();
        app.model_routes_settings.routes = vec![route("chat", "openai", "gpt-5", "1")];
        let started = Instant::now();
        let stale = update(
            &mut app,
            SettingsMessage::ModelRoutes(ModelRoutesMessage::PatternChanged(0, "cod".into())),
        );
        let _ = update(
            &mut app,
            SettingsMessage::ModelRoutes(ModelRoutesMessage::PatternChanged(0, "code".into())),
        );
        let _ = update(
            &mut app,
            SettingsMessage::ModelRoutes(ModelRoutesMessage::PriorityChanged(0, "invalid".into())),
        );
        let _ = update(&mut app, SettingsMessage::ModelRoutes(ModelRoutesMessage::Refresh));
        let latest = query_classification::update(
            &mut app,
            SettingsMessage::QueryClassification(Q::CategoryChanged(0, "latest".into())),
        );
        assert!(started.elapsed() < Duration::from_millis(200), "input blocked on gateway");
        assert!(requests.try_recv().is_err(), "typing must not GET or PATCH");
        assert!(
            iced_runtime::task::into_stream(finish(&runtime, run(&runtime, stale), &mut app))
                .is_none()
        );
        let save = finish(&runtime, run(&runtime, latest), &mut app);
        let first = run(&runtime, save);
        let first_patch = patch(requests.recv_timeout(TIMEOUT).unwrap());
        assert_eq!(first_patch["app_ui"]["system_settings"]["model_routes"][0]["pattern"], "code");
        assert_eq!(first_patch["app_ui"]["system_settings"]["model_routes"][0]["priority"], 1);
        assert_eq!(first_patch["model_routes"][0]["hint"], "code");
        assert_eq!(first_patch["query_classification"]["rules"][0]["hint"], "latest");
        assert_eq!(first_patch.as_object().unwrap().len(), 3);
        assert_eq!(first_patch["query_classification"].as_object().unwrap().len(), 2);

        // The first PATCH remains behind the HTTP gate while the next batch and
        // tab refresh are processed. Invalid drafts never replace valid snapshots.
        let started = Instant::now();
        let _ = query_classification::update(
            &mut app,
            SettingsMessage::QueryClassification(Q::AddRule),
        );
        let _ = query_classification::update(
            &mut app,
            SettingsMessage::QueryClassification(Q::RemoveRule(1)),
        );
        let pending = query_classification::update(
            &mut app,
            SettingsMessage::QueryClassification(Q::EnabledToggled(false)),
        );
        let _ = query_classification::update(
            &mut app,
            SettingsMessage::QueryClassification(Q::CategoryChanged(0, "".into())),
        );
        let _ = query_classification::update(
            &mut app,
            SettingsMessage::QueryClassification(Q::Refresh),
        );
        assert!(started.elapsed() < Duration::from_millis(200));
        assert!(
            iced_runtime::task::into_stream(finish(&runtime, run(&runtime, pending), &mut app))
                .is_none()
        );
        assert!(requests.try_recv().is_err(), "only one request may be in flight");
        responses.send(Some("{}".into())).unwrap();
        let second_task = finish(&runtime, first, &mut app);
        let second = run(&runtime, second_task);
        let second_patch = patch(requests.recv_timeout(TIMEOUT).unwrap());
        assert_eq!(second_patch.as_object().unwrap().len(), 1);
        assert_eq!(second_patch["query_classification"]["enabled"], false);
        assert_eq!(second_patch["query_classification"]["rules"][0]["hint"], "latest");
        responses.send(None).unwrap();
        let _ = finish(&runtime, second, &mut app);
        assert_eq!(app.query_classification_settings.rules[0].category, "");
        assert!(
            app.query_classification_settings.save_error.as_deref().unwrap().contains("category")
        );
        assert_eq!(app.model_routes_settings.routes[0].priority_input, "invalid");
        assert!(requests.try_recv().is_err(), "failed save must not retry indefinitely");

        // Explicit refresh retries the saved valid snapshot, retaining invalid UI text.
        let retry = run(
            &runtime,
            update(&mut app, SettingsMessage::ModelRoutes(ModelRoutesMessage::Refresh)),
        );
        let retry_patch = patch(requests.recv_timeout(TIMEOUT).unwrap());
        assert_eq!(retry_patch, second_patch);
        responses.send(Some("{}".into())).unwrap();
        let _ = finish(&runtime, retry, &mut app);
        assert_eq!(app.query_classification_settings.rules[0].category, "");

        // Read back via the real gateway client, including unrelated configuration.
        let mut persisted = json!({
            "model_routes": first_patch["model_routes"],
            "app_ui": {"system_settings": {"model_routes": first_patch["app_ui"]["system_settings"]["model_routes"], "terminal_font_size": 19}},
            "query_classification": second_patch["query_classification"],
            "embedding_routes": [{"hint": "keep", "provider": "local", "model": "embed"}]
        });
        persisted["query_classification"]["unmanaged"] = json!("keep");
        let reload = runtime.spawn(crate::app::config::load_global_config_value_async());
        assert!(requests.recv_timeout(TIMEOUT).unwrap().starts_with("GET /v1/global/config "));
        responses.send(Some(persisted.to_string())).unwrap();
        let loaded = runtime.block_on(reload).unwrap().unwrap();
        assert_eq!(loaded["query_classification"]["unmanaged"], "keep");
        let loaded: vw_config_types::config::Config = serde_json::from_value(loaded).unwrap();
        assert!(!loaded.query_classification.enabled);
        assert_eq!(loaded.query_classification.rules[0].hint, "latest");
        assert_eq!(loaded.model_routes[0].hint, "code");
        assert_eq!(loaded.embedding_routes[0].hint, "keep");
        assert_eq!(loaded.app_ui.system_settings.model_routes[0].priority, 1);
        assert_eq!(loaded.app_ui.system_settings.terminal_font_size, 19.0);
        eprintln!(
            "typing GET/PATCH=0/0; first batch PATCH=1; in-flight coalesced PATCH=1; retry PATCH=1; readback GET=1; latest classification=latest/disabled; invalid drafts preserved"
        );
    });
}
