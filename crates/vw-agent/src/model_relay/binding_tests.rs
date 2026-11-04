use super::*;
use std::time::Duration;
fn identity() -> RelayIdentity {
    RelayIdentity {
        tenant: vw_api_types::id::TenantScope::default(),
        project: "project".into(),
        session: "session".into(),
        provider: "provider".into(),
        model: "model".into(),
        generation: 1,
    }
}
#[test]
fn acp_model_source_relay_binding_rejects_cross_scope_and_revocation() {
    let id = identity();
    let binding = RelayBinding::new(id.clone(), Duration::from_secs(60)).unwrap();
    assert!(binding.accepts(binding.bearer_token(), &id));
    assert!(!binding.accepts("wrong", &id));
    for field in 0..6 {
        let mut other = id.clone();
        match field {
            0 => other.tenant = vw_api_types::id::TenantScope::new("other", "tenant").unwrap(),
            1 => other.project = "other".into(),
            2 => other.session = "other".into(),
            3 => other.provider = "other".into(),
            4 => other.model = "other".into(),
            _ => other.generation += 1,
        }
        assert!(!binding.accepts(binding.bearer_token(), &other));
    }
    binding.revoke();
    assert!(!binding.accepts(binding.bearer_token(), &id));
}
#[tokio::test]
async fn acp_model_source_relay_binding_expires() {
    let id = identity();
    let binding = RelayBinding::new(id.clone(), Duration::from_millis(1)).unwrap();
    tokio::time::sleep(Duration::from_millis(5)).await;
    assert!(!binding.accepts(binding.bearer_token(), &id));
}
