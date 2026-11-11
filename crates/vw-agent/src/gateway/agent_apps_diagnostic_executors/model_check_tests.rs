use std::{os::unix::fs::DirBuilderExt, time::Duration};

use serde_json::{Value, json};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    time::timeout,
};
use tokio_util::sync::CancellationToken;
use vw_agent_apps::auth::{
    copilot::CopilotAuth,
    credential_file::{AccountCredential, CredentialFileStore},
    flow::AuthAccountScope,
};
use vw_api_types::{
    TimestampMs,
    agent_apps::AppId,
    agent_apps_auth::{AuthErrorCode, AuthService},
    agent_apps_diagnostics::{
        DiagnosticEvidenceSource, DiagnosticFailureCode, DiagnosticOutcome, DiagnosticResult,
        DiagnosticScope,
    },
    app_providers::{
        AppProviderWrite, ProviderProtocol,
        claude::{ClaudeConfig, ClaudeProfile},
        codex_catalog::CodexProfile,
    },
    id::ProjectScope,
    model_catalog::{
        CodexNativeModel, ModelCatalogEdit, ModelCatalogEditRequest, ModelCatalogIdentity,
        SavedCodexCatalog,
    },
};

use super::{ModelPrepareError, ModelProbe};
use crate::{
    agent_apps::auth_accounts::{AuthAccountMetadata, AuthAccountStore},
    app_providers::AppProviderStore,
    gateway::agent_apps_diagnostic_jobs::{DiagnosticJobOwner, DiagnosticProbeError},
    global_database::GlobalDatabase,
    model_catalog::ModelCatalogStore,
};

#[tokio::test]
async fn diagnostic_model_check_validates_model_response() {
    use crate::model_relay::generation::GenerationProtocol;

    for (model, endpoint, expected) in [
        (
            "deepseek-flash",
            "https://api.deepseek.com/v1",
            "https://api.deepseek.com/anthropic/v1/messages",
        ),
        (
            "deepseek-v4.1-flash-1017",
            "https://token-plan.cn-beijing.maas.aliyuncs.com/compatible-mode/v1",
            "https://token-plan.cn-beijing.maas.aliyuncs.com/apps/anthropic/v1/messages",
        ),
    ] {
        assert_eq!(
            GenerationProtocol::Messages.url(endpoint, model, false).unwrap().as_str(),
            expected,
        );
    }
    let directory = tempfile::tempdir().unwrap();
    let database = GlobalDatabase::new(directory.path().join("global.sqlite")).unwrap();
    let providers = AppProviderStore::new(database.clone()).unwrap();
    let credentials = directory.path().join("credentials");
    std::fs::DirBuilder::new().mode(0o700).create(&credentials).unwrap();
    let accounts = AuthAccountStore::new(database.clone(), &credentials).unwrap();
    let copilot = CopilotAuth::new().unwrap();
    for (id, app, protocol, path, auth, valid) in [
        (
            "messages",
            AppId::Claude,
            ProviderProtocol::Anthropic,
            "/v1/messages",
            "x-api-key: fixture-key",
            json!({
                "type":"message","role":"assistant","model":"fixture-v1","content":[{"type":"text","text":"OK"}]
            }),
        ),
        (
            "responses",
            AppId::Codex,
            ProviderProtocol::OpenaiResponses,
            "/v1/responses",
            "authorization: bearer fixture-key",
            json!({
                "object":"response","status":"completed","model":"fixture-v1","output":[{"type":"message","role":"assistant","content":[{"type":"output_text","text":"OK"}]}]
            }),
        ),
        (
            "gemini",
            AppId::Gemini,
            ProviderProtocol::Gemini,
            "/v1/models/fixture:generateContent",
            "x-goog-api-key: fixture-key",
            json!({
                "modelVersion":"fixture-v1","candidates":[{"content":{"role":"model","parts":[{"text":"OK"}]}}]
            }),
        ),
        (
            "chat",
            AppId::Claude,
            ProviderProtocol::Anthropic,
            "/v1/chat/completions",
            "authorization: bearer fixture-key",
            json!({
                "object":"chat.completion","model":"fixture-v1","choices":[{"message":{"role":"assistant","content":"OK"}}]
            }),
        ),
    ] {
        let owner = DiagnosticJobOwner {
            scope: DiagnosticScope {
                project: ProjectScope::for_default_tenant("project".into()).unwrap(),
                target_host_id: "host".into(),
                app_id: app,
            },
            principal: "member".into(),
        };
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base_url = format!("http://{}/v1", listener.local_addr().unwrap());
        let tenant = owner.scope.project.tenant_scope();
        if id == "chat" {
            providers
                .claude_save_with_metadata(
                    tenant,
                    id,
                    &ClaudeProfile {
                        name: "Fixture".into(),
                        model: Some("fixture".into()),
                        config: ClaudeConfig::ThirdParty {
                            base_url,
                            api_key: "fixture-key".into(),
                            api_key_header: false,
                        },
                    },
                    None,
                    true,
                    Some(json!({"meta":{"apiFormat":"openai_chat"}})),
                )
                .unwrap();
        } else if id == "responses" {
            let deployment = ModelCatalogIdentity {
                target_host_id: owner.scope.target_host_id.clone(),
                app_id: app,
                provider_id: id.into(),
                endpoint: base_url.clone(),
            };
            let catalog = SavedCodexCatalog {
                models: vec![CodexNativeModel {
                    slug: "fixture".into(),
                    display_name: "Fixture".into(),
                    description: String::new(),
                    base_instructions: String::new(),
                    supports_reasoning_summaries: false,
                    supports_parallel_tool_calls: false,
                    context_window: None,
                    supported_reasoning_levels: Vec::new(),
                }],
            };
            let revision = ModelCatalogStore::new(database.clone())
                .unwrap()
                .save(tenant, &deployment, Some(&catalog))
                .unwrap();
            providers.codex_save(tenant, id, &CodexProfile {
                name: "Fixture".into(),
                config: format!(
                    "model = \"configured-other\"\nmodel_provider = \"custom\"\n[model_providers.custom]\nname = \"Fixture\"\nbase_url = \"{base_url}\"\nwire_api = \"responses\"\n"
                ),
                api_key: Some("fixture-key".into()),
                preserve_login: false,
            }, None, revision.as_deref()).unwrap();
        } else {
            providers
                .save(
                    tenant,
                    app,
                    id,
                    &AppProviderWrite {
                        name: "Fixture".into(),
                        protocol,
                        api_key: "fixture-key".into(),
                        base_url,
                        model: "fixture".into(),
                        request_options: None,
                    },
                )
                .unwrap();
        }
        let server = tokio::spawn(async move {
            for (status, response) in [
                (200, valid),
                (401, json!({})),
                (404, json!({})),
                (200, json!({"model":"fixture-v1"})),
            ] {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut headers = Vec::new();
                while !headers.ends_with(b"\r\n\r\n") {
                    headers.push(socket.read_u8().await.unwrap());
                    assert!(headers.len() < 8192);
                }
                let headers = String::from_utf8(headers).unwrap().to_ascii_lowercase();
                assert!(
                    headers
                        .starts_with(&format!("post {} http/1.1\r\n", path.to_ascii_lowercase()))
                );
                assert!(headers.lines().any(|line| line == auth));
                let len = headers
                    .lines()
                    .find_map(|line| line.strip_prefix("content-length: "))
                    .unwrap()
                    .parse()
                    .unwrap();
                let mut bytes = vec![0; len];
                socket.read_exact(&mut bytes).await.unwrap();
                let request: Value = serde_json::from_slice(&bytes).unwrap();
                assert!(request.get("tools").is_none());
                assert!(request.get("stream").is_none_or(|stream| stream == false));
                assert!(request.to_string().contains("Reply with OK."));
                let response = response.to_string();
                socket.write_all(format!("HTTP/1.1 {status} Fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response}", response.len()).as_bytes()).await.unwrap();
            }
            assert!(timeout(Duration::from_millis(50), listener.accept()).await.is_err());
        });
        for (status, error) in [
            (200, None),
            (401, Some(DiagnosticFailureCode::Unauthorized)),
            (404, Some(DiagnosticFailureCode::UnexpectedHttpStatus)),
            (200, Some(DiagnosticFailureCode::InvalidResponse)),
        ] {
            let probe = ModelProbe::prepare(
                &database,
                &accounts,
                owner.clone(),
                id.into(),
                "fixture".into(),
            )
            .unwrap();
            let result = probe
                .run_with_source(
                    &accounts,
                    &copilot,
                    CancellationToken::new(),
                    DiagnosticEvidenceSource::ControlledFixture,
                )
                .await
                .ok();
            let Some(DiagnosticResult::Model { report, may_incur_cost: true, .. }) = result else {
                panic!("authorized model must produce a model report for HTTP {status}");
            };
            assert_eq!(report.source, DiagnosticEvidenceSource::ControlledFixture);
            match (report.outcome, error) {
                (DiagnosticOutcome::Succeeded { evidence }, None) => {
                    assert_eq!(evidence.http_status, status);
                    assert_eq!(evidence.actual_model_id.as_ref(), "fixture-v1");
                }
                (DiagnosticOutcome::Failed { error, http_status }, Some(expected)) => {
                    assert_eq!(http_status, Some(status));
                    assert_eq!(error.code, expected);
                }
                (actual, expected) => panic!(
                    "unexpected outcome {actual:?} for HTTP {status}, expected error {expected:?}"
                ),
            }
        }
        assert!(
            ModelProbe::prepare(&database, &accounts, owner.clone(), id.into(), "other".into())
                .is_err()
        );
        let prepared = || {
            ModelProbe::prepare(&database, &accounts, owner.clone(), id.into(), "fixture".into())
                .unwrap()
        };
        let cancelled = CancellationToken::new();
        cancelled.cancel();
        assert!(matches!(
            prepared().run(&accounts, &copilot, cancelled).await,
            Err(DiagnosticProbeError::Cancelled)
        ));
        let stale = prepared();
        providers
            .set_presentation(
                tenant,
                app,
                id,
                &vw_api_types::app_providers::presentation::AppProviderPresentation {
                    notes: Some("configuration changed after preparation".into()),
                    ..Default::default()
                },
            )
            .unwrap();
        let Some(DiagnosticResult::Model { report, .. }) =
            stale.run(&accounts, &copilot, CancellationToken::new()).await.ok()
        else {
            panic!("stale provider must produce a model report");
        };
        assert!(
            matches!(report.outcome, DiagnosticOutcome::Failed { error, http_status: None } if error.code == DiagnosticFailureCode::Unauthorized)
        );
        server.await.unwrap();
    }

    let owner = DiagnosticJobOwner {
        scope: DiagnosticScope {
            project: ProjectScope::for_default_tenant("project".into()).unwrap(),
            target_host_id: "host".into(),
            app_id: AppId::Claude,
        },
        principal: "member".into(),
    };
    let tenant = owner.scope.project.tenant_scope();
    let endpoint = "https://api.githubcopilot.com";
    providers
        .claude_save_with_metadata(
            tenant,
            "copilot",
            &ClaudeProfile {
                name: "Managed fixture".into(),
                model: None,
                config: ClaudeConfig::ThirdParty {
                    base_url: endpoint.into(),
                    api_key: "unused-fixture-key".into(),
                    api_key_header: false,
                },
            },
            None,
            true,
            Some(json!({"meta":{
                "providerType":"github_copilot",
                "apiFormat":"openai_chat",
                "githubAccountId":"bound"
            }})),
        )
        .unwrap();
    let deployment = ModelCatalogIdentity {
        target_host_id: owner.scope.target_host_id.clone(),
        app_id: owner.scope.app_id,
        provider_id: "copilot".into(),
        endpoint: endpoint.into(),
    };
    let catalog = providers.read_model_catalog(tenant, &deployment).unwrap();
    providers
        .edit_model_catalog(
            tenant,
            &ModelCatalogEditRequest {
                deployment,
                expected_revision: catalog.revision,
                edits: vec![ModelCatalogEdit::Upsert {
                    model_id: "fixture".into(),
                    display_alias: None,
                    metadata: Box::default(),
                }],
                default_change: None,
            },
        )
        .unwrap();
    let scope = AuthAccountScope {
        tenant,
        principal: &owner.principal,
        host: &owner.scope.target_host_id,
        service: AuthService::GithubCopilot,
    };
    let mut metadata = AuthAccountMetadata::new("identity", TimestampMs(1)).unwrap();
    metadata.github_domain = Some("github.com".into());
    let store = CredentialFileStore::new(&credentials).unwrap();
    let write_credential = |id, value: &str| {
        store
            .lease_copilot(scope, id, true)
            .unwrap()
            .save(&AccountCredential::CopilotToken(value.into()))
            .unwrap();
    };
    accounts.insert_metadata(&scope, "different", &metadata).unwrap();
    write_credential("different", "unrelated-token");
    let prepare = |owner| {
        ModelProbe::prepare(&database, &accounts, owner, "copilot".into(), "fixture".into())
    };
    assert!(matches!(
        prepare(owner.clone()),
        Err(ModelPrepareError::Account(
            AuthErrorCode::NotFound | AuthErrorCode::StorageUnavailable
        ))
    ));
    accounts.insert_metadata(&scope, "bound", &metadata).unwrap();
    write_credential("bound", "original-token");
    let mut wrong_host = owner.clone();
    wrong_host.scope.target_host_id = "another-host".into();
    assert!(prepare(wrong_host).is_err());
    let stale = prepare(owner.clone()).unwrap();
    write_credential("bound", "replacement-token");
    assert_eq!(stale.target.check_current(&accounts), Err(DiagnosticFailureCode::Unauthorized));
}
