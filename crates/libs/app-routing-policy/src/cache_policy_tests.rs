use super::cache_policy::{inject_key, inject_messages};
use serde_json::json;

#[test]
fn cc_route_070_cache_policy_expected_behavior() {
    let mut body = json!({"tools":[{"name":"read"}],"system":"system",
        "messages":[{"role":"user","content":[{"type":"text","text":"hello"}]}]});
    let original = body.clone();
    inject_messages(&mut body, false);
    assert_eq!(body, original);
    inject_messages(&mut body, true);
    assert_eq!(body["tools"][0]["cache_control"], json!({"type":"ephemeral"}));
    assert_eq!(body["system"][0]["cache_control"], json!({"type":"ephemeral"}));
    let once = body.clone();
    inject_messages(&mut body, true);
    assert_eq!(body, once);
}

#[test]
fn cc_route_070_preserves_caller_breakpoints_and_thinking() {
    let mut body = json!({"tools": (0..5).map(|i| json!({"name":i,"cache_control":{"type":"ephemeral","ttl":"1h"}})).collect::<Vec<_>>(),
        "messages":[{"role":"assistant","content":[{"type":"thinking","thinking":"reason"},{"type":"text","text":"answer"}]}]});
    let original = body.clone();
    inject_messages(&mut body, true);
    assert_eq!(body, original);
    body["tools"] = json!([]);
    inject_messages(&mut body, true);
    assert!(body["messages"][0]["content"][0].get("cache_control").is_none());
    assert!(body["messages"][0]["content"][1].get("cache_control").is_some());
}

#[test]
fn cc_route_070_keys_are_scoped_bounded_and_caller_owned() {
    let mut a = json!({});
    inject_key(&mut a, false, b"account-a", Some("session"));
    assert_eq!(a, json!({}));
    inject_key(&mut a, true, b"account-a", None);
    assert_eq!(a, json!({}));
    inject_key(&mut a, true, b"account-a", Some("session"));
    let mut b = json!({});
    inject_key(&mut b, true, b"account-b", Some("session"));
    assert_ne!(a, b);
    assert_eq!(a["prompt_cache_key"].as_str().unwrap().len(), 64);
    let saved = a.clone();
    inject_key(&mut a, true, b"account-a", Some("session"));
    assert_eq!(a, saved);
    let mut explicit = json!({"prompt_cache_key":"caller-key"});
    inject_key(&mut explicit, true, b"account-a", Some("session"));
    assert_eq!(explicit["prompt_cache_key"], "caller-key");
}
