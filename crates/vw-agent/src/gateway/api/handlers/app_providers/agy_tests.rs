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
    pv["principals"][0]["grants"][0]["provider_config_write"] = json!(false);
    std::fs::write(&policy, pv.to_string()).unwrap();
    assert!(client.agy_provider_save("two", &request, TOKEN).await.is_err());
    assert!(
        crate::app_providers::AppProviderStore::new(db)
            .unwrap()
            .agy_record(&tenant, "two")
            .unwrap()
            .is_none()
    );
    server.abort();
}
