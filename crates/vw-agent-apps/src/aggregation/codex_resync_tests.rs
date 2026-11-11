use crate::{
    aggregation::{codex_catalog::*, codex_publish},
    providers::{codex, codex_catalog},
};
use vw_api_types::app_providers::codex_catalog::CodexProfile;
#[test]
fn aggregation_t16_codex_plan_composes_with_native_projection() {
    let profile=CodexProfile{name:"alpha".into(),config:"model='model-a'\nmodel_provider='vendor'\n[model_providers.vendor]\nname='Fixture'\nbase_url='http://127.0.0.1:7/v1'\nwire_api='responses'\n".into(),api_key:Some("fixture-key".into()),preserve_login:false};
    assert!(codex_catalog::identity("host", "alpha", &profile).is_ok());
    let cfg = profile.config.parse().unwrap();
    let before=b"# user comment\nmodel='model-a'\nmodel_provider='custom'\n[model_providers.custom]\nname='Old local'\nbase_url='http://127.0.0.1:7/v1'\nwire_api='responses'\n[projects.fixture]\ntrust_level='trusted'\n";
    let (config, _) = codex_publish::plan(
        Some(before),
        codex_publish::CatalogPolicy::Preserve,
        CodexCatalogDefault::ThirdParty(CodexCatalogRow { config: &cfg, catalog: None }),
        &[],
    )
    .unwrap();
    let mut doc: toml_edit::DocumentMut = std::str::from_utf8(&config).unwrap().parse().unwrap();
    let provider = doc["model_provider"].as_str().unwrap().to_owned();
    doc["model_providers"][&provider]["base_url"] = toml_edit::value("http://127.0.0.1:12345/v1");
    doc["model_providers"][&provider]["wire_api"] = toml_edit::value("responses");
    let settings = serde_json::json!({"config":doc.to_string(),"auth":{"OPENAI_API_KEY":"fixture-new-local-key"}});
    let projected = codex::project(std::str::from_utf8(&config).unwrap(), &settings, None, false);
    assert!(projected.is_ok(), "{projected:?}");
}
