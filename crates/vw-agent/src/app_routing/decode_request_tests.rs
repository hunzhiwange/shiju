use super::{JsonTarget, VerifiedRouteBinding};
use crate::app_routing::{
    RoutingListener, SharedTransport,
    decode_request::{ContentCodings, DecodeError},
};
use async_compression::tokio::bufread::{BrotliEncoder, DeflateEncoder, ZlibEncoder, ZstdEncoder};
use axum::{
    Router,
    body::Bytes,
    http::{HeaderMap, StatusCode, header},
    routing::post,
};
use flate2::{Compression, write::GzEncoder};
use serde_json::{Value, json};
use std::{io::Write, time::Duration};
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    net::TcpListener,
    sync::mpsc,
    time::timeout,
};
use vw_api_types::{
    agent_apps::AppId,
    id::{ProjectScope, TenantScope},
};

const BODY: &[u8] = br#" { "model": "fixture", "_private": true } "#;

fn gzip(body: &[u8]) -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(body).unwrap();
    encoder.finish().unwrap()
}

async fn compress(mut encoder: impl AsyncRead + Unpin) -> Vec<u8> {
    let mut output = Vec::new();
    encoder.read_to_end(&mut output).await.unwrap();
    output
}

#[tokio::test]
async fn cc_route_018_decode_request_expected_behavior() {
    timeout(Duration::from_secs(10), async {
        let upstream = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let target = format!("http://{}/fixture", upstream.local_addr().unwrap()).parse().unwrap();
        let (requests, mut received) = mpsc::unbounded_channel();
        let upstream_app = Router::new().route(
            "/fixture",
            post(move |headers: HeaderMap, body: Bytes| async move {
                requests.send((headers, body)).unwrap();
                ([(header::CONTENT_TYPE, "application/json")], "{}")
            }),
        );
        let upstream_task = tokio::spawn(async move {
            axum::serve(upstream, upstream_app).await.unwrap();
        });

        let listener = RoutingListener::default();
        let address = listener.start(0).unwrap();
        let grant = listener
            .auth
            .issue_with_target(
                VerifiedRouteBinding {
                    project: ProjectScope::new(
                        TenantScope::new("platform", "company").unwrap(),
                        "project".into(),
                    )
                    .unwrap(),
                    host: "host".into(),
                    app: AppId::Codex,
                    account: "account".into(),
                    account_generation: 1,
                },
                Some(JsonTarget {
                    url: target,
                    authorization: "Bearer provider-fixture".parse().unwrap(),
                    transport: SharedTransport::new().unwrap(),
                    request_options: Default::default(),
                }),
            )
            .unwrap();
        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        let url = format!("http://{address}/v1/chat/completions");
        let zlib = compress(ZlibEncoder::new(BODY)).await;
        let raw = compress(DeflateEncoder::new(BODY)).await;
        let br = compress(BrotliEncoder::new(BODY)).await;
        let zstd = compress(ZstdEncoder::new(BODY)).await;
        let repeated_gzip = gzip(&gzip(BODY));
        let stacked = compress(ZstdEncoder::new(repeated_gzip.as_slice())).await;
        let mut concatenated_gzip = gzip(&BODY[..15]);
        concatenated_gzip.extend(gzip(&BODY[15..]));
        for (encodings, encoded) in [
            (vec![], BODY.to_vec()),
            (vec!["identity"], BODY.to_vec()),
            (vec!["gzip"], gzip(BODY)),
            (vec!["x-gzip"], gzip(BODY)),
            (vec!["deflate"], zlib.clone()),
            (vec!["deflate"], raw.clone()),
            (vec!["br"], br.clone()),
            (vec!["zstd"], zstd.clone()),
            (vec!["zst"], zstd.clone()),
            (vec!["gzip"], concatenated_gzip),
            (vec!["gzip, X-GZip", "identity, ZsT"], stacked.clone()),
        ] {
            let mut request = client
                .post(&url)
                .bearer_auth(&grant.token)
                .header(header::CONTENT_TYPE, "application/json")
                .header("content-digest", "stale");
            for coding in &encodings {
                request = request.header(header::CONTENT_ENCODING, *coding);
            }
            let response = request.body(encoded).send().await.unwrap();
            assert_eq!(
                response.status(),
                StatusCode::OK,
                "{encodings:?} JSON should reach the upstream"
            );
            let (headers, body) = received.recv().await.unwrap();
            assert!(!headers.contains_key(header::CONTENT_ENCODING));
            assert!(!headers.contains_key("content-digest"));
            assert_eq!(headers[header::CONTENT_LENGTH], body.len().to_string());
            assert_eq!(
                serde_json::from_slice::<Value>(&body).unwrap(),
                json!({"model": "fixture"})
            );
        }

        let mut rejected = vec![
            ("unsupported", BODY.to_vec(), StatusCode::UNSUPPORTED_MEDIA_TYPE),
            ("gzip, unknown, zst", stacked, StatusCode::UNSUPPORTED_MEDIA_TYPE),
            ("gzip", gzip(&vec![b' '; 4 * 1024 * 1024 + 1]), StatusCode::PAYLOAD_TOO_LARGE),
        ];
        for (coding, mut encoded) in
            [("gzip", gzip(BODY)), ("deflate", zlib), ("deflate", raw), ("br", br), ("zstd", zstd)]
        {
            let mut trailing = encoded.clone();
            trailing.extend(b"trailing junk");
            rejected.push((coding, trailing, StatusCode::BAD_REQUEST));
            encoded.pop();
            rejected.push((coding, encoded, StatusCode::BAD_REQUEST));
        }
        for (coding, encoded, status) in rejected {
            let response = client
                .post(&url)
                .bearer_auth(&grant.token)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::CONTENT_ENCODING, coding)
                .body(encoded)
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), status, "invalid {coding} request");
        }
        let response = client
            .post(&url)
            .bearer_auth(&grant.token)
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::CONNECTION, "content-encoding")
            .header(header::CONTENT_ENCODING, "unknown")
            .body(BODY)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
        assert!(received.try_recv().is_err(), "rejected request reached the upstream");
        listener.stop().await.unwrap();
        upstream_task.abort();
        assert!(upstream_task.await.unwrap_err().is_cancelled());
    })
    .await
    .expect("routing compression fixture timed out");
}

#[tokio::test]
async fn cc_route_018_decode_request_bounds_every_stage() {
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_ENCODING, "gzip".parse().unwrap());
    let encoded = gzip(BODY);
    let codings = ContentCodings::from_headers(&headers).unwrap();
    assert_eq!(codings.decode(encoded.clone().into(), BODY.len()).await.unwrap(), BODY);

    // The final JSON fits but the intermediate gzip stream exceeds the budget.
    assert!(encoded.len() > BODY.len());
    headers.append(header::CONTENT_ENCODING, "zstd".parse().unwrap());
    let stacked = compress(ZstdEncoder::new(encoded.as_slice())).await;
    let codings = ContentCodings::from_headers(&headers).unwrap();
    assert_eq!(codings.decode(stacked.into(), BODY.len()).await, Err(DecodeError::TooLarge),);
}
