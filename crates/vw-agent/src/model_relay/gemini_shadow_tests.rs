use super::gemini_shadow::*;
use crate::{app_routing::reference_scope::ReferenceScope, model_relay::RelayIdentity};
use serde_json::json;

fn context() -> Context {
    Context { store: Default::default(), scope: ReferenceScope::Session(RelayIdentity {
        tenant: Default::default(), project: "project".into(), session: "session".into(),
        provider: "provider".into(), model: "model".into(), generation: 1,
    }), session: "conversation".into(), cancelled: Default::default() }
}

#[test]
fn cc_route_050_gemini_shadow_expected_behavior() {
    let ctx = context();
    let mut response = json!({"candidates":[{"content":{"parts":[{"functionCall":{"name":"search","args":{}},"thoughtSignature":"opaque"}]}}]});
    ctx.capture(&mut response).unwrap();
    let part = response["candidates"][0]["content"]["parts"][0].clone();
    assert!(part.get("thoughtSignature").is_none());
    let mut request = json!({"contents":[{"role":"model","parts":[part]}]});
    ctx.replay(&mut request).unwrap();
    assert_eq!(request["contents"][0]["parts"][0]["thoughtSignature"], "opaque");
    assert!(request["contents"][0]["parts"][0]["functionCall"].get("id").is_none());
}

#[test]
fn cc_route_050_rejects_other_sessions_mutations_and_revocation() {
    let ctx = context();
    let mut response = json!({"candidates":[{"content":{"parts":[{"functionCall":{"name":"search","args":{}},"thoughtSignature":"opaque"}]}}]});
    ctx.capture(&mut response).unwrap();
    let request = json!({"contents":[{"role":"model","parts":response["candidates"][0]["content"]["parts"]}]});
    let mut other = ctx.clone();
    other.session = "other".into();
    assert!(other.replay(&mut request.clone()).is_err());
    if let ReferenceScope::Session(identity) = &mut other.scope { identity.generation += 1; }
    assert!(other.replay(&mut request.clone()).is_err());
    let mut changed = request.clone();
    changed["contents"][0]["parts"][0]["functionCall"]["name"] = "other".into();
    assert!(ctx.replay(&mut changed).is_err());
    ctx.cancelled.cancel();
    assert!(ctx.replay(&mut request.clone()).is_err());
}
