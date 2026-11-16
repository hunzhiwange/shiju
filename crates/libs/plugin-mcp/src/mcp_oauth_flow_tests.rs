use super::PendingAuthorization;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use sha2::{Digest, Sha256};

fn pending() -> PendingAuthorization {
    PendingAuthorization::new(
        "https://auth.example/authorize",
        "public-client",
        "http://127.0.0.1:8432/callback",
        "https://mcp.example/mcp",
    )
    .unwrap()
}
fn callback(flow: &PendingAuthorization) -> String {
    let state = flow
        .authorization_url
        .query_pairs()
        .find(|(key, _)| key == "state")
        .unwrap()
        .1
        .into_owned();
    format!("http://127.0.0.1:8432/callback?code=fixture-code&state={state}")
}
#[test]
fn codex_plugin_compat_t27_accepts_only_matching_state_and_pkce() {
    let flow = pending();
    let challenge = flow
        .authorization_url
        .query_pairs()
        .find(|(key, _)| key == "code_challenge")
        .unwrap()
        .1
        .into_owned();
    let reply = callback(&flow);
    let (code, verifier) = flow.complete(&reply).unwrap();
    assert_eq!(code, "fixture-code");
    assert_eq!(challenge, URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes())));
    assert_eq!(verifier.len(), 43);
}
#[test]
fn codex_plugin_compat_t27_rejects_invalid_input() {
    assert!(
        PendingAuthorization::new(
            "https://auth.example/authorize",
            "client",
            "http://127.0.0.1:0/callback",
            "https://mcp.example/mcp"
        )
        .is_err()
    );
    assert!(
        pending().complete("http://127.0.0.1:8432/callback?state=forged&code=fixture").is_err()
    );
    let flow = pending();
    let reply = callback(&flow) + "&state=duplicate";
    assert!(flow.complete(&reply).is_err());
    let flow = pending();
    let reply = callback(&flow).replace("8432", "8433");
    assert!(flow.complete(&reply).is_err());
}
#[test]
fn codex_plugin_compat_t27_preserves_isolation() {
    let first = pending();
    let second = pending();
    assert_ne!(first.authorization_url, second.authorization_url);
    assert!(second.complete(&callback(&first)).is_err());
}

