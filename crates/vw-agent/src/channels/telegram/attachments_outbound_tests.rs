use super::TelegramChannel;
use super::attachments::TelegramAttachment;
use axum::{
    Router,
    body::Body,
    extract::{OriginalUri, State},
    http::{StatusCode, header::CONTENT_TYPE},
    response::Response,
    routing::get,
};
use base64::Engine as _;
use image::{DynamicImage, ImageBuffer, ImageFormat, Rgb};
use std::{collections::VecDeque, io::Cursor, sync::Arc};
use tokio::sync::{Mutex, oneshot};

#[derive(Clone, Debug)]
struct ResponseSpec {
    status: StatusCode,
    body: Vec<u8>,
    content_type: &'static str,
}

struct TestServerState {
    paths: Mutex<Vec<String>>,
    responses: Mutex<VecDeque<ResponseSpec>>,
}

struct TestServer {
    base_url: String,
    state: Arc<TestServerState>,
    shutdown: Option<oneshot::Sender<()>>,
}

impl TestServer {
    async fn spawn(responses: Vec<ResponseSpec>) -> Self {
        let state = Arc::new(TestServerState {
            paths: Mutex::new(Vec::new()),
            responses: Mutex::new(VecDeque::from(responses)),
        });
        let app = Router::new().route("/{*path}", get(record_request)).with_state(state.clone());
        let listener =
            tokio::net::TcpListener::bind(("127.0.0.1", 0)).await.expect("bind test server");
        let addr = listener.local_addr().expect("server addr");
        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = shutdown_rx.await;
                })
                .await
                .expect("serve test server");
        });

        Self { base_url: format!("http://{addr}"), state, shutdown: Some(shutdown_tx) }
    }

    async fn paths(&self) -> Vec<String> {
        self.state.paths.lock().await.clone()
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
    }
}

impl ResponseSpec {
    fn json(body: &str) -> Self {
        Self {
            status: StatusCode::OK,
            body: body.as_bytes().to_vec(),
            content_type: "application/json",
        }
    }

    fn bytes(body: Vec<u8>, content_type: &'static str) -> Self {
        Self { status: StatusCode::OK, body, content_type }
    }
}

async fn record_request(
    State(state): State<Arc<TestServerState>>,
    uri: OriginalUri,
) -> Response<Body> {
    state
        .paths
        .lock()
        .await
        .push(uri.path_and_query().map(ToString::to_string).unwrap_or_default());

    let response = state.responses.lock().await.pop_front().expect("test should provide response");

    Response::builder()
        .status(response.status)
        .header(CONTENT_TYPE, response.content_type)
        .body(Body::from(response.body))
        .expect("response should build")
}

fn png_bytes(width: u32, height: u32) -> Vec<u8> {
    let image = ImageBuffer::<Rgb<u8>, Vec<u8>>::from_fn(width, height, |x, y| {
        if (x + y) % 2 == 0 { Rgb([255, 0, 0]) } else { Rgb([0, 0, 255]) }
    });
    let mut bytes = Vec::new();
    DynamicImage::ImageRgb8(image)
        .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
        .expect("encode png");
    bytes
}

fn decoded_data_uri_image(data_uri: &str) -> DynamicImage {
    let encoded = data_uri.strip_prefix("data:image/jpeg;base64,").expect("jpeg data uri prefix");
    let decoded = base64::engine::general_purpose::STANDARD.decode(encoded).expect("base64 jpeg");

    assert!(decoded.starts_with(&[0xFF, 0xD8]));
    image::load_from_memory(&decoded).expect("decoded jpeg image")
}

#[test]
fn telegram_attachment_preserves_target_verbatim() {
    let attachment = TelegramAttachment {
        kind: super::attachments::TelegramAttachmentKind::Image,
        target: "https://example.test/a b.png".to_string(),
    };

    assert_eq!(attachment.target, "https://example.test/a b.png");
}

#[tokio::test]
async fn resolve_photo_data_uri_downloads_and_encodes_image() {
    let image = png_bytes(8, 6);
    let server = TestServer::spawn(vec![
        ResponseSpec::json(r#"{"ok":true,"result":{"file_path":"photos/file.png"}}"#),
        ResponseSpec::bytes(image, "image/png"),
    ])
    .await;
    let channel = TelegramChannel::new("123:ABC".to_string(), vec![], false)
        .with_api_base(server.base_url.clone());

    let data_uri = channel.resolve_photo_data_uri("photo-id").await.expect("data uri");

    let decoded = decoded_data_uri_image(&data_uri);
    assert_eq!((decoded.width(), decoded.height()), (8, 6));

    let paths = server.paths().await;
    assert_eq!(
        paths,
        vec!["/bot123:ABC/getFile?file_id=photo-id", "/file/bot123:ABC/photos/file.png"]
    );
}

#[tokio::test]
async fn resolve_photo_data_uri_errors_when_file_path_missing() {
    let server = TestServer::spawn(vec![ResponseSpec::json(r#"{"ok":true,"result":{}}"#)]).await;
    let channel = TelegramChannel::new("123:ABC".to_string(), vec![], false)
        .with_api_base(server.base_url.clone());

    let error = channel
        .resolve_photo_data_uri("photo-id")
        .await
        .expect_err("missing file path should fail");

    assert!(error.to_string().contains("no file_path"));
    assert_eq!(server.paths().await, vec!["/bot123:ABC/getFile?file_id=photo-id"]);
}

#[tokio::test]
async fn resolve_photo_data_uri_resizes_large_images_to_max_dimension() {
    let image = png_bytes(1024, 256);
    let server = TestServer::spawn(vec![
        ResponseSpec::json(r#"{"ok":true,"result":{"file_path":"photos/wide.png"}}"#),
        ResponseSpec::bytes(image, "image/png"),
    ])
    .await;
    let channel = TelegramChannel::new("123:ABC".to_string(), vec![], false)
        .with_api_base(server.base_url.clone());

    let data_uri = channel.resolve_photo_data_uri("wide-photo").await.expect("data uri");

    let decoded = decoded_data_uri_image(&data_uri);
    assert_eq!((decoded.width(), decoded.height()), (512, 128));
    assert_eq!(
        server.paths().await,
        vec!["/bot123:ABC/getFile?file_id=wide-photo", "/file/bot123:ABC/photos/wide.png"]
    );
}

#[tokio::test]
async fn resolve_photo_data_uri_errors_when_get_file_json_is_invalid() {
    let server = TestServer::spawn(vec![ResponseSpec::json("not json")]).await;
    let channel = TelegramChannel::new("123:ABC".to_string(), vec![], false)
        .with_api_base(server.base_url.clone());

    let error =
        channel.resolve_photo_data_uri("photo-id").await.expect_err("invalid json should fail");

    assert!(!error.to_string().is_empty());
    assert_eq!(server.paths().await, vec!["/bot123:ABC/getFile?file_id=photo-id"]);
}

#[tokio::test]
async fn resolve_photo_data_uri_errors_when_download_is_not_an_image() {
    let server = TestServer::spawn(vec![
        ResponseSpec::json(r#"{"ok":true,"result":{"file_path":"photos/not-image.bin"}}"#),
        ResponseSpec::bytes(b"not an image".to_vec(), "application/octet-stream"),
    ])
    .await;
    let channel = TelegramChannel::new("123:ABC".to_string(), vec![], false)
        .with_api_base(server.base_url.clone());

    let error =
        channel.resolve_photo_data_uri("photo-id").await.expect_err("non-image body should fail");

    assert!(
        error.to_string().contains("format")
            || error.to_string().contains("image")
            || error.to_string().contains("reader")
    );
    assert_eq!(
        server.paths().await,
        vec!["/bot123:ABC/getFile?file_id=photo-id", "/file/bot123:ABC/photos/not-image.bin"]
    );
}
// 本文件验证 Telegram 出站图片的数据地址解析、下载与尺寸约束。
