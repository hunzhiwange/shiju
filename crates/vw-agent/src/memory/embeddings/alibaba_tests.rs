use super::*;

#[cfg(not(target_arch = "wasm32"))]
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

#[test]
fn constructor_normalizes_trailing_slashes_and_preserves_metadata() {
    let provider = AlibabaEmbedding::new(
        "alibaba-cn",
        "https://dashscope.example/v1///",
        "secret",
        "text-embedding-v4",
        768,
    );

    assert_eq!(provider.name(), "alibaba-cn");
    assert_eq!(provider.dimensions(), 768);
    assert_eq!(provider.embeddings_url(), "https://dashscope.example/v1/embeddings");
}

#[test]
fn embeddings_url_appends_one_path_segment_when_base_has_no_trailing_slash() {
    let provider =
        AlibabaEmbedding::new("alibaba", "https://dashscope.example/v1", "key", "model", 1);

    assert_eq!(provider.embeddings_url(), "https://dashscope.example/v1/embeddings");
}

#[tokio::test]
async fn empty_embedding_batch_returns_without_building_a_request() {
    let provider = AlibabaEmbedding::new("alibaba", "not a URL", "", "", 0);

    let embeddings = provider.embed(&[]).await.expect("empty input should not require HTTP");

    assert!(embeddings.is_empty());
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn embed_posts_dashscope_payload_and_decodes_float_embeddings() {
    let (base_url, request) =
        spawn_embedding_server("200 OK", r#"{"data":[{"embedding":[1,2.5,"skip",3]}]}"#).await;
    let provider = AlibabaEmbedding::new("alibaba", &base_url, "secret-key", "embed-model", 3);

    let embeddings = provider.embed(&["alpha", "beta"]).await.unwrap();
    let request = request.await.unwrap();

    assert_eq!(embeddings, vec![vec![1.0, 2.5, 3.0]]);
    assert!(request.starts_with("POST /embeddings HTTP/1.1"));
    assert!(request.contains("authorization: Bearer secret-key"));
    assert!(request.contains(r#""model":"embed-model""#));
    assert!(request.contains(r#""input":["alpha","beta"]"#));
    assert!(request.contains(r#""dimensions":3"#));
    assert!(request.contains(r#""encoding_format":"float""#));
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn embed_reports_non_success_status_with_response_body() {
    let (base_url, request) =
        spawn_embedding_server("429 Too Many Requests", r#"{"error":"rate limited"}"#).await;
    let provider = AlibabaEmbedding::new("alibaba", &base_url, "secret-key", "embed-model", 3);

    let error = provider.embed(&["alpha"]).await.unwrap_err().to_string();
    let _request = request.await.unwrap();

    assert!(error.contains("Alibaba embedding API error 429 Too Many Requests"));
    assert!(error.contains("rate limited"));
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn embed_rejects_success_response_without_data_array() {
    let (base_url, request) = spawn_embedding_server("200 OK", r#"{"result":[]}"#).await;
    let provider = AlibabaEmbedding::new("alibaba", &base_url, "secret-key", "embed-model", 3);

    let error = provider.embed(&["alpha"]).await.unwrap_err().to_string();
    let _request = request.await.unwrap();

    assert!(error.contains("Invalid Alibaba embedding response: missing 'data'"));
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn embed_rejects_items_without_embedding_array() {
    let (base_url, request) = spawn_embedding_server("200 OK", r#"{"data":[{}]}"#).await;
    let provider = AlibabaEmbedding::new("alibaba", &base_url, "secret-key", "embed-model", 3);

    let error = provider.embed(&["alpha"]).await.unwrap_err().to_string();
    let _request = request.await.unwrap();

    assert!(error.contains("Invalid Alibaba embedding item"));
}

#[cfg(not(target_arch = "wasm32"))]
async fn spawn_embedding_server(
    status: &str,
    body: &str,
) -> (String, tokio::task::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let status = status.to_string();
    let body = body.to_string();

    let handle = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        let mut buffer = [0_u8; 1024];

        loop {
            let read = socket.read(&mut buffer).await.unwrap();
            if read == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..read]);
            if http_request_is_complete(&request) {
                break;
            }
        }

        let response = format!(
            "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
            body.len()
        );
        socket.write_all(response.as_bytes()).await.unwrap();
        String::from_utf8_lossy(&request).to_string()
    });

    (format!("http://{address}"), handle)
}

#[cfg(not(target_arch = "wasm32"))]
fn http_request_is_complete(request: &[u8]) -> bool {
    let Some(header_end) = request.windows(4).position(|window| window == b"\r\n\r\n") else {
        return false;
    };
    let body_start = header_end + 4;
    let headers = String::from_utf8_lossy(&request[..header_end]);
    let content_length = headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().ok())
                .flatten()
        })
        .unwrap_or(0);

    request.len() >= body_start + content_length
}
