//! # Embedding Provider 测试模块
//!
//! 本模块包含向量嵌入提供者的单元测试和集成测试。
//!
//! ## 测试覆盖范围
//!
//! - **工厂函数测试**：验证 `create_embedding_provider` 对各种配置的正确处理
//! - **OpenAiEmbedding 测试**：验证 OpenAI 兼容提供者的 URL 构建和维度配置
//!
use super::*;
#[cfg(not(target_arch = "wasm32"))]
use tokio::io::{AsyncReadExt, AsyncWriteExt};
#[cfg(not(target_arch = "wasm32"))]
use tokio::net::TcpListener;



/// 测试工厂函数创建 none 提供者
///
/// # 验证点
///
/// - 当 provider_type 为 `"none"` 时，应返回 NoopEmbedding
/// - 名称应为 `"none"`
#[test]
fn factory_none() {
    let p = create_embedding_provider("none", None, "model", 1536);
    assert_eq!(p.name(), "none");
}

/// 测试工厂函数创建 OpenAI 提供者
///
/// # 验证点
///
/// - 当 provider_type 为 `"openai"` 时，应返回 OpenAI 兼容提供者
/// - 应正确配置 API key、模型名称和维度
#[test]
fn factory_openai() {
    let p = create_embedding_provider("openai", Some("key"), "text-embedding-3-small", 1536);
    assert_eq!(p.name(), "openai");
    assert_eq!(p.dimensions(), 1536);
}

/// 测试工厂函数通过 OpenRouter 创建 OpenAI 兼容提供者
///
/// # 验证点
///
/// - 当 provider_type 为 `"openrouter"` 时，底层使用 OpenAI 兼容实现
/// - OpenRouter 是一个 AI API 聚合服务，支持多种模型提供商
/// - 名称应为 `"openai"`（底层实现）
#[test]
fn factory_openrouter() {
    let p = create_embedding_provider(
        "openrouter",
        Some("sk-or-test"),
        "openai/text-embedding-3-small",
        1536,
    );
    assert_eq!(p.name(), "openai");
    assert_eq!(p.dimensions(), 1536);
}

#[test]
fn factory_alibaba() {
    let p = create_embedding_provider("alibaba", Some("dashscope-key"), "text-embedding-v4", 1024);
    assert_eq!(p.name(), "alibaba");
    assert_eq!(p.dimensions(), 1024);
}

#[test]
fn factory_alibaba_cn() {
    let p =
        create_embedding_provider("alibaba-cn", Some("dashscope-key"), "text-embedding-v4", 1536);
    assert_eq!(p.name(), "alibaba-cn");
    assert_eq!(p.dimensions(), 1536);
}

#[test]
fn factory_alibaba_normalizes_fixed_provider_key() {
    let p =
        create_embedding_provider(" Alibaba-CN ", Some("dashscope-key"), "text-embedding-v4", 1024);
    assert_eq!(p.name(), "alibaba-cn");
}

/// 测试工厂函数创建自定义 URL 的提供者
///
/// # 验证点
///
/// - 支持通过 `custom:<url>` 格式指定自定义 API 端点
/// - 适用于本地部署或私有服务的场景
/// - 底层使用 OpenAI 兼容实现
#[test]
fn factory_custom_url() {
    let p = create_embedding_provider("custom:http://localhost:1234", None, "model", 768);
    assert_eq!(p.name(), "openai");
    assert_eq!(p.dimensions(), 768);
}




/// 测试工厂函数处理空字符串返回 Noop
///
/// # 验证点
///
/// - 当 provider_type 为空字符串时，应回退到 NoopEmbedding
/// - 这是安全的默认行为
#[test]
fn factory_empty_string_returns_noop() {
    let p = create_embedding_provider("", None, "model", 1536);
    assert_eq!(p.name(), "none");
}

/// 测试工厂函数处理未知提供者返回 Noop
///
/// # 验证点
///
/// - 当 provider_type 为不支持的值时，应回退到 NoopEmbedding
/// - 例如 "cohere" 目前不被支持
#[test]
fn factory_unknown_provider_returns_noop() {
    let p = create_embedding_provider("cohere", None, "model", 1536);
    assert_eq!(p.name(), "none");
}

/// 测试工厂函数处理 custom 前缀但空 URL 的情况
///
/// # 验证点
///
/// - 当 provider_type 为 `"custom:"` 但 URL 为空时
/// - 仍应创建 OpenAI 兼容提供者
#[test]
fn factory_custom_empty_url() {
    let p = create_embedding_provider("custom:", None, "model", 768);
    assert_eq!(p.name(), "openai");
}

/// 测试工厂函数在没有 API key 时创建 OpenAI 提供者
///
/// # 验证点
///
/// - 即使没有提供 API key，也应正确创建提供者实例
/// - API key 可能通过环境变量等方式在运行时注入
#[test]
fn factory_openai_no_api_key() {
    let p = create_embedding_provider("openai", None, "text-embedding-3-small", 1536);
    assert_eq!(p.name(), "openai");
    assert_eq!(p.dimensions(), 1536);
}

/// 测试 OpenAI 提供者自动移除 base URL 尾部斜杠
///
/// # 验证点
///
/// - 当 base_url 以 `/` 结尾时，应自动移除
/// - 避免后续拼接时出现双斜杠问题
#[test]
fn openai_trailing_slash_stripped() {
    let p = OpenAiEmbedding::new("https://api.openai.com/", "key", "model", 1536);
    assert_eq!(p.base_url, "https://api.openai.com");
}

/// 测试 OpenAI 提供者支持自定义维度
///
/// # 验证点
///
/// - 维度参数应正确存储和返回
/// - 支持非标准维度值（如 384）
#[test]
fn openai_dimensions_custom() {
    let p = OpenAiEmbedding::new("http://localhost", "k", "m", 384);
    assert_eq!(p.dimensions(), 384);
}

/// 测试 OpenRouter 端点的 embeddings URL 构建
///
/// # 验证点
///
/// - OpenRouter 的 API 端点已包含 `/api/v1`
/// - 不应重复添加 `/v1` 前缀
/// - 最终 URL 应为 `https://openrouter.ai/api/v1/embeddings`
#[test]
fn embeddings_url_openrouter() {
    let p = OpenAiEmbedding::new(
        "https://openrouter.ai/api/v1",
        "key",
        "openai/text-embedding-3-small",
        1536,
    );
    assert_eq!(p.embeddings_url(), "https://openrouter.ai/api/v1/embeddings");
}

#[test]
fn embeddings_url_alibaba_regions() {
    let intl = AlibabaEmbedding::new(
        "alibaba",
        "https://dashscope-intl.aliyuncs.com/compatible-mode/v1",
        "key",
        "text-embedding-v4",
        1024,
    );
    let cn = AlibabaEmbedding::new(
        "alibaba-cn",
        "https://dashscope.aliyuncs.com/compatible-mode/v1",
        "key",
        "text-embedding-v4",
        1024,
    );

    assert_eq!(
        intl.embeddings_url(),
        "https://dashscope-intl.aliyuncs.com/compatible-mode/v1/embeddings"
    );
    assert_eq!(cn.embeddings_url(), "https://dashscope.aliyuncs.com/compatible-mode/v1/embeddings");
}

/// 测试标准 OpenAI API 端点的 embeddings URL 构建
///
/// # 验证点
///
/// - 标准 OpenAI API 端点不含版本前缀
/// - 应自动添加 `/v1/embeddings` 路径
#[test]
fn embeddings_url_standard_openai() {
    let p = OpenAiEmbedding::new("https://api.openai.com", "key", "model", 1536);
    assert_eq!(p.embeddings_url(), "https://api.openai.com/v1/embeddings");
}

/// 测试 base URL 已包含 /v1 时不重复添加
///
/// # 验证点
///
/// - 当 base_url 以 `/v1` 结尾时，直接追加 `/embeddings`
/// - 避免生成 `/v1/v1/embeddings` 这样的错误路径
#[test]
fn embeddings_url_base_with_v1_no_duplicate() {
    let p = OpenAiEmbedding::new("https://api.example.com/v1", "key", "model", 1536);
    assert_eq!(p.embeddings_url(), "https://api.example.com/v1/embeddings");
}

/// 测试非标准 API 路径的 embeddings URL 构建
///
/// # 验证点
///
/// - 当 base_url 包含非 `/v1` 的路径时（如 `/api/coding/v3`）
/// - 直接在原路径后追加 `/embeddings`
/// - 不进行任何版本路径转换
#[test]
fn embeddings_url_non_v1_api_path_uses_raw_suffix() {
    let p = OpenAiEmbedding::new("https://api.example.com/api/coding/v3", "key", "model", 1536);
    assert_eq!(p.embeddings_url(), "https://api.example.com/api/coding/v3/embeddings");
}

/// 测试完整端点 URL 直接作为 base_url 的情况
///
/// # 验证点
///
/// - 当 base_url 已经是完整的 embeddings 端点时
/// - `embeddings_url()` 应原样返回，不做任何修改
/// - 支持用户指定完全自定义的端点地址
#[test]
fn embeddings_url_custom_full_endpoint() {
    let p =
        OpenAiEmbedding::new("https://my-api.example.com/api/v2/embeddings", "key", "model", 1536);
    assert_eq!(p.embeddings_url(), "https://my-api.example.com/api/v2/embeddings");
}

#[test]
fn factory_openai_is_trimmed_and_case_insensitive() {
    let p = create_embedding_provider(" OpenAI ", Some("key"), "model", 42);

    assert_eq!(p.name(), "openai");
    assert_eq!(p.dimensions(), 42);
}

#[test]
fn factory_custom_prefix_is_case_sensitive() {
    let p = create_embedding_provider("Custom:http://localhost:9999", None, "model", 42);

    assert_eq!(p.name(), "none");
}

#[test]
fn openai_strips_multiple_trailing_slashes() {
    let p = OpenAiEmbedding::new("https://api.example.com/v1///", "key", "model", 1536);

    assert_eq!(p.base_url, "https://api.example.com/v1");
    assert_eq!(p.embeddings_url(), "https://api.example.com/v1/embeddings");
}

#[test]
fn embeddings_url_invalid_base_url_falls_back_to_default_path_suffix() {
    let p = OpenAiEmbedding::new("not a url", "key", "model", 1536);

    assert_eq!(p.embeddings_url(), "not a url/v1/embeddings");
}

#[test]
fn embeddings_url_endpoint_with_trailing_slash_is_used_directly() {
    let p = OpenAiEmbedding::new("https://api.example.com/v1/embeddings/", "key", "model", 1536);

    assert_eq!(p.embeddings_url(), "https://api.example.com/v1/embeddings");
}

#[tokio::test]
async fn openai_embed_empty_batch_short_circuits() {
    let p = OpenAiEmbedding::new("http://127.0.0.1:1", "key", "model", 1536);

    assert!(p.embed(&[]).await.unwrap().is_empty());
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn openai_embed_posts_request_and_parses_numeric_embeddings() {
    let (base_url, request) =
        spawn_embedding_server("200 OK", r#"{"data":[{"embedding":[1.5,2,"skip",null]}]}"#).await;
    let p = OpenAiEmbedding::new(&base_url, "secret", "embed-model", 2);

    let embeddings = p.embed(&["alpha", "beta"]).await.unwrap();
    let request = request.await.unwrap();

    assert_eq!(embeddings, vec![vec![1.5, 2.0]]);
    assert!(request.starts_with("POST /v1/embeddings HTTP/1.1"));
    assert!(request.to_ascii_lowercase().contains("authorization: bearer secret"));
    assert!(request.contains(r#""model":"embed-model""#));
    assert!(request.contains(r#""input":["alpha","beta"]"#));
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn openai_embed_reports_http_error_body() {
    let (base_url, request) =
        spawn_embedding_server("500 Internal Server Error", "backend down").await;
    let p = OpenAiEmbedding::new(&base_url, "secret", "embed-model", 2);

    let error = p.embed(&["alpha"]).await.unwrap_err().to_string();
    let _request = request.await.unwrap();

    assert!(error.contains("Embedding API error 500 Internal Server Error"));
    assert!(error.contains("backend down"));
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn openai_embed_rejects_response_without_data() {
    let (base_url, request) = spawn_embedding_server("200 OK", r#"{"usage":{}}"#).await;
    let p = OpenAiEmbedding::new(&base_url, "secret", "embed-model", 2);

    let error = p.embed(&["alpha"]).await.unwrap_err().to_string();
    let _request = request.await.unwrap();

    assert!(error.contains("missing 'data'"));
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn openai_embed_rejects_items_without_embedding_array() {
    let (base_url, request) = spawn_embedding_server("200 OK", r#"{"data":[{}]}"#).await;
    let p = OpenAiEmbedding::new(&base_url, "secret", "embed-model", 2);

    let error = p.embed(&["alpha"]).await.unwrap_err().to_string();
    let _request = request.await.unwrap();

    assert!(error.contains("Invalid embedding item"));
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
