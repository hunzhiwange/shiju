use vw_api_tools::{ToolExposure, ToolSource, ToolSpecDto};

#[test]
fn dynamic_tools_contract_mcp_registration_preserves_identity_and_stable_id() {
    use crate::mcp_tool::mcp_tools_to_specs;
    let definition = rmcp::model::Tool::new("read", "Read data", serde_json::Map::new());
    let alpha = mcp_tools_to_specs("alpha", None, &[definition.clone()], &[]).unwrap();
    let beta = mcp_tools_to_specs("beta", None, &[definition], &[]).unwrap();
    assert_ne!(alpha[0].id, beta[0].id);
    assert_ne!(alpha[0].identity, beta[0].identity);
    assert_eq!(alpha[0].identity.namespace.as_deref(), Some("alpha"));
    assert_eq!(alpha[0].identity.name, "read");
    assert_eq!(alpha[0].source, ToolSource::Mcp);
    assert_eq!(alpha[0].exposure, ToolExposure::Deferred);
    assert_eq!(alpha[0].clone().with_display_name("New title").id, alpha[0].id);
    let dto = alpha[0].to_dto();
    assert_eq!(
        serde_json::from_value::<ToolSpecDto>(serde_json::to_value(&dto).unwrap()).unwrap(),
        dto
    );
    assert!(
        mcp_tools_to_specs(
            "alpha",
            None,
            &[rmcp::model::Tool::new("read", "Read", serde_json::Map::new())],
            &[alpha[0].id.as_str()]
        )
        .is_err()
    );
}
