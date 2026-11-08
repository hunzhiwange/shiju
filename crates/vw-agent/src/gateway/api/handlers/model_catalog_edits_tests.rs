use super::*;

const MODEL: &str = "vendor/One:free";

async fn read(f: &Fixture) -> ModelCatalogReadResponse {
    f.client().model_catalog(&f.project, &f.query, TOKEN).await.unwrap()
}

fn alias(model: &str, value: &str) -> ModelCatalogEdit {
    ModelCatalogEdit::Upsert {
        model_id: model.into(),
        display_alias: Some(value.into()),
        metadata: Box::default(),
    }
}

fn edit(f: &Fixture, revision: &str, edits: Vec<ModelCatalogEdit>) -> ModelCatalogEditRequest {
    ModelCatalogEditRequest {
        deployment: f.query.deployment.clone(),
        expected_revision: revision.into(),
        edits,
        default_change: None,
    }
}

#[tokio::test]
async fn catalog_api_edits_conflict_keeps_persisted_model() {
    let f = Fixture::new().await;
    let before = read(&f).await;
    let store = AppProviderStore::new(GlobalDatabase::new(&f.db_path).unwrap()).unwrap();
    let current = store
        .edit_model_catalog(
            &before.items[0].identity.scope,
            &edit(&f, &before.revision, vec![alias(MODEL, "新别名")]),
        )
        .unwrap();
    let stale = edit(&f, &before.revision, vec![alias(MODEL, "旧别名")]);
    assert_http(f.client().edit_model_catalog(&f.project, &stale, EDITOR_TOKEN).await, 409);
    let after = read(&f).await;
    assert_eq!(after.revision, current.revision);
    assert_eq!(after.items[0].display_alias.as_deref(), Some("新别名"));
}

#[tokio::test]
async fn catalog_api_edits_concurrent_edits_have_one_winner() {
    let f = Fixture::new().await;
    let before = read(&f).await;
    let first = edit(&f, &before.revision, vec![alias(MODEL, "first")]);
    let second = edit(&f, &before.revision, vec![alias(MODEL, "second")]);
    let client = f.client();
    let (first, second) = tokio::join!(
        client.edit_model_catalog(&f.project, &first, EDITOR_TOKEN),
        client.edit_model_catalog(&f.project, &second, EDITOR_TOKEN),
    );
    let (winner, label) = match first {
        Ok(first) => {
            assert_http(second, 409);
            (first, "first")
        }
        Err(first) => {
            assert_http(Err::<ModelCatalogWriteResponse, _>(first), 409);
            (second.unwrap(), "second")
        }
    };
    assert!(winner.changed);
    let after = read(&f).await;
    assert_eq!(after.revision, winner.revision);
    assert_eq!(after.items[0].display_alias.as_deref(), Some(label));
    assert_eq!(after.items[0].metadata, before.items[0].metadata);
}

#[tokio::test]
async fn catalog_api_edits_denies_missing_credentials_and_read_only_writes() {
    let f = Fixture::new().await;
    let before = read(&f).await;
    let request = edit(&f, &before.revision, vec![alias(MODEL, "forbidden")]);
    let client = f.client();
    assert_http(client.edit_model_catalog(&f.project, &request, "invalid").await, 401);
    assert_http(client.edit_model_catalog(&f.project, &request, TOKEN).await, 403);
    let defaults = ModelCatalogDefaultRequest {
        deployment: request.deployment,
        expected_revision: before.revision.clone(),
        change: ModelCatalogDefaultChange::Clear,
    };
    assert_http(client.set_model_catalog_default(&f.project, &defaults, TOKEN).await, 403);
    let commit = ModelCatalogFillCommitRequest {
        identity: before.items[0].identity.clone(),
        expected_revision: before.revision.clone(),
    };
    assert_http(client.commit_model_catalog_fill(&f.project, &commit, TOKEN).await, 403);
    assert_eq!(read(&f).await, before);
}

#[tokio::test]
async fn catalog_api_edits_denies_other_hosts_projects_and_tenants() {
    let f = Fixture::new().await;
    let before = read(&f).await;
    let mut request = edit(&f, &before.revision, vec![alias(MODEL, "forbidden")]);
    let client = f.client();
    let wrong_project =
        ModelCatalogProject { project_id: "other-project".into(), ..f.project.clone() };
    assert_http(client.edit_model_catalog(&wrong_project, &request, EDITOR_TOKEN).await, 403);
    let other_tenant = TenantScope::new("platform", "other-company").unwrap();
    let other_client =
        GatewayClient::new(f.endpoint.clone().with_tenant_scope(other_tenant.clone())).unwrap();
    assert_http(other_client.edit_model_catalog(&f.project, &request, EDITOR_TOKEN).await, 403);
    request.deployment.target_host_id = "other-host".into();
    assert_http(client.edit_model_catalog(&f.project, &request, EDITOR_TOKEN).await, 403);
    let mut identity = before.items[0].identity.clone();
    identity.scope = other_tenant;
    let preview = ModelCatalogFillPreviewRequest { identity };
    assert_http(client.preview_model_catalog_fill(&f.project, &preview, TOKEN).await, 403);
    assert_eq!(read(&f).await, before);
}

#[tokio::test]
async fn catalog_api_edits_persists_alias_and_default_across_gateway_reopen() {
    let mut f = Fixture::new().await;
    let before = read(&f).await;
    let client = f.client();
    let edit = edit(&f, &before.revision, vec![alias("vendor/Two:free", "  新展示  ")]);
    let changed = client.edit_model_catalog(&f.project, &edit, EDITOR_TOKEN).await.unwrap();
    assert!(changed.changed);
    let default = ModelCatalogDefaultRequest {
        deployment: edit.deployment,
        expected_revision: changed.revision,
        change: ModelCatalogDefaultChange::Set { model_id: "vendor/Two:free".into() },
    };
    let changed =
        client.set_model_catalog_default(&f.project, &default, EDITOR_TOKEN).await.unwrap();
    assert!(changed.changed);

    f.server.abort();
    let _ = (&mut f.server).await;
    let db = GlobalDatabase::new(&f.db_path).unwrap();
    (f.server, f.endpoint) = Fixture::serve(
        db,
        f._root.path().join("access.json"),
        before.items[0].identity.scope.clone(),
    )
    .await;
    let after = read(&f).await;
    assert_eq!(after.revision, changed.revision);
    assert_eq!(after.default_model_id, Some("vendor/Two:free".into()));
    assert_eq!(after.items[1].display_alias.as_deref(), Some("新展示"));

    let clear = ModelCatalogDefaultRequest {
        change: ModelCatalogDefaultChange::Clear,
        expected_revision: after.revision,
        ..default
    };
    assert!(
        f.client()
            .set_model_catalog_default(&f.project, &clear, EDITOR_TOKEN)
            .await
            .unwrap()
            .changed
    );
    assert!(read(&f).await.default_model_id.is_none());
}

#[tokio::test]
async fn catalog_api_edits_rejects_default_from_another_deployment_atomically() {
    let mut f = Fixture::new().await;
    let original = read(&f).await;
    f.query.deployment.endpoint = "https://other.test/v1".into();
    let empty = read(&f).await;
    let mut edits = edit(&f, &empty.revision, vec![alias("new/model", "unused")]);
    edits.default_change = Some(ModelCatalogDefaultChange::Set { model_id: MODEL.into() });
    assert_http(f.client().edit_model_catalog(&f.project, &edits, EDITOR_TOKEN).await, 422);
    assert_eq!(read(&f).await, empty);
    f.query.deployment = original.items[0].identity.deployment.clone();
    assert_eq!(read(&f).await, original);
}

#[tokio::test]
async fn catalog_api_edits_fill_preview_is_read_only_and_commit_preserves_manual_values() {
    let mut f = Fixture::new().await;
    f.query.deployment.endpoint = "https://api.baiduqianfan.ai/v2/tokenplan/team".into();
    let client = f.client();
    let empty = read(&f).await;
    let add = edit(
        &f,
        &empty.revision,
        vec![ModelCatalogEdit::Upsert {
            model_id: "glm-5.2".into(),
            display_alias: Some("保留别名".into()),
            metadata: Box::new(ModelMetadataDto {
                reasoning: ModelCapabilityState::Unsupported,
                ..Default::default()
            }),
        }],
    );
    client.edit_model_catalog(&f.project, &add, EDITOR_TOKEN).await.unwrap();
    let before = read(&f).await;
    let request = ModelCatalogFillPreviewRequest { identity: before.items[0].identity.clone() };
    let preview = client.preview_model_catalog_fill(&f.project, &request, TOKEN).await.unwrap();
    assert_eq!(preview.identity, request.identity);
    assert_eq!(preview.revision, before.revision);
    assert_eq!(preview.proposed.context_window, Some(500_000));
    assert_eq!(
        preview.proposed.provenance[&ModelMetadataField::ContextWindow].source,
        ModelMetadataSource::Preset
    );
    assert_eq!(preview.proposed.reasoning, ModelCapabilityState::Unknown);
    assert_eq!(read(&f).await, before);

    let commit = ModelCatalogFillCommitRequest {
        identity: preview.identity,
        expected_revision: preview.revision,
    };
    let written =
        client.commit_model_catalog_fill(&f.project, &commit, EDITOR_TOKEN).await.unwrap();
    assert!(written.changed);
    let after = read(&f).await;
    assert_eq!(after.revision, written.revision);
    assert_eq!(after.items[0].metadata.context_window, preview.proposed.context_window);
    assert_eq!(after.items[0].metadata.reasoning, ModelCapabilityState::Unsupported);
    assert_eq!(after.items[0].display_alias, before.items[0].display_alias);
    let retry = client.commit_model_catalog_fill(&f.project, &commit, EDITOR_TOKEN).await.unwrap();
    assert_eq!(retry, ModelCatalogWriteResponse { revision: written.revision, changed: false });
}

#[tokio::test]
async fn catalog_api_edits_fill_rejects_stale_revision_and_other_endpoint() {
    let f = Fixture::new().await;
    let before = read(&f).await;
    let mut identity = before.items[0].identity.clone();
    let client = f.client();
    identity.deployment.endpoint = "https://other.test/v1".into();
    let preview = ModelCatalogFillPreviewRequest { identity };
    assert_http(client.preview_model_catalog_fill(&f.project, &preview, TOKEN).await, 404);
    let edit = edit(&f, &before.revision, vec![alias(MODEL, "")]);
    client.edit_model_catalog(&f.project, &edit, EDITOR_TOKEN).await.unwrap();
    let current = read(&f).await;
    assert!(current.items[0].display_alias.is_none());
    let stale = ModelCatalogFillCommitRequest {
        identity: before.items[0].identity.clone(),
        expected_revision: before.revision,
    };
    assert_http(client.commit_model_catalog_fill(&f.project, &stale, EDITOR_TOKEN).await, 409);
    assert_eq!(read(&f).await, current);
}
