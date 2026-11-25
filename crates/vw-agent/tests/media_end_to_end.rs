#![cfg(not(target_arch = "wasm32"))]

use std::{
    io::Cursor,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering::Relaxed},
    },
    time::Duration,
};

use axum::{
    Json, Router,
    body::Bytes,
    extract::{Path as RoutePath, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use futures_util::TryStreamExt as _;
use gateway_endpoint::{GatewayAuth, GatewayEndpoint};
use serde_json::{Value, json};
use tokio::{net::TcpListener, sync::Notify, task::JoinHandle};
use vw_agent::{config, gateway, project, session::ui_store};
use vw_api_transport::TenantScope;
use vw_gateway_client::{
    GatewayChatStreamEvent, GatewayChatStreamRequest, GatewayClient,
    media::{
        MEDIA_MODEL_SELECTION_OPTION, MediaApiError, MediaAsset, MediaErrorCode as Code, MediaInput,
        MediaJobId, MediaJobState, MediaModel, MediaModelSelection, MediaOperation as Op, MediaResult,
        MediaSessionQuery, MediaSubmit,
    },
};

const CHILD_ROOT: &str = "SHIJU_MEDIA_E2E_ROOT";
const TOKEN: &str = "sk-media-e2e-fixture";
const OTHER_TOKEN: &str = "sk-media-e2e-other-fixture";
const MANAGED_KEY: &str = "managed-fixture-key";
const DIRECT_KEY: &str = "direct-fixture-key";
const MANAGED_ENV: &str = "SHIJU_MEDIA_E2E_MANAGED_KEY";
const DIRECT_ENV: &str = "SHIJU_MEDIA_E2E_DIRECT_KEY";
const PLATFORM: &str = "media-e2e";
const COMPANY: &str = "fixture-company";
const SESSION: &str = "media-session";
const OTHER_SESSION: &str = "other-media-session";
const IMAGE: &str = "fixture-image";
const MANAGED_IMAGE: &str = "managed-image-wire";
const DIRECT_IMAGE: &str = "direct-image-wire";
const VIDEO: &str = "fixture-video";
const SPEECH: &str = "fixture-speech";
const ASR: &str = "fixture-asr";
const VISION: &str = "fixture-vision";
const VISION_WIRE: &str = "qwen3-vl-plus";
const CLIP: &[u8] = b"fixture-video-bytes";
const VOICE: &[u8] = b"ID3fixture-audio";

#[derive(Default)]
struct Wire {
    png: Vec<u8>,
    managed_images: AtomicUsize,
    direct_images: AtomicUsize,
    video_submits: AtomicUsize,
    canceled: Notify,
}

fn authorize(headers: &HeaderMap, expected: &str) {
    assert_eq!(headers[header::AUTHORIZATION].to_str().unwrap(), format!("Bearer {expected}"));
}

fn image_url(png: &[u8]) -> String {
    format!("data:image/png;base64,{}", STANDARD.encode(png))
}

async fn managed_catalog(headers: HeaderMap) -> Json<Value> {
    authorize(&headers, MANAGED_KEY);
    Json(json!({"data": [
        {"id": MANAGED_IMAGE, "supported_endpoint_types": ["gemini-chat-image"]},
        {"id": VIDEO, "supported_endpoint_types": ["openai-video"]},
        {"id": SPEECH, "supported_endpoint_types": ["audio-speech"]},
        {"id": ASR, "supported_endpoint_types": ["audio-transcription"]},
        {"id": "ungranted-image"}
    ]}))
}

async fn managed_image(
    State(wire): State<Arc<Wire>>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Json<Value> {
    authorize(&headers, MANAGED_KEY);
    assert_eq!(body["model"], MANAGED_IMAGE);
    assert_eq!(body["stream"], false);
    assert_eq!(body["modalities"], json!(["image", "text"]));
    let content = &body["messages"][0]["content"];
    match content[0]["text"].as_str() {
        Some("draw") => assert_eq!(content.as_array().unwrap().len(), 1),
        Some("edit") => assert_eq!(content[1]["image_url"]["url"], image_url(&wire.png)),
        other => panic!("unexpected managed image prompt {other:?}"),
    }
    wire.managed_images.fetch_add(1, Relaxed);
    Json(json!({"choices": [{"message": {"images": [{
        "image_url": {"url": image_url(&wire.png)}
    }]}}]}))
}

async fn direct_image(
    State(wire): State<Arc<Wire>>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    authorize(&headers, DIRECT_KEY);
    assert_eq!(body["model"], DIRECT_IMAGE);
    wire.direct_images.fetch_add(1, Relaxed);
    if body["prompt"] == "reject" {
        return (StatusCode::BAD_REQUEST, DIRECT_KEY).into_response();
    }
    assert_eq!(body["prompt"], "draw");
    Json(json!({"data": [{"b64_json": STANDARD.encode(&wire.png)}]})).into_response()
}

async fn start_video(State(wire): State<Arc<Wire>>, headers: HeaderMap, body: Bytes) -> Response {
    authorize(&headers, MANAGED_KEY);
    wire.video_submits.fetch_add(1, Relaxed);
    if headers[header::CONTENT_TYPE].to_str().unwrap().starts_with("multipart/form-data;") {
        let contains = |part: &[u8]| body.windows(part.len()).any(|window| window == part);
        assert!(contains(format!("name=\"model\"\r\n\r\n{VIDEO}\r\n").as_bytes()));
        assert!(contains(b"name=\"prompt\"\r\n\r\nfinish\r\n"));
        assert!(contains(b"name=\"input_reference\"; filename=\"reference.png\""));
        assert!(contains(&wire.png));
        return Json(json!({"id": "finish", "status": "queued"})).into_response();
    }
    let body: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(body["model"], VIDEO);
    match body["prompt"].as_str() {
        Some("cancel") => {
            Json(json!({"id": "cancel", "status": "in_progress", "progress": 30})).into_response()
        }
        Some("uncertain") => StatusCode::SERVICE_UNAVAILABLE.into_response(),
        other => panic!("unexpected video prompt {other:?}"),
    }
}

async fn video_status(headers: HeaderMap, RoutePath(id): RoutePath<String>) -> Json<Value> {
    authorize(&headers, MANAGED_KEY);
    match id.as_str() {
        "cancel" => Json(json!({"id": id, "status": "in_progress", "progress": 30})),
        "finish" => Json(json!({"id": id, "status": "completed", "progress": 100})),
        _ => panic!("unexpected video ID {id}"),
    }
}

async fn cancel_video(
    State(wire): State<Arc<Wire>>,
    headers: HeaderMap,
    RoutePath(id): RoutePath<String>,
) -> StatusCode {
    authorize(&headers, MANAGED_KEY);
    assert_eq!(id, "cancel");
    wire.canceled.notify_one();
    StatusCode::NO_CONTENT
}

async fn video_content(headers: HeaderMap, RoutePath(id): RoutePath<String>) -> impl IntoResponse {
    authorize(&headers, MANAGED_KEY);
    assert_eq!(id, "finish");
    ([(header::CONTENT_TYPE, "video/mp4")], Bytes::from_static(CLIP))
}

async fn speech(headers: HeaderMap, Json(body): Json<Value>) -> impl IntoResponse {
    authorize(&headers, MANAGED_KEY);
    assert_eq!(body["model"], SPEECH);
    assert_eq!(body["input"], "read this");
    ([(header::CONTENT_TYPE, "audio/mpeg")], Bytes::from_static(VOICE))
}

async fn transcription(headers: HeaderMap, body: Bytes) -> Json<Value> {
    authorize(&headers, MANAGED_KEY);
    assert!(headers[header::CONTENT_TYPE].to_str().unwrap().starts_with("multipart/form-data;"));
    let body = std::str::from_utf8(&body).unwrap();
    assert!(body.contains(&format!("name=\"model\"\r\n\r\n{ASR}\r\n")));
    let (media, text) = if body.contains("name=\"file\"; filename=\"media.mp3\"") {
        (VOICE, "read this")
    } else {
        assert!(body.contains("name=\"file\"; filename=\"media.mp4\""));
        (CLIP, "clip transcript")
    };
    assert!(body.contains(std::str::from_utf8(media).unwrap()));
    Json(json!({"text": text}))
}

async fn vision(
    State(wire): State<Arc<Wire>>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    authorize(&headers, DIRECT_KEY);
    assert_eq!(body["model"], VISION_WIRE);
    assert_eq!(body["stream"], true);
    let content = body["messages"][0]["content"].as_array().unwrap();
    let answer = match content[1]["type"].as_str() {
        Some("image_url") => {
            assert_eq!(content[1]["image_url"]["url"], image_url(&wire.png));
            "image description"
        }
        Some("video_url") => {
            assert_eq!(
                content[1]["video_url"]["url"],
                format!("data:video/mp4;base64,{}", STANDARD.encode(CLIP))
            );
            "video description"
        }
        other => panic!("unexpected vision input {other:?}"),
    };
    let chunk = |delta, finish_reason| {
        json!({
            "id": "chatcmpl-fixture", "object": "chat.completion.chunk",
            "created": 0, "model": VISION_WIRE,
            "choices": [{"index": 0, "delta": delta, "finish_reason": finish_reason}],
        })
    };
    let text = chunk(json!({"content": answer}), Value::Null);
    let stop = chunk(json!({}), json!("stop"));
    (
        [(header::CONTENT_TYPE, "text/event-stream")],
        format!("data: {text}\n\ndata: {stop}\n\ndata: [DONE]\n\n"),
    )
}

struct Server<T>(JoinHandle<T>);

impl<T> Drop for Server<T> {
    fn drop(&mut self) {
        self.0.abort();
    }
}

async fn provider_fixture() -> (String, Arc<Wire>, Server<()>) {
    let mut png = Cursor::new(Vec::new());
    image::DynamicImage::new_rgb8(2, 3).write_to(&mut png, image::ImageFormat::Png).unwrap();
    let wire = Arc::new(Wire { png: png.into_inner(), ..Wire::default() });
    let router = Router::new()
        .route("/managed/v1/models", get(managed_catalog))
        .route("/managed/v1/chat/completions", post(managed_image))
        .route("/managed/v1/videos", post(start_video))
        .route("/managed/v1/videos/{id}", get(video_status).delete(cancel_video))
        .route("/managed/v1/videos/{id}/content", get(video_content))
        .route("/managed/v1/audio/speech", post(speech))
        .route("/managed/v1/audio/transcriptions", post(transcription))
        .route(
            "/direct/v1/models",
            get(|headers: HeaderMap| async move {
                authorize(&headers, DIRECT_KEY);
                Json(json!({"data": [{"id": DIRECT_IMAGE}, {"id": VISION_WIRE}]}))
            }),
        )
        .route("/direct/v1/images/generations", post(direct_image))
        .route("/direct/v1/chat/completions", post(vision))
        .with_state(wire.clone());
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let server = Server(tokio::spawn(async move { axum::serve(listener, router).await.unwrap() }));
    (origin, wire, server)
}

fn tenant() -> TenantScope {
    TenantScope::new(PLATFORM, COMPANY).unwrap()
}

async fn configure(root: &Path, origin: &str) {
    config::update_global_for_tenant(
        &tenant(),
        json!({
            "gateway": {"auth_enabled": true, "skeys": [
                {"skey": TOKEN, "name": "media fixture"},
                {"skey": OTHER_TOKEN, "name": "other principal"}
            ]},
            "providers": {
                "shiju": {
                    "api": format!("{origin}/managed/v1"), "adapter": "openai-compatible",
                    "env": [MANAGED_ENV],
                    "models": {
                        (IMAGE): {"id": MANAGED_IMAGE, "status": "active"},
                        (VIDEO): {"status": "active", "options": {
                            "media_video_cancel": true, "media_video_image_reference": true
                        }},
                        (SPEECH): {"status": "active"},
                        (ASR): {"status": "active"},
                        "ungranted-image": {"status": "active", "options": {
                            "media_endpoint_types": ["image-generation"]
                        }}
                    }
                },
                "alibaba-cn": {
                    "api": format!("{origin}/direct/v1"), "adapter": "openai-compatible",
                    "env": [DIRECT_ENV],
                    "models": {
                        (IMAGE): {"id": DIRECT_IMAGE, "status": "active", "options": {
                            "media_endpoint_types": ["image-generation"]
                        }},
                        (VISION): {"id": VISION_WIRE, "status": "active",
                            "modalities": {"input": ["text", "image", "video"], "output": ["text"]},
                            "options": {"media_endpoint_types": ["chat-completions"]}
                        }
                    }
                }
            }
        }),
    )
    .await
    .unwrap();

    let workspace = root.join("project");
    std::fs::create_dir(&workspace).unwrap();
    let (project, _) = project::from_directory(&tenant(), &workspace).await.unwrap();
    for id in [SESSION, OTHER_SESSION] {
        let info = serde_json::from_value(json!({
            "id": id, "slug": id, "projectID": project.id, "directory": workspace,
            "title": id, "version": "1", "time": {"created": 1, "updated": 1}
        }))
        .unwrap();
        ui_store::save_agent_session_scoped_for_tenant(&tenant(), &info, Some(&project.id))
            .unwrap();
    }
}

fn client(port: u16, token: Option<&str>) -> GatewayClient {
    let endpoint = GatewayEndpoint::new("127.0.0.1", port)
        .with_tenant_scope(tenant())
        .with_auth(GatewayAuth { skey: token.map(str::to_owned) });
    GatewayClient::new(endpoint).unwrap()
}

fn scope(id: &str) -> MediaSessionQuery {
    MediaSessionQuery { session_id: id.into() }
}

fn model<'a>(models: &'a [MediaModel], provider: &str, id: &str, operation: Op) -> &'a MediaModel {
    models
        .iter()
        .find(|model| {
            model.deployment_id.split_once(':').is_some_and(|(id, _)| id == provider)
                && model.model_id == id
                && model.operations.contains(&operation)
        })
        .unwrap_or_else(|| panic!("missing {provider}/{id} for {operation:?} in {models:?}"))
}

fn request(model: &MediaModel, operation: Op, prompt: &str) -> MediaSubmit {
    MediaSubmit {
        request_id: MediaJobId::new_v4(),
        session_id: SESSION.into(),
        operation,
        deployment_id: model.deployment_id.clone(),
        model_id: model.model_id.clone(),
        text: Some(prompt.into()),
        inputs: vec![],
    }
}

async fn wait_for(
    client: &GatewayClient,
    id: MediaJobId,
    ready: impl Fn(&MediaJobState) -> bool,
) -> MediaJobState {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let state = client.media_job(&scope(SESSION), id).await.unwrap().state;
            if ready(&state) {
                return state;
            }
            assert!(
                matches!(state, MediaJobState::Submitted | MediaJobState::Running { .. }),
                "job {id} ended unexpectedly in {state:?}"
            );
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("job {id} did not reach the expected state"))
}

async fn finish(client: &GatewayClient, id: MediaJobId) -> MediaJobState {
    wait_for(client, id, |state| {
        !matches!(state, MediaJobState::Submitted | MediaJobState::Running { .. })
    })
    .await
}

async fn submit_success(client: &GatewayClient, request: &MediaSubmit) -> MediaResult {
    let job = client.media_submit(request).await.unwrap();
    assert_eq!(job.id, request.request_id);
    match finish(client, job.id).await {
        MediaJobState::Succeeded { result } => result,
        state => panic!("expected {:?} success, got {state:?}", request.operation),
    }
}

async fn download(client: &GatewayClient, id: MediaJobId, asset: &MediaAsset) -> Vec<u8> {
    client.media_asset(&scope(SESSION), id, asset).await.unwrap().try_concat().await.unwrap()
}

fn reference(result: &MediaResult) -> MediaInput {
    let [asset] = result.assets.as_slice() else { panic!("expected one asset") };
    MediaInput::Asset { asset_id: asset.id.clone() }
}

async fn assert_selected_vision_turn(owner: &GatewayClient, root: &Path, wire: &Wire, vision: &MediaModel) {
    let file = root.join("project").join("input.png");
    std::fs::write(&file, &wire.png).unwrap();
    let selection = MediaModelSelection {
        operation: Op::ImageUnderstand,
        deployment_id: vision.deployment_id.clone(),
        model_id: vision.model_id.clone(),
    };
    let request = GatewayChatStreamRequest {
        acp_model_source: Default::default(),
        session_id: Some(SESSION.into()),
        messages: vec![json!({
            "role": "user",
            "content": format!("describe\n\n[IMAGE:{}]", file.display()),
        })],
        system: None,
        model: Some(format!("alibaba-cn/{IMAGE}")),
        reasoning_effort: None,
        agent: None,
        allowed_tools: None,
        acp_allowed_tools: None,
        options: Some(json!({(MEDIA_MODEL_SELECTION_OPTION): selection})),
    };
    let expected_model = format!("alibaba-cn/{VISION}");
    let mut text = String::new();
    let mut reported_model = false;
    let mut persisted = false;
    GatewayClient::stream_chat(owner.endpoint(), file.parent().unwrap().to_str(), &request, |event| {
        match event {
            GatewayChatStreamEvent::Delta(delta) => text.push_str(&delta),
            GatewayChatStreamEvent::Other(value) if value["type"] == "chat.step_finish" => {
                reported_model = value["model"] == expected_model.as_str();
            }
            GatewayChatStreamEvent::Done { message_id, .. } => persisted = message_id.is_some(),
            GatewayChatStreamEvent::Error(error) => panic!("selected vision turn failed: {error}"),
            _ => {}
        }
        true
    })
    .await
    .unwrap();
    assert_eq!(text, "image description");
    assert!(reported_model && persisted);
}

#[tokio::test]
async fn media_end_to_end_routes_providers_and_recovers_session_jobs() {
    let Some(root) = std::env::var_os(CHILD_ROOT) else {
        // The linked gateway caches path and configuration state at process scope.
        let root = tempfile::tempdir().unwrap();
        let root_path = root.path().canonicalize().unwrap();
        let mut child = tokio::process::Command::new(std::env::current_exe().unwrap());
        child
            .args(["--exact", "media_end_to_end_routes_providers_and_recovers_session_jobs"])
            .env_clear()
            .env(CHILD_ROOT, &root_path)
            .env("HOME", &root_path)
            .env("VIBEWINDOW_TEST_ROOT", &root_path)
            .env(MANAGED_ENV, MANAGED_KEY)
            .env(DIRECT_ENV, DIRECT_KEY)
            .current_dir(&root_path)
            .kill_on_drop(true);
        #[cfg(windows)]
        if let Some(system_root) = std::env::var_os("SystemRoot") {
            child.env("SystemRoot", system_root);
        }
        let output = tokio::time::timeout(Duration::from_secs(50), child.output())
            .await
            .expect("isolated media acceptance timed out")
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success() && stdout.contains("1 passed; 0 failed"),
            "{stdout}\n{}",
            String::from_utf8_lossy(&output.stderr),
        );
        return;
    };

    let root = Path::new(&root);
    assert_eq!(vw_agent::global::paths().home, root);
    let (origin, wire, _upstream) = provider_fixture().await;
    configure(root, &origin).await;
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let gateway_config = vw_agent::Config {
        workspace_dir: root.join("project"),
        config_path: root.join("config.json"),
        ..Default::default()
    };
    let _gateway = Server(tokio::spawn(async move {
        gateway::run_gateway("127.0.0.1", port, gateway_config).await.unwrap();
    }));
    let health = format!("http://127.0.0.1:{port}/v1/health");
    let probe = reqwest::Client::builder().no_proxy().build().unwrap();
    tokio::time::timeout(Duration::from_secs(8), async {
        while !probe.get(&health).send().await.is_ok_and(|response| response.status().is_success()) {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("full chat gateway did not start");
    let owner = client(port, Some(TOKEN));
    let session = scope(SESSION);
    let models = owner.media_models(&session).await.unwrap().models;
    let managed_image = model(&models, "shiju", IMAGE, Op::ImageGenerate);
    let direct_image = model(&models, "alibaba-cn", IMAGE, Op::ImageGenerate);
    assert_ne!(managed_image.deployment_id, direct_image.deployment_id);
    assert!(managed_image.operations.contains(&Op::ImageEdit));
    assert!(!direct_image.operations.contains(&Op::ImageEdit));
    assert!(!models.iter().any(|model| model.model_id == "ungranted-image"));
    assert_selected_vision_turn(
        &owner,
        root,
        &wire,
        model(&models, "alibaba-cn", VISION, Op::ImageUnderstand),
    )
    .await;

    let draw = request(managed_image, Op::ImageGenerate, "draw");
    let mut ungranted = draw.clone();
    ungranted.model_id = "ungranted-image".into();
    assert_eq!(owner.media_submit(&ungranted).await, Err(MediaApiError::Service(Code::Forbidden)));
    assert_eq!(
        client(port, None).media_submit(&draw).await,
        Err(MediaApiError::Service(Code::Unauthorized)),
    );
    let (first, duplicate) = tokio::join!(owner.media_submit(&draw), owner.media_submit(&draw));
    assert_eq!(first.unwrap().id, draw.request_id);
    assert_eq!(duplicate.unwrap().id, draw.request_id);
    let MediaJobState::Succeeded { result: drawn } = finish(&owner, draw.request_id).await else {
        panic!("managed image did not complete");
    };
    assert_eq!(wire.managed_images.load(Relaxed), 1);
    let [asset] = drawn.assets.as_slice() else { panic!("expected one PNG") };
    assert_eq!((asset.mime.as_str(), asset.width, asset.height), ("image/png", Some(2), Some(3)));
    assert_eq!(download(&owner, draw.request_id, asset).await, wire.png);
    let foreign = client(port, Some(OTHER_TOKEN));
    assert_eq!(
        foreign.media_job(&session, draw.request_id).await,
        Err(MediaApiError::Service(Code::NotFound))
    );
    assert!(matches!(
        foreign.media_asset(&session, draw.request_id, asset).await,
        Err(MediaApiError::Service(Code::NotFound))
    ));
    assert_eq!(
        owner.media_job(&scope(OTHER_SESSION), draw.request_id).await,
        Err(MediaApiError::Service(Code::NotFound))
    );
    let mut conflict = draw.clone();
    conflict.text = Some("changed prompt".into());
    assert_eq!(owner.media_submit(&conflict).await, Err(MediaApiError::Service(Code::Conflict)));

    let direct = request(direct_image, Op::ImageGenerate, "draw");
    let result = submit_success(&owner, &direct).await;
    assert_eq!(download(&owner, direct.request_id, &result.assets[0]).await, wire.png);
    let mut edit = request(managed_image, Op::ImageEdit, "edit");
    edit.inputs.push(reference(&drawn));
    assert_eq!(submit_success(&owner, &edit).await.assets.len(), 1);
    let reject = request(direct_image, Op::ImageGenerate, "reject");
    owner.media_submit(&reject).await.unwrap();
    assert_eq!(
        finish(&owner, reject.request_id).await,
        MediaJobState::Failed { code: Code::UpstreamFailed }
    );
    assert_eq!(wire.managed_images.load(Relaxed), 2);
    assert_eq!(wire.direct_images.load(Relaxed), 2);

    let video = model(&models, "shiju", VIDEO, Op::VideoGenerate);
    let cancel = request(video, Op::VideoGenerate, "cancel");
    owner.media_submit(&cancel).await.unwrap();
    drop(owner);
    let owner = client(port, Some(TOKEN));
    wait_for(&owner, cancel.request_id, |state| {
        *state == MediaJobState::Running { progress_percent: Some(30) }
    })
    .await;
    assert_eq!(
        owner.media_cancel(&session, cancel.request_id).await.unwrap().state,
        MediaJobState::Canceled
    );
    tokio::time::timeout(Duration::from_secs(8), wire.canceled.notified()).await.unwrap();
    assert_eq!(finish(&owner, cancel.request_id).await, MediaJobState::Canceled);
    assert_eq!(owner.media_submit(&cancel).await.unwrap().state, MediaJobState::Canceled);

    let mut completed = request(video, Op::VideoGenerate, "finish");
    completed.inputs.push(reference(&drawn));
    let clip = submit_success(&owner, &completed).await;
    let [asset] = clip.assets.as_slice() else { panic!("expected one video") };
    assert_eq!(asset.mime, "video/mp4");
    assert_eq!(download(&owner, completed.request_id, asset).await, CLIP);
    let uncertain = request(video, Op::VideoGenerate, "uncertain");
    owner.media_submit(&uncertain).await.unwrap();
    assert_eq!(finish(&owner, uncertain.request_id).await, MediaJobState::SubmissionUnknown);
    assert_eq!(
        owner.media_submit(&uncertain).await.unwrap().state,
        MediaJobState::SubmissionUnknown
    );
    assert_eq!(wire.video_submits.load(Relaxed), 3);

    let synth = request(
        model(&models, "shiju", SPEECH, Op::SpeechSynthesize),
        Op::SpeechSynthesize,
        "read this",
    );
    let speech = submit_success(&owner, &synth).await;
    let [asset] = speech.assets.as_slice() else { panic!("expected one audio file") };
    assert_eq!(asset.mime, "audio/mpeg");
    assert_eq!(download(&owner, synth.request_id, asset).await, VOICE);

    for (provider, id, operation, input, expected) in [
        ("shiju", ASR, Op::AudioTranscribe, &speech, "read this"),
        ("shiju", ASR, Op::AudioTranscribe, &clip, "clip transcript"),
        ("alibaba-cn", VISION, Op::ImageUnderstand, &drawn, "image description"),
        ("alibaba-cn", VISION, Op::VideoUnderstand, &clip, "video description"),
    ] {
        let mut request = request(model(&models, provider, id, operation), operation, "describe");
        request.inputs.push(reference(input));
        let result = submit_success(&owner, &request).await;
        assert_eq!(result.text.as_deref(), Some(expected));
        assert!(result.assets.is_empty());
    }

    let mut unsupported =
        request(model(&models, "shiju", ASR, Op::AudioTranscribe), Op::AudioTranscribe, "describe");
    unsupported.inputs.push(MediaInput::upload("video/quicktime".into(), CLIP).unwrap());
    assert_eq!(
        owner.media_submit(&unsupported).await,
        Err(MediaApiError::Service(Code::UnsupportedMedia))
    );
}
