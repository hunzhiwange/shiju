use super::{AppId, Provider, project};
use serde_json::json;

#[test]
fn app_presets_use_evidenced_endpoints_and_supported_authentication() {
    let evidence = json!([{
        "kind": "official", "source": "fixture", "reference": "fixture", "checked_at": "2026-01-01"
    }]);
    let facts = json!([
        {
            "protocol":"messages", "url":"https://fixture.test/anthropic/v1/messages",
            "deployment":"messages", "authentication":{"kind":"api_key_header","name":"x-api-key"},
            "models":{"kind":"all"}, "evidence":evidence
        },
        {
            "protocol":"responses", "url":"https://fixture.test/openai/v1/responses",
            "deployment":"responses", "authentication":{"kind":"bearer"},
            "models":{"kind":"models","ids":["fixture-model"]}, "evidence":evidence
        },
        {
            "protocol":"responses", "url":"https://YOUR_ACCOUNT_ID.fixture.test/v1/responses",
            "deployment":"template", "authentication":{"kind":"bearer"},
            "models":{"kind":"all"}, "evidence":evidence
        }
    ]);
    let provider: Provider = serde_json::from_value(json!({
        "id":"fixture", "name":"Fixture", "api":"https://incorrect.test",
        "protocols":facts,
        "models":{
            "fixture-model":{"id":"fixture-model","limit":{"context":32000}},
            "blocked-model":{"id":"blocked-model","protocols":[]}
        }
    }))
    .unwrap();

    let claude = project(AppId::Claude, [&provider]);
    assert_eq!(claude.len(), 1);
    assert_eq!(claude[0].base_url, "https://fixture.test/anthropic");
    assert!(claude[0].api_key_header);
    assert!(project(AppId::ClaudeDesktop, [&provider]).is_empty());

    let codex = project(AppId::Codex, [&provider]);
    assert_eq!(codex.len(), 1);
    assert_eq!(codex[0].base_url, "https://fixture.test/openai/v1");
    assert_eq!(codex[0].models.len(), 1);
    assert_eq!(codex[0].models[0].id, "fixture-model");
    assert_eq!(codex[0].models[0].context_window, Some(32000));
}
