use vw_api_types::{app_providers::agy::*, id::TenantScope};
#[test]
fn cc04_agy_native_card_is_saved_without_live_activation() {
    let root = tempfile::tempdir().unwrap();
    let db = crate::global_database::GlobalDatabase::for_home(root.path()).unwrap();
    let store = crate::app_providers::AppProviderStore::new(db).unwrap();
    let t = TenantScope::new("p", "a").unwrap();
    let p = AgyProfile {
        name: "Native login".into(),
        model: Some("Display Name".into()),
        api_key: None,
        base_url: None,
    };
    store.agy_save(&t, "one", &p).unwrap();
    assert_eq!(
        store.get_summary(&t, vw_api_types::agent_apps::AppId::Agy, "one").unwrap().name,
        "Native login"
    );
    assert!(!root.path().join(".gemini").exists());
    assert!(store.agy_profile(&TenantScope::new("p", "b").unwrap(), "one").is_err());
}

use axum::{Extension, Router, middleware};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{os::unix::fs::PermissionsExt, path::Path};
use vw_api_types::agent_apps::*;
use vw_gateway_client::{GatewayClient, GatewayEndpoint};
const TOKEN: &str = "fixture-agy-authorized-token-32-characters";
fn terminal(
    _: Option<&str>,
    _: &Path,
    m: &vw_agent_apps::apps::launch::request::LaunchMaterial,
) -> Result<(), String> {
    use std::io::Read;
    let mut stream =
        std::os::unix::net::UnixStream::connect(m.cwd.join("io")).map_err(|_| "attach failed")?;
    std::thread::spawn(move || {
        let mut b = Vec::new();
        let _ = stream.read_to_end(&mut b);
    });
    Ok(())
}
#[tokio::test]
async fn cc04_agy_sdk_api_launch_uses_persistent_profile_and_denies_before_writes() {
    let root = tempfile::tempdir().unwrap();
    let home = root.path();
    std::fs::set_permissions(home, std::fs::Permissions::from_mode(0o700)).unwrap();
    let private = home.join("private");
    std::fs::create_dir(&private).unwrap();
    std::fs::set_permissions(&private, std::fs::Permissions::from_mode(0o700)).unwrap();
    let bin = home.join("bin");
    std::fs::create_dir(&bin).unwrap();
    std::fs::create_dir_all(home.join(".gemini/config")).unwrap();
    std::fs::create_dir_all(home.join(".gemini/antigravity-cli")).unwrap();
    std::fs::write(home.join(".gemini/antigravity-cli/settings.json"), b"native-settings").unwrap();
    std::fs::write(home.join("login"), b"native-login").unwrap();
    let script = r#"#!/bin/sh
if [ "$1" = --version ]; then echo 'agy 1.0.0'; exit 0; fi
test -t 0 && test -t 1 || exit 30
test "$GEMINI_API_KEY" = fake-key || exit 31
case "$1" in --gemini_dir=*) dir="${1#--gemini_dir=}" ;; *) exit 32 ;; esac
test -f "$dir/antigravity-cli/settings.json" || exit 33
test -L "$dir/config" || exit 34
printf '%s' session > "$dir/session.json"
printf '%s' "$dir" > proof
"#;
    std::fs::write(bin.join("agy"), script).unwrap();
    std::fs::set_permissions(bin.join("agy"), std::fs::Permissions::from_mode(0o700)).unwrap();
    let policy = home.join("policy.json");
    let mut pv = json!({"principals":[{"id":"principal","token_sha256":hex::encode(Sha256::digest(TOKEN)),"grants":[{"platform_id":"p","company_id":"a","host_id":"host","provider_config_read":true,"provider_config_write":true,"launch":true,"identities":[{"appId":"agy","kind":"provider","id":"one"}]}]}],"hosts":[{"id":"host","online":true,"binding":"local","home":home,"credential_root":private,"search_paths":[bin],"applications":home.join("Applications")}]});
    std::fs::write(&policy, pv.to_string()).unwrap();
    std::fs::set_permissions(&policy, std::fs::Permissions::from_mode(0o600)).unwrap();
    let db = crate::global_database::GlobalDatabase::for_home(home).unwrap();
    let router = Router::new()
        .merge(super::router_with_terminal(Some(policy.clone()), terminal))
        .merge(crate::gateway::api::handlers::app_providers::agy::router_with_policy(Some(
            policy.clone(),
        )))
        .layer(Extension(db.clone()))
        .layer(middleware::from_fn(crate::gateway::middleware::tenant_scope_middleware));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = tokio::spawn(async move {
        axum::serve(listener, Router::new().nest("/v1", router)).await.unwrap()
    });
    let tenant = TenantScope::new("p", "a").unwrap();
    let client = GatewayClient::new(
        GatewayEndpoint::new("127.0.0.1", port).with_tenant_scope(tenant.clone()),
    )
    .unwrap();
    let request = AgyWrite {
        target_host_id: "host".into(),
        profile: AgyProfile {
            name: "Relay".into(),
            model: Some("Display Name".into()),
            api_key: Some("fake-key".into()),
            base_url: Some("https://example.com".into()),
        },
    };
    assert!(client.agy_provider_save("one", &request, "wrong-token").await.is_err());
    assert!(!private.join("agy-profiles").exists());
    let saved = client.agy_provider_save("one", &request, TOKEN).await.unwrap();
    assert!(saved.catalog_saved);
    assert!(!private.join("agy-profiles").exists());
    assert!(
        client
            .agy_provider_switch("one", &AgyTarget { target_host_id: "host".into() }, TOKEN)
            .await
            .is_err()
    );
    assert!(!private.join("agy-profiles").exists());
    let observed = vw_agent_apps::apps::discovery::read::read(
        AppId::Agy,
        &vw_agent_apps::apps::discovery::read::LocalReadEnvironment {
            host_id: "host".into(),
            home: home.into(),
            search_paths: vec![bin],
            applications: home.join("Applications"),
        },
    )
    .unwrap();
    assert!(!observed.installations.is_empty());
    let launch = LaunchRequest {
        app_id: AppId::Agy,
        installation_id: observed.installations[0].id.clone(),
        cwd: home.to_str().unwrap().into(),
        args: vec![],
        skip_permissions: false,
    };
    let identity = LaunchIdentity {
        kind: LaunchIdentityKind::Provider,
        id: "one".into(),
        label: "Relay".into(),
    };
    assert!(
        client
            .agent_apps_launch_identity_authorized("host", &launch, &identity, "wrong-token")
            .await
            .is_err()
    );
    assert!(!private.join("agy-profiles").exists());
    let mut wrong = identity.clone();
    wrong.id = "unknown".into();
    assert!(
        client.agent_apps_launch_identity_authorized("host", &launch, &wrong, TOKEN).await.is_err()
    );
    assert!(!private.join("agy-profiles").exists());
    let result = client
        .agent_apps_launch_identity_authorized("host", &launch, &identity, TOKEN)
        .await
        .unwrap();
    let session = result.session_id.unwrap();
    for _ in 0..200 {
        let status = client.agent_apps_launch_session("host", &session, TOKEN).await.unwrap();
        if matches!(status.phase, LaunchSessionPhase::Completed | LaunchSessionPhase::Failed) {
            assert_eq!(status.phase, LaunchSessionPhase::Completed);
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    let path = std::path::PathBuf::from(std::fs::read_to_string(home.join("proof")).unwrap());
    assert_eq!(std::fs::read(path.join("session.json")).unwrap(), b"session");
    let doc: Value =
        serde_json::from_slice(&std::fs::read(path.join("antigravity-cli/settings.json")).unwrap())
            .unwrap();
    assert_eq!(doc["modelProvider"], "gemini");
    assert_eq!(doc["model"], "Display Name");
    assert_eq!(std::fs::read_link(path.join("config")).unwrap(), home.join(".gemini/config"));
    assert_eq!(std::fs::read(home.join("login")).unwrap(), b"native-login");
    assert_eq!(
        std::fs::read(home.join(".gemini/antigravity-cli/settings.json")).unwrap(),
        b"native-settings"
    );

    // Real SDK -> HTTP -> tenant SQLite form editing; private settings change only on launch.
    use vw_api_types::app_providers::{basic::ProviderBasic, create::ProviderSecretWrite};
    let target = AgyTarget { target_host_id: "host".into() };
    let public = client.agy_form_read("one", &target, TOKEN).await.unwrap();
    let public_json = serde_json::to_string(&public).unwrap();
    assert!(!public_json.contains("fake-key"));
    let mut edit = AgyFormWrite {
        target_host_id: "host".into(),
        fields: ProviderBasic {
            name: "中文档案".into(),
            category: Some("工作".into()),
            notes: Some("保留会话".into()),
            website_url: None,
        },
        model: Some("新的中文模型".into()),
        base_url: public.base_url.clone(),
        api_key: ProviderSecretWrite::Keep,
        revision: Some(public.revision.clone()),
    };
    let original = std::fs::read(path.join("antigravity-cli/settings.json")).unwrap();
    let before = crate::app_providers::AppProviderStore::new(db.clone())
        .unwrap()
        .agy_record(&tenant, "one")
        .unwrap();
    edit.api_key = ProviderSecretWrite::Clear;
    assert!(client.agy_form_update("one", &edit, TOKEN).await.is_err());
    assert_eq!(
        crate::app_providers::AppProviderStore::new(db.clone())
            .unwrap()
            .agy_record(&tenant, "one")
            .unwrap(),
        before
    );
    edit.api_key = ProviderSecretWrite::Keep;
    edit.base_url = Some("https://other.example.test".into());
    assert!(client.agy_form_update("one", &edit, TOKEN).await.is_err());
    edit.base_url = public.base_url.clone();
    assert!(client.agy_form_update("one", &edit, TOKEN).await.unwrap().catalog_saved);
    assert_eq!(std::fs::read(path.join("antigravity-cli/settings.json")).unwrap(), original);
    assert!(
        client.agy_form_update("one", &edit, TOKEN).await.is_err(),
        "stale revision must not overwrite"
    );
    let public = client.agy_form_read("one", &target, TOKEN).await.unwrap();
    assert_eq!(public.model.as_deref(), Some("新的中文模型"));
    assert!(public.has_saved_key);
    assert_eq!(public.fields.notes.as_deref(), Some("保留会话"));
    // Keep the actual controlled process alive while exercising the deletion guard.
    std::fs::write(home.join("hold"), b"hold").unwrap();
    let script = script.replace(
        "printf '%s' session",
        "while test -f hold; do sleep 0.02; done\nprintf '%s' session",
    );
    std::fs::write(home.join("bin/agy"), script).unwrap();
    let second = client
        .agent_apps_launch_identity_authorized("host", &launch, &identity, TOKEN)
        .await
        .unwrap()
        .session_id
        .unwrap();
    assert!(
        client.agy_form_delete("one", &target, TOKEN).await.is_err(),
        "active catalog lease blocks deletion"
    );
    std::fs::remove_file(home.join("hold")).unwrap();
    let mut completed = false;
    for _ in 0..200 {
        let status = client.agent_apps_launch_session("host", &second, TOKEN).await.unwrap();
        if status.phase == LaunchSessionPhase::Completed {
            completed = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert!(completed);
    let changed: Value =
        serde_json::from_slice(&std::fs::read(path.join("antigravity-cli/settings.json")).unwrap())
            .unwrap();
    assert_eq!(changed["model"], "新的中文模型");
    assert_eq!(std::fs::read(path.join("session.json")).unwrap(), b"session");
    // Delete only the catalog row; retained profile/session/login remain unchanged.
    assert!(client.agy_form_delete("one", &target, TOKEN).await.unwrap().deleted);
    assert!(client.agy_form_read("one", &target, TOKEN).await.is_err());
    assert!(path.join("session.json").is_file());
    assert_eq!(std::fs::read(home.join("login")).unwrap(), b"native-login");
    edit.revision = None;
    edit.base_url = None;
    edit.api_key = ProviderSecretWrite::Clear;
    assert!(client.agy_form_create("native", &edit, TOKEN).await.unwrap().catalog_saved);
    let native = client.agy_form_read("native", &target, TOKEN).await.unwrap();
    assert!(!native.has_saved_key);
    assert!(client.agy_form_delete("native", &target, "wrong-token").await.is_err());
    assert!(client.agy_form_delete("native", &target, TOKEN).await.unwrap().deleted);
    assert!(client.agy_form_create("read-only", &edit, TOKEN).await.unwrap().catalog_saved);
    pv["principals"][0]["grants"][0]["provider_config_write"] = json!(false);
    std::fs::write(&policy, pv.to_string()).unwrap();
    assert!(client.agy_form_read("read-only", &target, TOKEN).await.is_ok());
    assert!(client.agy_form_delete("read-only", &target, TOKEN).await.is_err());
    assert!(client.agy_provider_save("two", &request, TOKEN).await.is_err());
    assert!(client.agy_form_create("two", &edit, TOKEN).await.is_err());
    assert!(client.agy_form_delete("two", &target, TOKEN).await.is_err());
    assert!(
        crate::app_providers::AppProviderStore::new(db)
            .unwrap()
            .agy_record(&tenant, "two")
            .unwrap()
            .is_none()
    );
    server.abort();
}

#[test]
fn cc04_ui_agy_editor_preserves_metadata_and_latest_secret() {
    use vw_api_types::app_providers::create::ProviderSecretWrite;
    let root = tempfile::tempdir().unwrap();
    let db = crate::global_database::GlobalDatabase::for_home(root.path()).unwrap();
    let store = crate::app_providers::AppProviderStore::new(db.clone()).unwrap();
    let tenant = TenantScope::new("p", "a").unwrap();
    let settings = crate::gateway::settings_store::GatewaySettingsStore::new(db).unwrap();
    let profile = AgyProfile {
        name: "Fixture".into(),
        model: Some("旧模型".into()),
        api_key: Some("fake-key".into()),
        base_url: Some("https://example.test".into()),
    };
    store.agy_save(&tenant, "one", &profile).unwrap();
    let mut row = store.agy_record(&tenant, "one").unwrap().unwrap();
    row["custom"] = json!({"extension":"preserved"});
    row["settingsConfig"]["config"]["unknown"] = json!("preserved");
    row["settingsConfig"]["env"]["UNRELATED"] = json!("preserved");
    settings.upsert(&tenant, "app_providers.records", "\"agy\":one", &row.to_string()).unwrap();
    let view = store.agy_form_read(&tenant, "one").unwrap();
    let body = AgyFormWrite {
        target_host_id: "host".into(),
        fields: view.fields.clone(),
        model: Some("新模型".into()),
        base_url: view.base_url.clone(),
        api_key: ProviderSecretWrite::Keep,
        revision: Some(view.revision.clone()),
    };
    row["agy"]["apiKey"] = json!("fake-rotated-key");
    settings.upsert(&tenant, "app_providers.records", "\"agy\":one", &row.to_string()).unwrap();
    assert_eq!(
        store.agy_form_read(&tenant, "one").unwrap().revision,
        view.revision,
        "public revision must not fingerprint secrets"
    );
    store.agy_form_write(&tenant, "one", &body, false).unwrap();
    let saved = store.agy_record(&tenant, "one").unwrap().unwrap();
    assert_eq!(saved["agy"]["apiKey"], "fake-rotated-key");
    assert_eq!(saved["custom"], row["custom"]);
    assert_eq!(saved["settingsConfig"]["config"]["unknown"], "preserved");
    assert_eq!(saved["settingsConfig"]["env"]["UNRELATED"], "preserved");
    let public = store.agy_form_read(&tenant, "one").unwrap();
    assert!(!serde_json::to_string(&public).unwrap().contains("fake-rotated-key"));
    let mut replace = AgyFormWrite {
        revision: Some(public.revision),
        api_key: ProviderSecretWrite::Replace { value: "FAKE-REPLACEMENT".into() },
        ..body
    };
    store.agy_form_write(&tenant, "one", &replace, false).unwrap();
    assert_eq!(
        store.agy_profile(&tenant, "one").unwrap().api_key.as_deref(),
        Some("FAKE-REPLACEMENT")
    );
    replace.revision = Some(store.agy_form_read(&tenant, "one").unwrap().revision);
    replace.api_key = ProviderSecretWrite::Replace { value: "[REDACTED]".into() };
    assert!(store.agy_form_write(&tenant, "one", &replace, false).is_err());
    let mut referenced = store.agy_record(&tenant, "one").unwrap().unwrap();
    referenced["meta"] = json!({"unknownReference":"retained"});
    settings
        .upsert(&tenant, "app_providers.records", "\"agy\":one", &referenced.to_string())
        .unwrap();
    assert!(store.agy_form_delete(&tenant, "one").is_err());
    assert_eq!(store.agy_record(&tenant, "one").unwrap(), Some(referenced));
}
