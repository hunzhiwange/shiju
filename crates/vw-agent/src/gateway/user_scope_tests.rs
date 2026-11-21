use super::*;

#[test]
fn bound_skey_only_allows_its_user_and_requires_an_active_credential() {
    let token = "test-session-credential";
    let mut key = GatewaySkey {
        enabled: true,
        skey: None,
        skey_hash: hash_skey(token),
        masked_skey: String::new(),
        name: String::new(),
        expires_at: None,
        user_id: Some("42".to_owned()),
    };

    assert!(allows_user(vec![key.clone()], token, "42"));
    assert!(!allows_user(vec![key.clone()], token, "43"));
    assert!(!allows_user(vec![key.clone()], token, DEFAULT_USER_ID));
    assert!(!allows_user(vec![key.clone()], "other-test-credential", "42"));
    key.enabled = false;
    assert!(!allows_user(vec![key], token, "42"));
    assert!(allows_user(Vec::new(), token, DEFAULT_USER_ID));
}
