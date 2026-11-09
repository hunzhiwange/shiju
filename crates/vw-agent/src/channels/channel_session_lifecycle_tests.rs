
#[cfg(test)]
#[tokio::test]
async fn channel_session_lifecycle_new_preserves_old() { runtime_command::session_commands_tests::channel_session_lifecycle_new_preserves_old().await; }
#[cfg(test)]
#[test]
fn channel_session_lifecycle_lists_are_parsed() { runtime_command::session_commands_tests::channel_session_lifecycle_lists_are_parsed(); }
#[cfg(test)]
#[test]
fn channel_session_lifecycle_mutations_are_parsed() { runtime_command::session_commands_tests::channel_session_lifecycle_mutations_are_parsed(); }
#[cfg(test)]
#[test]
fn channel_session_lifecycle_clear_is_separate() { runtime_command::session_commands_tests::channel_session_lifecycle_clear_is_separate(); }
#[cfg(test)]
#[tokio::test]
async fn channel_session_lifecycle_new_clears_cache() { runtime_command::session_commands_tests::channel_session_lifecycle_new_clears_cache().await; }
#[cfg(test)]
#[test]
fn channel_session_lifecycle_help_explains_clear() { runtime_command::session_commands_tests::channel_session_lifecycle_help_explains_clear(); }

#[tokio::test]
async fn channel_session_lifecycle_resume_reloads_persisted_state() { runtime_command::session_commands_tests::channel_session_lifecycle_resume_reloads_persisted_state().await; }

#[tokio::test]
async fn channel_session_lifecycle_resume_rejects_unowned_and_invalid_ids() { runtime_command::session_commands_tests::channel_session_lifecycle_resume_rejects_unowned_and_invalid_ids().await; }

#[tokio::test]
async fn channel_session_lifecycle_resume_isolates_all_chat_dimensions() { runtime_command::session_commands_tests::channel_session_lifecycle_resume_isolates_all_chat_dimensions().await; }

#[tokio::test]
async fn channel_session_lifecycle_archive_requires_matching_confirmation_and_preserves_history() { runtime_command::session_commands_tests::channel_session_lifecycle_archive_requires_matching_confirmation_and_preserves_history().await; }

#[tokio::test]
async fn channel_session_lifecycle_archive_rejects_stale_and_busy_state() { runtime_command::session_commands_tests::channel_session_lifecycle_archive_rejects_stale_and_busy_state().await; }

#[tokio::test]
async fn channel_session_lifecycle_continue_and_rename_use_owned_sessions() { runtime_command::session_commands_tests::channel_session_lifecycle_continue_and_rename_use_owned_sessions().await; }

#[tokio::test]
async fn channel_session_lifecycle_pipeline_persists_and_refreshes_model_options() { runtime_command::session_commands_tests::channel_session_lifecycle_pipeline_persists_and_refreshes_model_options().await; }

#[tokio::test]
async fn channel_session_lifecycle_resume_isolates_projects_and_tenants() { runtime_command::session_commands_tests::channel_session_lifecycle_resume_isolates_projects_and_tenants().await; }

#[tokio::test]
async fn channel_session_lifecycle_failed_creation_and_rename_preserve_binding() { runtime_command::session_commands_tests::channel_session_lifecycle_failed_creation_and_rename_preserve_binding().await; }

#[tokio::test]
async fn channel_session_lifecycle_invalid_saved_model_preserves_current_state() { runtime_command::session_commands_tests::channel_session_lifecycle_invalid_saved_model_preserves_current_state().await; }

#[tokio::test]
async fn channel_session_lifecycle_lists_are_stable_bounded_and_owned() { runtime_command::session_commands_tests::channel_session_lifecycle_lists_are_stable_bounded_and_owned().await; }

#[tokio::test]
async fn channel_session_lifecycle_new_rejects_invalid_model_options() {
    runtime_command::session_commands_tests::channel_session_lifecycle_new_rejects_invalid_model_options().await;
}

#[tokio::test]
async fn channel_first_response_hides_ordinary_and_explicit_drafts() {
    use runtime_command::session_commands_tests::{lifecycle_dispatch, lifecycle_fixture, lifecycle_id};

    let (_temp, ctx, msg) = lifecycle_fixture().await;
    let snapshot = ctx.message_snapshot.as_ref().unwrap();
    let mut input = msg.clone();
    input.content = "first input before success".into();
    let ordinary = resolve_or_create_sender_session_id(snapshot, &input, None).await.unwrap();
    let info = crate::session::session::get_any(&ordinary).await.unwrap();
    assert_eq!(info.title, "", "initial input is not a formal title");
    assert!(info.is_first_response_pending());
    assert!(!lifecycle_dispatch(&ctx, &msg, "/sessions").await.contains(&ordinary));
    assert!(lifecycle_dispatch(&ctx, &msg, "/continue").await.contains("暂无可继续"));
    assert!(lifecycle_dispatch(&ctx, &msg, &format!("/resume {ordinary}")).await.contains("失败"));
    assert_eq!(lifecycle_id(&ctx), ordinary);
    assert!(lifecycle_dispatch(&ctx, &msg, "/new").await.contains("已创建"));
    let draft = lifecycle_id(&ctx);
    assert_ne!(draft, ordinary);
    assert!(crate::session::session::get_any(&draft).await.unwrap().title.is_empty());
    assert!(channel_session_ownership(snapshot, &draft).await.unwrap().route.is_some());
    assert!(!lifecycle_dispatch(&ctx, &msg, "/sessions").await.contains(&draft));
}

#[tokio::test]
async fn channel_first_response_reuses_failed_ingress_then_publishes_success() {
    use runtime_command::session_commands_tests::{lifecycle_dispatch, lifecycle_fixture, lifecycle_id};

    let (_temp, ctx, mut msg) = lifecycle_fixture().await;
    let snapshot = ctx.message_snapshot.as_ref().unwrap();
    msg.id = "failed".into();
    let failure = lifecycle_dispatch(&ctx, &msg, "this requires the unavailable model").await;
    assert!(failure.contains("Error:"), "{failure}");
    let id = lifecycle_id(&ctx);
    assert_eq!(crate::session::session::get_any(&id).await.unwrap().title, "");
    assert!(!lifecycle_dispatch(&ctx, &msg, "/sessions").await.contains(&id));

    std::fs::create_dir(snapshot.directory.join("docs")).unwrap();
    std::fs::write(snapshot.directory.join("docs/guide.md"), "# Test guide").unwrap();
    msg.id = "successful".into();
    let prompt = "请读取 docs 文件列表";
    let reply = lifecycle_dispatch(&ctx, &msg, prompt).await;
    assert!(reply.contains("docs/guide.md"), "{reply}");
    assert_eq!(lifecycle_id(&ctx), id);
    let listed = lifecycle_dispatch(&ctx, &msg, "/sessions").await;
    assert!(listed.contains(&format!("{id}  {prompt}")), "{listed}");
    assert!(lifecycle_dispatch(&ctx, &msg, "/continue").await.contains(&id));
    let history = crate::session::message::messages(&id, None).await.unwrap();
    assert_eq!(history.len(), 2, "only the successful turn was committed");
}

#[tokio::test]
async fn channel_first_response_requests_title_once_after_commit() {
    use axum::{Json, Router, routing::{get, post}};
    use runtime_command::session_commands_tests::{lifecycle_dispatch, lifecycle_fixture, lifecycle_id};
    use serde_json::{Value, json};

    let (_temp, ctx, mut msg) = lifecycle_fixture().await;
    let snapshot = ctx.message_snapshot.as_ref().unwrap();
    let requests = Arc::new(Mutex::new(Vec::<Value>::new()));
    let posted = requests.clone();
    let service = Router::new()
        .route("/v1/models", get(|| async { Json(json!({"data": [{"id": "default-model"}]})) }))
        .route("/v1/chat/completions", post(move |Json(body): Json<Value>| {
            let posted = posted.clone();
            async move {
                posted.lock().unwrap().push(body);
                ([("content-type", "text/event-stream")], concat!(
                    "data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"Fixture title\"}}]}\n\n",
                    "data: {\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n",
                    "data: [DONE]\n\n",
                ))
            }
        }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, service).await.unwrap() });
    const KEY: &str = "VW_CHANNEL_FIRST_RESPONSE_FIXTURE_KEY";
    struct RemoveKey;
    impl Drop for RemoveKey {
        fn drop(&mut self) { crate::env::remove(KEY); }
    }
    crate::env::set(KEY, "local-fixture");
    let _key = RemoveKey;
    crate::config::update_project_for_tenant(
        &snapshot.scope.chat.tenant,
        &snapshot.directory,
        json!({"providers": {"default-provider": {
            "api": format!("http://{address}/v1"),
            "env": [KEY],
            "models": {"default-model": {"status": "active"}}
        }}}),
    )
    .await
    .unwrap();
    let prompt = "请读取 docs 文件列表";
    assert!(lifecycle_dispatch(&ctx, &msg, prompt).await.contains("Error:"));
    assert!(requests.lock().unwrap().is_empty());
    let id = lifecycle_id(&ctx);
    std::fs::create_dir(snapshot.directory.join("docs")).unwrap();
    std::fs::write(snapshot.directory.join("docs/guide.md"), "# Guide").unwrap();
    assert!(lifecycle_dispatch(&ctx, &msg, prompt).await.contains("docs/guide.md"));
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if crate::session::session::get_any(&id).await.unwrap().title == "Fixture title" {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }).await.expect("isolated model title should be saved");
    assert!(lifecycle_dispatch(&ctx, &msg, "/rename Explicit title").await.contains("已重命名"));
    msg.id = "second-turn".into();
    assert!(lifecycle_dispatch(&ctx, &msg, prompt).await.contains("docs/guide.md"));
    assert!(lifecycle_dispatch(&ctx, &msg, "/sessions").await.contains("Explicit title"));
    let captured = requests.lock().unwrap();
    assert_eq!(captured.len(), 1);
    assert!(captured[0]["messages"].to_string().contains(prompt));
    assert!(captured[0].get("tools").is_none_or(|tools| tools.as_array().is_some_and(Vec::is_empty)));
    server.abort();
}
