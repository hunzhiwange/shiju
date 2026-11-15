use super::*;
use vw_api_tools::{ToolExposure, ToolIdentity, ToolSource};

#[test]
fn dynamic_tools_contract_namespace_identity_roundtrips() {
    let mut spec = ToolSpec::new("stable-id", "description", json!({"type":"object"}));
    spec.identity = ToolIdentity::new(Some("alpha".into()), "read").unwrap();
    spec.source = ToolSource::Client;
    spec.exposure = ToolExposure::DeferredModelOnly;
    let dto = spec.clone().with_display_name("renamed").to_dto();
    assert_eq!(dto.id.0.as_str(), "stable-id");
    assert_eq!(dto.identity, spec.identity);
    assert_eq!(dto.source, spec.source);
    assert_eq!(dto.exposure, spec.exposure);
    assert_eq!(dto.input_schema, spec.input_schema);
    assert_eq!(
        serde_json::from_value::<ToolSpecDto>(serde_json::to_value(&dto).unwrap()).unwrap(),
        dto
    );
}
