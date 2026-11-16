use vw_api_app_diagnostics::*;
use vw_api_app_identity::AppId;
use vw_api_app_provider_base::basic::ProviderBasic;
#[test]
fn diagnostic_desktop_endpoint_selection_requires_save_other_forms() {
    use crate::agent_apps_diagnostics::endpoint_state;
    use desktop_provider_apps::state::{
        codex,
        edit::{EditForm, Mode},
        gemini,
    };
    let basic =
        ProviderBasic { name: "provider".into(), category: None, notes: None, website_url: None };
    let mut form = EditForm::new(Mode::Update, "provider".into(), basic, None);
    form.codex = Some(codex::Draft::new());
    form.codex.as_mut().unwrap().toml = iced::widget::text_editor::Content::with_text(
        "# keep this comment\nmodel_provider='vendor'\nmodel='kept'\n[model_providers.vendor]\nbase_url='https://draft.example' # keep URL comment\nwire_api='responses'\n[model_providers.other]\nbase_url='https://other.example'\n",
    );
    let query = endpoint_state::query(&form, "PackyCode").unwrap();
    assert_eq!(query.draft_urls, ["https://draft.example"]);
    assert_eq!(query.preset_name.as_deref(), Some("PackyCode"));
    // Selection applies through the same pending editor state used by the UI, no SDK writes.
    let scope = desktop_provider_apps::Scope {
        context: desktop_provider_apps::AgentAppsContext {
            gateway: "http://localhost:1".into(),
            tenant: Default::default(),
            project: None,
        },
        host: "host".into(),
        app: AppId::Codex,
    };
    let client = vw_gateway_client::GatewayClient::new(gateway_endpoint::GatewayEndpoint::new(
        "127.0.0.1",
        1,
    ))
    .unwrap();
    let id = DiagnosticEndpointId("selected".into());
    let mut state = endpoint_state::State {
        context: crate::diagnostics::model_state::ModelContext {
            scope,
            project: "project".into(),
            credentials_generation: 0,
            provider: "provider".into(),
        },
        generation: 0,
        transport: endpoint_state::Transport { client, token: "fixture".into() },
        candidates: vec![DiagnosticEndpointCandidate {
            id: id.clone(),
            display_url: "https://selected.example".into(),
            sources: vec![DiagnosticEndpointSource::Draft],
        }],
        selected: None,
        preset_name: String::new(),
        loading: false,
        starting: false,
        cancelling: false,
        closing: false,
        serial: 0,
        job: None,
        snapshot: None,
        error: None,
    };
    let before = form.codex.as_ref().unwrap().toml.text();
    state.choose(id);
    assert_eq!(form.codex.as_ref().unwrap().toml.text(), before);
    state.apply(&mut form).unwrap();
    assert!(form.codex.as_ref().unwrap().toml.text().contains("# keep this comment"));
    assert!(form.codex.as_ref().unwrap().toml.text().contains("# keep URL comment"));
    let doc: toml::Value = toml::from_str(&form.codex.as_ref().unwrap().toml.text()).unwrap();
    assert_eq!(
        doc["model_providers"]["vendor"]["base_url"].as_str(),
        Some("https://selected.example")
    );
    assert_eq!(doc["model_providers"]["other"]["base_url"].as_str(), Some("https://other.example"));
    assert_eq!(doc["model"].as_str(), Some("kept"));
    form.codex.as_mut().unwrap().toml =
        iced::widget::text_editor::Content::with_text("model_provider='missing'");
    assert!(
        state.apply(&mut form).is_err(),
        "missing selected table cannot silently write an ignored root URL"
    );
    form.codex = None;
    form.gemini = Some(gemini::Draft::from_view(
        vw_api_app_provider_profiles::gemini::GeminiFormKind::ThirdParty {
            base_url: "https://gemini.example".into(),
        },
        None,
    ));
    assert_eq!(endpoint_state::query(&form, "").unwrap().draft_urls, ["https://gemini.example"]);
    state.apply(&mut form).unwrap();
    assert_eq!(form.gemini.as_ref().unwrap().base_url, "https://selected.example");
}
