use super::endpoints::{EndpointCandidate, collect};
use crate::providers::endpoints;
use serde_json::json;
use vw_api_types::{
    agent_apps::AppId,
    agent_apps_diagnostics::DiagnosticEndpointSource::{self, Custom, Draft, Preset, SavedConfig},
    app_providers::endpoints::{AppProviderEndpointChange::Add, AppProviderEndpointKind},
};

fn candidate(url: &str, sources: &[DiagnosticEndpointSource]) -> EndpointCandidate {
    EndpointCandidate { url: url.into(), sources: sources.to_vec() }
}

#[test]
fn diagnostic_endpoints_merge_without_duplicates() {
    let stored = json!({
        "baseUrl": "https://www.packyapi.ai",
        "settingsConfig": {"env": {"ANTHROPIC_BASE_URL": "https://saved.example.test//"}},
    });
    let stored = [
        ("https://www.packyapi.ai/", AppProviderEndpointKind::BaseUrl),
        ("https://custom.example.test/messages/", AppProviderEndpointKind::FullUrl),
    ]
    .into_iter()
    .enumerate()
    .fold(stored, |stored, (index, (url, kind))| {
        endpoints::update(&stored, &Add { url: url.into(), kind }, index as u64).unwrap()
    });
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("provider.json");
    let before = serde_json::to_vec(&stored).unwrap();
    std::fs::write(&path, &before).unwrap();
    let profile = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();

    let draft = [
        " https://www.packyapi.ai/// ".into(),
        "http://127.0.0.1:1234/v1/".into(),
        "https://www.packyapi.ai".into(),
        " ".into(),
    ];
    let result = collect(AppId::Claude, &draft, &profile, Some("PackyCode")).unwrap();
    assert_eq!(
        result,
        [
            candidate("https://www.packyapi.ai", &[Draft, SavedConfig, Preset, Custom]),
            candidate("http://127.0.0.1:1234/v1", &[Draft]),
            candidate("https://saved.example.test", &[SavedConfig]),
            candidate("https://cf.api.fan", &[Preset]),
            candidate("https://slb-v1.api.fan", &[Preset]),
            candidate("https://www.packyapi.com", &[Preset]),
            candidate("https://custom.example.test/messages/", &[Custom]),
        ]
    );
    assert_eq!(profile, stored);
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert_eq!(draft[0], " https://www.packyapi.ai/// ");

    let gemini = json!({
        "settingsConfig": {"env": {"GOOGLE_GEMINI_BASE_URL": " https://gemini.example.test/ "}}
    });
    assert_eq!(
        collect(AppId::Gemini, &[], &gemini, Some("AICodeMirror")).unwrap(),
        [
            candidate("https://gemini.example.test", &[SavedConfig]),
            candidate("https://api.aicodemirror.ai/api/gemini", &[Preset]),
        ]
    );
    let codex = json!({
        "settingsConfig": {
            "config": "model_provider = 'active'\n[model_providers.other]\nbase_url = 'https://other.example.test'\n[model_providers.active]\nbase_url = 'https://codex.example.test/v1/'"
        }
    });
    assert_eq!(
        collect(AppId::Codex, &[], &codex, None).unwrap(),
        [candidate("https://codex.example.test/v1", &[SavedConfig])]
    );
    let legacy = json!({
        "settingsConfig": {"config": "openai_base_url = 'https://legacy.example.test/v1/'"}
    });
    assert_eq!(
        collect(AppId::Codex, &[], &legacy, None).unwrap(),
        [candidate("https://legacy.example.test/v1", &[SavedConfig])]
    );
    assert!(collect(AppId::Claude, &[], &json!({}), Some("Microsoft Foundry")).unwrap().is_empty());

    for invalid in [
        "file:///tmp/service",
        "https://user:secret@example.test/",
        "https://@example.test/",
        "https://example.test/v1?token=secret",
        "http://example.test",
    ] {
        assert!(collect(AppId::Claude, &[invalid.into()], &stored, None).is_err());
    }
}
