use super::*;

#[test]
fn unpaired_catalog_requires_a_direct_local_default_scope_request() {
    let tenant = TenantScope::default();
    let other_tenant = TenantScope::new("media-fixture", "other").unwrap();
    let local = Some(Extension(LocalMediaCatalog));
    let loopback = Some(Extension(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 1234)))));
    let remote = Some(Extension(ConnectInfo(SocketAddr::from(([192, 0, 2, 1], 1234)))));
    let global = Query::try_from_uri(&"/?global=true".parse().unwrap());
    let session = Query::try_from_uri(&"/?session_id=fixture".parse().unwrap());
    let project = Query::try_from_uri(&"/?project_id=fixture".parse().unwrap());
    let mut headers = HeaderMap::new();

    assert!(is_unpaired_local_catalog(&global, &tenant, local, loopback, &headers));
    for (query, tenant, local, peer) in [
        (&session, &tenant, local, loopback),
        (&project, &tenant, local, loopback),
        (&global, &other_tenant, local, loopback),
        (&global, &tenant, None, loopback),
        (&global, &tenant, local, None),
        (&global, &tenant, local, remote),
    ] {
        assert!(!is_unpaired_local_catalog(query, tenant, local, peer, &headers));
    }

    headers.insert("x-forwarded-for", "192.0.2.1".parse().unwrap());
    assert!(!is_unpaired_local_catalog(&global, &tenant, local, loopback, &headers));
    headers.clear();
    headers.insert(header::AUTHORIZATION, "Bearer fixture".parse().unwrap());
    assert!(!is_unpaired_local_catalog(&global, &tenant, local, loopback, &headers));
}

#[test]
fn media_gateway_flow_rechecks_input_shape() {
    let file = |mime: &str| MediaFile { mime: mime.into(), bytes: vec![1] };
    assert_eq!(
        validate_inputs(MediaOperation::VideoGenerate, Some("go"), &[file("image/png")]),
        Ok(())
    );
    assert_eq!(
        validate_inputs(MediaOperation::VideoGenerate, Some("go"), &[file("video/mp4")]),
        Err(Code::UnsupportedMedia)
    );
    assert_eq!(
        validate_inputs(MediaOperation::AudioTranscribe, None, &[]),
        Err(Code::UnsupportedMedia)
    );
    assert_eq!(validate_inputs(MediaOperation::AudioTranscribe, None, &[file("video/mp4")]), Ok(()));
    assert_eq!(
        validate_inputs(MediaOperation::AudioTranscribe, None, &[file("video/quicktime")]),
        Err(Code::UnsupportedMedia)
    );
}

use std::{
    io::Cursor,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use agent_gateway::api::media::MediaJob;
use axum::http::Method;
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use tokio::sync::Notify;

const TOKEN: &str = "sk-media-fixture-client";
const OTHER_TOKEN: &str = "sk-media-fixture-other";
const SECRET: &str = "fixture-provider-secret";
const SESSION: &str = "fixture-session";
static FIXTURES: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

struct Server {
    base: String,
    task: tokio::task::JoinHandle<()>,
}

impl Server {
    async fn start(router: Router) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let task = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        Self { base, task }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}

struct Credential(String);

impl Drop for Credential {
    fn drop(&mut self) {
        agent_env::remove(&self.0);
    }
}

pub(crate) struct Fixture {
    gateway: Server,
    _upstream: Server,
    workspace: tempfile::TempDir,
    _credential: Credential,
    platform: String,
    client: reqwest::Client,
    pub(crate) model: MediaModel,
    image: Vec<u8>,
    image_posts: Arc<AtomicUsize>,
    video_posts: Arc<AtomicUsize>,
    video_polled: Arc<Notify>,
    video_canceled: Arc<Notify>,
    _serial: tokio::sync::MutexGuard<'static, ()>,
}

impl Fixture {
    pub(crate) async fn new() -> Self {
        Self::with_provider(None).await
    }

    pub(crate) async fn new_shiju() -> Self {
        Self::with_provider(Some(deployments::SHIJU_PROVIDER_ID)).await
    }

    async fn with_provider(provider: Option<&str>) -> Self {
        let serial = FIXTURES.lock().await;
        let mut image = Cursor::new(Vec::new());
        image::DynamicImage::new_rgb8(2, 3).write_to(&mut image, image::ImageFormat::Png).unwrap();
        let image = image.into_inner();
        let encoded = STANDARD.encode(&image);
        let image_posts = Arc::new(AtomicUsize::new(0));
        let video_posts = Arc::new(AtomicUsize::new(0));
        let video_polled = Arc::new(Notify::new());
        let video_canceled = Arc::new(Notify::new());
        let upstream = Server::start(Router::new()
            .route("/v1/models", get(|headers: HeaderMap| async move {
                assert_eq!(headers[header::AUTHORIZATION].to_str().unwrap(), format!("Bearer {SECRET}"));
                Json(json!({"data": [{
                    "id": "fixture-multimodal-model",
                    "supported_endpoint_types": ["image-generation", "openai-video"]
                }]}))
            }))
            .route("/v1/images/generations", post({
                let posts = image_posts.clone();
                move |headers: HeaderMap, Json(input): Json<Value>| {
                    let posts = posts.clone();
                    let encoded = encoded.clone();
                    async move {
                        assert_eq!(headers[header::AUTHORIZATION].to_str().unwrap(), format!("Bearer {SECRET}"));
                        assert_eq!(input["model"], "fixture-multimodal-model");
                        posts.fetch_add(1, Ordering::Relaxed);
                        if input["prompt"] == "reject" {
                            return (StatusCode::BAD_REQUEST, SECRET).into_response();
                        }
                        Json(json!({"data": [{"b64_json": encoded}]})).into_response()
                    }
                }
            }))
            .route("/v1/videos", post({
                let posts = video_posts.clone();
                move || {
                    posts.fetch_add(1, Ordering::Relaxed);
                    async { Json(json!({"id": "video_123", "status": "queued"})) }
                }
            }))
            .route("/v1/videos/{id}", get({
                let polled = video_polled.clone();
                move || {
                    polled.notify_one();
                    async { Json(json!({"id": "video_123", "status": "in_progress", "progress": 30})) }
                }
            }).delete({
                let canceled = video_canceled.clone();
                move || {
                    canceled.notify_one();
                    async { StatusCode::NO_CONTENT }
                }
            })),
        ).await;

        let unique = MediaJobId::new_v4().simple().to_string();
        let platform = format!("media-{unique}");
        let provider = provider.map_or_else(|| format!("media-fixture-{unique}"), str::to_owned);
        let owner = TenantScope::new(&platform, "owner").unwrap();
        let other = TenantScope::new(&platform, "other").unwrap();
        let workspace = tempfile::tempdir().unwrap();
        let (project, _) = crate::project::from_directory(&owner, workspace.path()).await.unwrap();
        let key_env = format!("SHIJU_MEDIA_FIXTURE_{unique}");
        agent_env::set(&key_env, SECRET);
        let credential = Credential(key_env.clone());
        let base = format!("{}/v1", upstream.base);
        crate::config::update_global_for_tenant(
            &owner,
            json!({
                "gateway": {"auth_enabled": true, "skeys": [
                    {"skey": TOKEN, "name": "fixture"},
                    {"skey": OTHER_TOKEN, "name": "second fixture credential"}
                ]},
                "providers": {
                    (provider): {
                        "api": base, "adapter": "openai-compatible", "env": [key_env],
                        "models": {"fixture-multimodal-model": {"status": "active", "options": {
                            "media_endpoint_types": ["image-generation", "openai-video"],
                            "media_video_cancel": true
                        }}}
                    }
                }
            }),
        )
        .await
        .unwrap();
        crate::config::update_global_for_tenant(&other, json!({
            "gateway": {"auth_enabled": true, "skeys": [{"skey": OTHER_TOKEN, "name": "fixture"}]}
        })).await.unwrap();
        let info = serde_json::from_value(json!({
            "id": SESSION, "slug": "fixture", "projectID": project.id,
            "directory": project.worktree, "title": "fixture", "version": "1",
            "time": {"created": 1, "updated": 1}
        }))
        .unwrap();
        crate::session::ui_store::save_agent_session_scoped_for_tenant(
            &owner,
            &info,
            Some(&project.id),
        )
        .unwrap();
        let gateway =
            Server::start(Router::new().nest("/api", crate::gateway::api::media_router())).await;
        let client = reqwest::Client::builder().timeout(Duration::from_secs(15)).build().unwrap();
        let models = client
            .get(format!("{}{MEDIA_MODELS_PATH}", gateway.base))
            .bearer_auth(TOKEN)
            .header("x-platform-id", &platform)
            .header("x-company-id", "owner")
            .query(&[("session_id", SESSION)])
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap()
            .json::<MediaModels>()
            .await
            .unwrap();
        let project_models = client
            .get(format!("{}{MEDIA_MODELS_PATH}", gateway.base))
            .bearer_auth(TOKEN)
            .header("x-platform-id", &platform)
            .header("x-company-id", "owner")
            .query(&[("project_id", project.id.as_ref())])
            .send().await.unwrap().error_for_status().unwrap().json::<MediaModels>().await.unwrap();
        assert_eq!(project_models, models);
        let global_models = client
            .get(format!("{}{MEDIA_MODELS_PATH}", gateway.base))
            .bearer_auth(TOKEN)
            .header("x-platform-id", &platform)
            .header("x-company-id", "owner")
            .query(&[("global", true)])
            .send().await.unwrap().error_for_status().unwrap().json::<MediaModels>().await.unwrap();
        assert_eq!(global_models, models);
        let model = models
            .models
            .into_iter()
            .find(|model| model.model_id == "fixture-multimodal-model")
            .unwrap();
        Self {
            gateway,
            _upstream: upstream,
            workspace,
            _credential: credential,
            platform,
            client,
            model,
            image,
            image_posts,
            video_posts,
            video_polled,
            video_canceled,
            _serial: serial,
        }
    }

    fn request(
        &self,
        method: Method,
        path: &str,
        company: &str,
        token: Option<&str>,
    ) -> reqwest::RequestBuilder {
        let request = self
            .client
            .request(method, format!("{}{path}", self.gateway.base))
            .header("x-platform-id", &self.platform)
            .header("x-company-id", company);
        match token {
            Some(token) => request.bearer_auth(token),
            None => request,
        }
    }

    pub(crate) fn workspace(&self) -> &std::path::Path {
        self.workspace.path()
    }

    pub(crate) fn image_posts(&self) -> usize {
        self.image_posts.load(Ordering::Relaxed)
    }

    pub(crate) async fn tool_context(&self, authenticated: bool) -> Option<Arc<crate::tools::ToolUseContext>> {
        self.context_for_token(authenticated.then_some(TOKEN)).await
    }

    pub(crate) async fn other_tool_context(&self) -> Arc<crate::tools::ToolUseContext> {
        self.context_for_token(Some(OTHER_TOKEN)).await.unwrap()
    }

    async fn context_for_token(&self, token: Option<&str>) -> Option<Arc<crate::tools::ToolUseContext>> {
        use crate::gateway::api::handlers::session::stream::Host;
        use gateway_session::stream_host::StreamHost as _;

        let tenant = TenantScope::new(&self.platform, "owner").unwrap();
        let mut headers = HeaderMap::new();
        if let Some(token) = token {
            headers.insert(header::AUTHORIZATION, format!("Bearer {token}").parse().unwrap());
        }
        crate::session::ui_store::with_tenant_scope(tenant.clone(), async {
            Host::authenticated_tool_context(&tenant, Some(SESSION), &headers).await.0
        })
        .await
    }

    pub(crate) fn submission(&self, operation: MediaOperation, text: &str) -> MediaSubmit {
        MediaSubmit {
            request_id: MediaJobId::new_v4(),
            session_id: SESSION.into(),
            operation,
            deployment_id: self.model.deployment_id.clone(),
            model_id: self.model.model_id.clone(),
            text: Some(text.into()),
            inputs: Vec::new(),
        }
    }

    pub(crate) async fn submit(&self, request: &MediaSubmit) -> reqwest::Response {
        self.request(Method::POST, MEDIA_JOBS_PATH, "owner", Some(TOKEN))
            .json(request)
            .send()
            .await
            .unwrap()
    }

    pub(crate) async fn wait(&self, job_id: MediaJobId) -> MediaJobState {
        tokio::time::timeout(Duration::from_secs(8), async {
            loop {
                let job: MediaJob = self
                    .request(
                        Method::GET,
                        &format!("{MEDIA_JOBS_PATH}/{job_id}"),
                        "owner",
                        Some(TOKEN),
                    )
                    .query(&[("session_id", SESSION)])
                    .send()
                    .await
                    .unwrap()
                    .error_for_status()
                    .unwrap()
                    .json()
                    .await
                    .unwrap();
                if !matches!(job.state, MediaJobState::Submitted | MediaJobState::Running { .. }) {
                    return job.state;
                }
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        })
        .await
        .unwrap()
    }
}

#[tokio::test]
async fn media_gateway_flow_authorizes_and_publishes_once() {
    let fixture = Fixture::new().await;
    let mut request = fixture.submission(MediaOperation::ImageGenerate, "draw");
    let unauthed = fixture
        .request(Method::POST, MEDIA_JOBS_PATH, "owner", None)
        .json(&request)
        .send()
        .await
        .unwrap();
    assert_eq!(unauthed.status(), StatusCode::UNAUTHORIZED);
    let cross_tenant = fixture
        .request(Method::POST, MEDIA_JOBS_PATH, "other", Some(TOKEN))
        .json(&request)
        .send()
        .await
        .unwrap();
    assert_eq!(cross_tenant.status(), StatusCode::UNAUTHORIZED);

    let model_id = std::mem::replace(&mut request.model_id, "unentitled".into());
    assert_eq!(fixture.submit(&request).await.status(), StatusCode::FORBIDDEN);
    request.model_id = model_id;
    let (first, second) = tokio::join!(fixture.submit(&request), fixture.submit(&request));
    let mut statuses = [first.status().as_u16(), second.status().as_u16()];
    statuses.sort_unstable();
    assert_eq!(statuses, [200, 202]);

    let MediaJobState::Succeeded { result } = fixture.wait(request.request_id).await else {
        panic!("expected a published image");
    };
    assert_eq!(fixture.image_posts.load(Ordering::Relaxed), 1);
    let [asset] = &result.assets[..] else { panic!("expected one asset") };
    assert_eq!(asset.mime, "image/png");
    assert_eq!(asset.width, Some(2));
    assert_eq!(asset.height, Some(3));
    let path = format!("{MEDIA_JOBS_PATH}/{}/assets/{}", request.request_id, asset.id);
    let download = fixture
        .request(Method::GET, &path, "owner", Some(TOKEN))
        .query(&[("session_id", SESSION)])
        .send()
        .await
        .unwrap();
    assert_eq!(download.status(), StatusCode::OK);
    assert_eq!(download.bytes().await.unwrap().as_ref(), fixture.image.as_slice());
    let foreign = fixture
        .request(Method::GET, &path, "other", Some(OTHER_TOKEN))
        .query(&[("session_id", SESSION)])
        .send()
        .await
        .unwrap();
    assert_eq!(foreign.status(), StatusCode::NOT_FOUND);

    let rejection = fixture.submission(MediaOperation::ImageGenerate, "reject");
    assert_eq!(fixture.submit(&rejection).await.status(), StatusCode::ACCEPTED);
    assert_eq!(
        fixture.wait(rejection.request_id).await,
        MediaJobState::Failed { code: Code::UpstreamFailed }
    );
}

#[tokio::test]
async fn media_gateway_flow_cancels_a_known_video_job() {
    let fixture = Fixture::new().await;
    let request = fixture.submission(MediaOperation::VideoGenerate, "animate");
    assert_eq!(fixture.submit(&request).await.status(), StatusCode::ACCEPTED);
    tokio::time::timeout(Duration::from_secs(8), fixture.video_polled.notified()).await.unwrap();
    let job: MediaJob = fixture
        .request(
            Method::DELETE,
            &format!("{MEDIA_JOBS_PATH}/{}", request.request_id),
            "owner",
            Some(TOKEN),
        )
        .query(&[("session_id", SESSION)])
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(job.state, MediaJobState::Canceled);
    tokio::time::timeout(Duration::from_secs(8), fixture.video_canceled.notified()).await.unwrap();
    assert_eq!(fixture.video_posts.load(Ordering::Relaxed), 1);
    assert_eq!(fixture.wait(request.request_id).await, MediaJobState::Canceled);
}
