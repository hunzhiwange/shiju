use super::{JsonTarget, VerifiedRouteBinding};
use crate::app_routing::{RoutingListener, SharedTransport};
use async_compression::tokio::bufread::{BrotliEncoder, DeflateEncoder, ZlibEncoder, ZstdEncoder};
use axum::{
    Router,
    body::{Body, Bytes},
    http::{HeaderMap, StatusCode, header},
    response::Response,
    routing::post,
};
use flate2::{Compression, write::GzEncoder};
use std::{collections::VecDeque, io::Write, sync::Arc, time::Duration};
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    net::TcpListener,
    time::timeout,
};
use vw_api_types::{
    agent_apps::AppId,
    id::{ProjectScope, TenantScope},
};

const JSON: &[u8] = br#"{"id":"upstream-response"}"#;
const BODY_LIMIT: usize = 4 * 1024 * 1024;

fn gzip(body: &[u8]) -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
    encoder.write_all(body).unwrap();
    encoder.finish().unwrap()
}

async fn compress(mut encoder: impl AsyncRead + Unpin) -> Vec<u8> {
    let mut output = Vec::new();
    encoder.read_to_end(&mut output).await.unwrap();
    output
}

fn upstream_response(codings: &[&str], body: Vec<u8>) -> Response {
    let mut response = Response::builder()
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::CONTENT_LENGTH, body.len())
        .header("content-digest", "stale")
        .header("repr-digest", "stale")
        .header("etag", "\"stale\"")
        .body(Body::from(body))
        .unwrap();
    for coding in codings {
        response.headers_mut().append(header::CONTENT_ENCODING, coding.parse().unwrap());
    }
    response
}

#[tokio::test]
async fn cc_route_019_decode_response_expected_behavior() {
    timeout(Duration::from_secs(20), async {
        let gzipped = gzip(JSON);
        let stacked = compress(ZstdEncoder::new(gzipped.as_slice())).await;
        let success = [
            (vec!["x-gzip"], gzipped.clone()),
            (vec!["gzip"], gzipped.clone()),
            (vec!["deflate"], compress(ZlibEncoder::new(JSON)).await),
            (vec!["deflate"], compress(DeflateEncoder::new(JSON)).await),
            (vec!["br"], compress(BrotliEncoder::new(JSON)).await),
            (vec!["zstd"], compress(ZstdEncoder::new(JSON)).await),
            (vec!["zst"], compress(ZstdEncoder::new(JSON)).await),
            (vec!["GZip", "identity, ZsT"], stacked),
            (vec!["identity, Identity"], JSON.to_vec()),
            (vec![], JSON.to_vec()),
        ];

        let mut corrupt = gzipped;
        corrupt.pop();
        let bomb = gzip(&vec![b' '; BODY_LIMIT + 1]);
        let failures = [
            ("gzip", corrupt, StatusCode::OK),
            ("gzip", bomb, StatusCode::OK),
            ("unknown", JSON.to_vec(), StatusCode::OK),
            ("gzip", gzip(b"private error"), StatusCode::TOO_MANY_REQUESTS),
        ];
        let cases = Arc::new(parking_lot::Mutex::new(VecDeque::new()));

        let upstream = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let target = format!("http://{}/fixture", upstream.local_addr().unwrap()).parse().unwrap();
        let upstream_app = Router::new().route(
            "/fixture",
            post({
                let cases = cases.clone();
                move |headers: HeaderMap, body: Bytes| {
                    let cases = cases.clone();
                    async move {
                        assert_eq!(headers[header::AUTHORIZATION], "Bearer provider-fixture");
                        assert_eq!(body, br#"{"model":"fixture"}"#.as_slice());
                        cases.lock().pop_front().unwrap()
                    }
                }
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
                    #[cfg(target_os = "macos")]
                    managed: None,
                    url: target,
                    authorization: "Bearer provider-fixture".parse().unwrap(),
                    transport: SharedTransport::new().unwrap(),
                    request_options: Default::default(),
                }),
            )
            .unwrap();
        let client = reqwest::Client::builder()
            .no_proxy()
            .no_gzip()
            .no_brotli()
            .no_deflate()
            .no_zstd()
            .build()
            .unwrap();
        let url = format!("http://{address}/v1/chat/completions");
        for ordered in [true, false] {
            cases.lock().extend(
                success
                    .iter()
                    .map(|(codings, body)| upstream_response(codings, body.clone()))
                    .chain(failures.iter().map(|(coding, body, status)| {
                        let mut response = upstream_response(&[*coding], body.clone());
                        *response.status_mut() = *status;
                        response
                    })),
            );
            for index in 0..success.len() + failures.len() {
                let mut request = client
                    .post(&url)
                    .bearer_auth(&grant.token)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(br#"{"model":"fixture"}"#.as_slice());
                if ordered {
                    request = request.header(header::USER_AGENT, "fixture");
                }
                let response = request.send().await.unwrap();
                let context = format!("ordered={ordered}, response case {index}");
                if index < success.len() {
                    assert_eq!(response.status(), StatusCode::OK, "{context}");
                    assert!(!response.headers().contains_key(header::CONTENT_ENCODING));
                    assert!(!response.headers().contains_key("content-digest"));
                    assert!(!response.headers().contains_key("repr-digest"));
                    assert!(!response.headers().contains_key(header::ETAG));
                    if let Some(length) = response.headers().get(header::CONTENT_LENGTH) {
                        assert_eq!(length.to_str().unwrap().parse::<usize>().unwrap(), JSON.len());
                    }
                    assert_eq!(response.bytes().await.unwrap(), JSON, "{context}");
                } else {
                    assert_eq!(response.status(), StatusCode::BAD_GATEWAY, "{context}");
                    assert_eq!(response.headers()[header::CONTENT_TYPE], "application/json");
                    assert!(!response.headers().contains_key(header::CONTENT_ENCODING));
                    let body = response.text().await.unwrap();
                    assert!(!body.contains("private error"), "{context}");
                    let body: serde_json::Value = serde_json::from_str(&body).unwrap();
                    assert_eq!(body["error"]["code"], "upstream_error", "{context}");
                }
            }
            assert!(cases.lock().is_empty());
        }

        listener.stop().await.unwrap();
        upstream_task.abort();
        assert!(upstream_task.await.unwrap_err().is_cancelled());
    })
    .await
    .expect("routing compression fixture timed out");
}

#[tokio::test]
async fn cc_route_019_decode_response_distinguishes_corruption_from_each_budget() {
    use crate::app_routing::{
        decode_request::{ContentCodings, DecodeError},
        decode_response::decode,
    };
    let limit = 256;
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_ENCODING, "gzip".parse().unwrap());
    let gzip_coding = || ContentCodings::from_headers(&headers).unwrap();

    let exact = vec![b'x'; limit];
    let encoded = gzip(&exact);
    assert!(encoded.len() < limit);
    let body = decode(Body::from(encoded.clone()), gzip_coding(), limit).await.unwrap();
    assert_eq!(body, exact);

    let truncated = |mut encoded: Vec<u8>| {
        encoded.truncate(encoded.len() - 8); // Remove the gzip trailer.
        Body::from(encoded)
    };
    assert_eq!(
        decode(truncated(encoded), gzip_coding(), limit).await.unwrap_err(),
        DecodeError::Corrupt,
    );

    // A decoder that expands to EOF before checking the output budget would
    // report the missing trailer, even though it already crossed the limit.
    let bomb = gzip(&vec![b'x'; limit * 64]);
    assert!(bomb.len() < limit);
    assert_eq!(
        decode(truncated(bomb), gzip_coding(), limit).await.unwrap_err(),
        DecodeError::TooLarge,
    );

    let chunks =
        [Ok::<_, std::io::Error>(Bytes::from(vec![0; limit])), Ok(Bytes::from_static(b"x"))];
    let wire = Body::from_stream(futures_util::stream::iter(chunks));
    assert_eq!(decode(wire, gzip_coding(), limit).await.unwrap_err(), DecodeError::TooLarge);

    let mut intermediate = gzip(&[]).repeat(40);
    intermediate.extend(gzip(JSON));
    assert!(intermediate.len() > limit);
    let stacked = compress(ZstdEncoder::new(intermediate.as_slice())).await;
    assert!(stacked.len() < limit);
    headers.append(header::CONTENT_ENCODING, "zstd".parse().unwrap());
    let codings = ContentCodings::from_headers(&headers).unwrap();
    assert_eq!(
        decode(Body::from(stacked), codings, limit).await.unwrap_err(),
        DecodeError::TooLarge
    );
}
