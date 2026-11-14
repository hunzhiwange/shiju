use super::RelayIdentity;
use super::native_responses::{check, remember};
use crate::app_routing::reference_scope::{ReferenceScope, References};
use axum::http::StatusCode;
use serde_json::json;

#[test]
fn cc_route_036_native_responses_expected_behavior() {
    let identity = RelayIdentity {
        tenant: vw_api_types::id::TenantScope::new("platform", "company").unwrap(),
        generation: 1,
        project: "p".into(),
        session: "s".into(),
        provider: "provider".into(),
        model: "m".into(),
    };
    let scope = ReferenceScope::Session(identity.clone());
    let references = References::default();
    let request =
        json!({"previous_response_id":"resp_1","input":[{"type":"item_reference","id":"item_1"}]});
    assert_eq!(check(&request, &references, &scope), Err(StatusCode::FORBIDDEN));
    remember(&json!({"id":"resp_1","output":[{"id":"item_1"}]}), &references, &scope).unwrap();
    assert_eq!(check(&request, &references, &scope), Ok(()));
    let other = ReferenceScope::Session(RelayIdentity { session: "other".into(), ..identity });
    assert_eq!(check(&request, &references, &other), Err(StatusCode::FORBIDDEN));
    assert_eq!(
        check(&json!({"input":[{"type":"item_reference","id":"foreign"}]}), &references, &scope),
        Err(StatusCode::FORBIDDEN)
    );
}

#[tokio::test]
async fn cc_route_036_stream_records_references_without_changing_bytes() {
    let identity = RelayIdentity {
        tenant: vw_api_types::id::TenantScope::new("platform", "company").unwrap(),
        generation: 1,
        project: "p".into(),
        session: "s".into(),
        provider: "provider".into(),
        model: "m".into(),
    };
    let scope = ReferenceScope::Session(identity);
    let references = std::sync::Arc::new(References::default());
    let wire = format!(
        "data: {}\n\n",
        json!({"type":"response.completed","response":{"id":"stream_response","output":[{"id":"stream_item"}]}})
    );
    let chunks = wire
        .as_bytes()
        .chunks(3)
        .map(|chunk| Ok::<_, std::io::Error>(axum::body::Bytes::copy_from_slice(chunk)))
        .collect::<Vec<_>>();
    let body = super::native_responses::stream(
        axum::body::Body::from_stream(futures_util::stream::iter(chunks)),
        references.clone(),
        scope.clone(),
    );
    let bytes = axum::body::to_bytes(body, 4096).await.unwrap();
    assert_eq!(&bytes[..], wire.as_bytes());
    assert_eq!(
        check(
            &json!({"previous_response_id":"stream_response","input":[{"type":"item_reference","id":"stream_item"}]}),
            &references,
            &scope
        ),
        Ok(())
    );
}
