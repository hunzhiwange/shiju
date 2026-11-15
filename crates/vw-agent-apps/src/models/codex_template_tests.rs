use super::codex_template::codex_custom_template;
use crate::providers::codex_catalog::form;
use toml_edit::{DocumentMut, value};
use vw_api_app_provider_base::basic::ProviderBasic;
use vw_api_types::app_providers::codex_catalog::CodexFormWrite;
use vw_api_app_provider_base::create::ProviderSecretWrite;

#[test]
fn catalog_codex_template_requires_endpoint_and_key_to_prepare_provider() {
    let mut draft: DocumentMut = codex_custom_template().config.parse().unwrap();
    let mut request = CodexFormWrite {
        target_host_id: "local".into(),
        fields: ProviderBasic {
            name: "Example".into(),
            category: None,
            notes: None,
            website_url: None,
        },
        toml: draft.to_string(),
        api_key: ProviderSecretWrite::Replace { value: "fixture-key".into() },
        models: Some(Vec::new()),
        apply: false,
    };
    assert!(form::prepare("example", &request, None).is_err());

    let provider = draft
        .get_mut("model_providers")
        .and_then(|providers| providers.get_mut("custom"))
        .expect("template must contain the custom provider");
    provider["base_url"] = value("https://example.com/v1");
    request.toml = draft.to_string();
    request.api_key = ProviderSecretWrite::Keep;
    assert!(form::prepare("example", &request, None).is_err());

    request.api_key = ProviderSecretWrite::Replace { value: "fixture-key".into() };
    let profile = form::prepare("example", &request, None).unwrap();
    assert_eq!(profile.api_key.as_deref(), Some("fixture-key"));
    let config: DocumentMut = profile.config.parse().unwrap();
    assert_eq!(config["model"].as_str(), Some("gpt-5.6-sol"));
    assert_eq!(config["model_providers"]["custom"]["wire_api"].as_str(), Some("responses"));
}
