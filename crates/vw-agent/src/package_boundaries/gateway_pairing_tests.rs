use crate::security::{self, pairing};
use vw_config_types::gateway::GatewaySkey;

#[tokio::test]
async fn gateway_pairing_production_boundary() {
    let guard = pairing::PairingGuard::new(true, &[]);
    let package_guard: &gateway_pairing::pairing::PairingGuard = &guard;
    let root_guard: &security::PairingGuard = package_guard;
    assert!(std::ptr::eq(root_guard, &guard));
    let shared_guard = package_guard.clone();
    let code = root_guard.pairing_code().expect("pairing code");
    let token = root_guard
        .try_pair(&code, "boundary-fixture-client")
        .await
        .expect("client is not locked out")
        .expect("paired token");
    assert!(package_guard.is_authenticated(&token));
    assert!(shared_guard.is_authenticated(&token));
    assert!(shared_guard.pairing_code().is_none());
    assert!(!root_guard.is_authenticated("invalid-boundary-token"));

    let skey = "boundary-fixture-skey";
    root_guard.update_from_skeys(true, &[GatewaySkey {
        enabled: true,
        skey: None,
        skey_hash: pairing::hash_skey(skey),
        masked_skey: pairing::masked_skey_name(skey),
        name: "boundary fixture".to_string(),
        expires_at: None,
    }]);
    assert!(shared_guard.is_authenticated(skey));
    assert!(!shared_guard.is_authenticated(&token));
    shared_guard.update_from_skeys(false, &[]);
    assert!(!root_guard.auth_enabled());
    assert!(root_guard.is_authenticated(""));
}
