//! 验证 ACP 请求选项、模型和消息载荷的构造规则。

use std::collections::HashMap;

use serde_json::json;

use crate::app::agent::provider::provider;
use crate::app::agent::session::message;

use super::do_stream_request_acp;

fn test_model(provider_id: &str) -> provider::Model {
    serde_json::from_value(json!({
        "id": "gpt-5-mini",
        "providerID": provider_id,
        "api": {
            "id": "gpt-5-mini",
            "url": "http://localhost",
            "adapter": "openai-compatible"
        },
        "name": "gpt-5-mini",
        "family": null,
        "capabilities": {
            "temperature": true,
            "reasoning": true,
            "attachment": false,
            "toolcall": true,
            "input": {
                "text": true,
                "audio": false,
                "image": false,
                "video": false,
                "pdf": false
            },
            "output": {
                "text": true,
                "audio": false,
                "image": false,
                "video": false,
                "pdf": false
            },
            "interleaved": false
        },
        "cost": {
            "input": 0.0,
            "output": 0.0,
            "cache": {
                "read": 0.0,
                "write": 0.0
            },
            "experimental_over_200k": null
        },
        "limit": {
            "context": 8192,
            "input": null,
            "output": 4096
        },
        "status": "active",
        "options": {},
        "headers": {},
        "release_date": "2026-01-01",
        "variants": {}
    }))
    .expect("test model should deserialize")
}

fn test_provider(key: Option<&str>) -> provider::Info {
    provider::Info {
        id: "github-copilot".to_string(),
        name: "GitHub Copilot".to_string(),
        source: provider::ProviderSource::Api,
        env: Vec::new(),
        key: key.map(ToString::to_string),
        options: HashMap::new(),
        models: HashMap::new(),
    }
}

#[cfg(unix)]
#[tokio::test]
async fn acp_model_source_launch_profile_production_request_rejects_unverified_driver() {
    let _lock = crate::test_support::global_async_env_lock().lock().await;
    let directory = tempfile::tempdir().unwrap();
    let script = directory.path().join("unexpected-launch.py");
    let launched = directory.path().join("launched");
    std::fs::write(
        &script,
        "import pathlib, sys\npathlib.Path(sys.argv[1]).write_text('unexpected launch')\n",
    )
    .unwrap();
    let tenant = crate::session::ui_store::current_tenant_scope();
    let (project, _) = crate::project::from_directory(&tenant, directory.path()).await.unwrap();
    crate::config::update_project_for_tenant(&tenant, directory.path(), json!({
        "providers": {"github-copilot": {"api": "http://127.0.0.1:1/v1", "models": {"gpt-5-mini": {"status": "active"}}}},
        "acp": {"unverified-fixture": {"command": "python3", "args": [&script, &launched]}}
    })).await.unwrap();
    let context = crate::project::instance::Context {
        directory: directory.path().to_string_lossy().into(),
        worktree: project.worktree.clone(),
        project,
    };
    // A synthetic driver stays unverified as reviewed CLI support expands.
    let result = crate::project::instance::provide_context(
        context,
        do_stream_request_acp(
            &test_model("github-copilot"),
            &test_provider(Some("provider-secret-never-distributed")),
            None,
            &json!({"acp_agent": "unverified-fixture", "acp_model_source": "session_model", "cwd": directory.path()}),
            &json!([{"role": "user", "content": "hello"}]),
            "task9-production",
            None,
            &mut |_| {},
        ),
    )
    .await;
    match result {
        Err(super::Error::Api(message::AssistantError::Unknown { message })) => {
            assert!(message.contains("ACP session_model driver is not verified"), "{message}");
            assert!(!message.contains("provider-secret-never-distributed"));
        }
        other => panic!("unexpected production result: {other:?}"),
    }
    assert!(!launched.exists(), "an unverified driver must be rejected before launch");
}
