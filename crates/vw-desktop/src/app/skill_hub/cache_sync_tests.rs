use super::cache_sync::{
    build_cache_sync_command, resolve_shiju_cli, sync_business_skill_cache,
};
use std::fs;
use std::path::Path;

fn quote_for_shell(path: &Path) -> String {
    path.to_string_lossy().replace('\'', "'\\''")
}

fn write_fixture(root: &Path, log: &Path, rebuild_response: &str) {
    let scripts = root.join("scripts");
    fs::create_dir_all(&scripts).expect("create fixture scripts");
    fs::write(root.join("SKILL.md"), "---\nversion: 3.0.3\n---\n").expect("write manifest");
    let script = format!(
        "#!/bin/sh\n\
         printf '%s|%s\\n' \"$*\" \"${{SHIJU_KEY-unset}}\" >> '{}'\n\
         case \"$1\" in\n\
           skill:fetch) printf '%s\\n' '{{\"success\":true,\"data\":{{\"version\":\"1.2.0\"}}}}' ;;\n\
           index) printf '%s\\n' '{}' ;;\n\
           *) printf '%s\\n' '{{\"success\":false,\"error\":\"unexpected\"}}' ;;\n\
         esac\n",
        quote_for_shell(log),
        rebuild_response.replace('\'', "'\\''"),
    );
    let executable = scripts.join("shiju");
    fs::write(&executable, script).expect("write fixture executable");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o755))
            .expect("make fixture executable");
    }
}

#[tokio::test]
async fn cache_sync_fetches_selected_server_version_then_rebuilds_index_without_credentials() {
    let fixture = tempfile::tempdir().expect("fixture root");
    let log = fixture.path().join("argv.log");
    let root = fixture.path().join("installed-shiju");
    write_fixture(&root, &log, r#"{"success":true,"data":{"count":1}}"#);

    let report =
        sync_business_skill_cache(&root, 42).await.expect("cache sync should succeed");

    assert_eq!(report.fetch_exit_code, 0);
    assert_eq!(report.rebuild_exit_code, 0);
    assert_eq!(
        fs::read_to_string(&log).expect("read argv log").lines().collect::<Vec<_>>(),
        ["skill:fetch --skill-id 42|unset", "index rebuild|unset"]
    );
    let command = build_cache_sync_command(
        &resolve_shiju_cli(&root).expect("resolve fixture cli"),
        &root,
        &["skill:fetch", "--skill-id", "42"],
    );
    assert!(Path::new(command.get_program()).ends_with("scripts/shiju"));
    assert!(command.get_envs().any(|(name, value)| name == "SHIJU_KEY" && value.is_none()));
    assert!(!format!("{command:?}").contains("credential"));
}

#[tokio::test]
async fn cache_sync_rejects_business_error_after_fetch_without_exposing_payload() {
    let fixture = tempfile::tempdir().expect("fixture root");
    let log = fixture.path().join("argv.log");
    let root = fixture.path().join("installed-shiju");
    write_fixture(
        &root,
        &log,
        r#"{"success":false,"error":"payload-secret"}"#,
    );

    let error =
        sync_business_skill_cache(&root, 7).await.expect_err("rebuild business error must fail");

    assert!(error.to_string().contains("索引"));
    assert!(!error.to_string().contains("payload-secret"));
    assert_eq!(fs::read_to_string(log).unwrap().lines().count(), 2);
}

#[tokio::test]
async fn cache_sync_rejects_missing_non_executable_and_symlinked_cli() {
    let fixture = tempfile::tempdir().expect("fixture root");
    let root = fixture.path().join("installed-shiju");
    fs::create_dir_all(root.join("scripts")).unwrap();
    fs::write(root.join("scripts/shiju"), "#!/bin/sh\n").unwrap();

    #[cfg(unix)]
    assert!(resolve_shiju_cli(&root).unwrap_err().to_string().contains("不可执行"));

    fs::remove_file(root.join("scripts/shiju")).unwrap();
    assert!(resolve_shiju_cli(&root).unwrap_err().to_string().contains("不存在"));

    #[cfg(unix)]
    {
        use std::os::unix::fs::{PermissionsExt, symlink};
        let outside = fixture.path().join("outside");
        fs::write(&outside, "#!/bin/sh\n").unwrap();
        fs::set_permissions(&outside, fs::Permissions::from_mode(0o755)).unwrap();
        symlink(&outside, root.join("scripts/shiju")).unwrap();
        assert!(resolve_shiju_cli(&root).unwrap_err().to_string().contains("符号链接"));
    }
}

#[tokio::test]
async fn cache_sync_stops_before_rebuild_on_nonzero_fetch_exit() {
    let fixture = tempfile::tempdir().expect("fixture root");
    let root = fixture.path().join("installed-shiju");
    let scripts = root.join("scripts");
    fs::create_dir_all(&scripts).unwrap();
    let executable = scripts.join("shiju");
    fs::write(&executable, "#!/bin/sh\nexit 7\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();
    }

    let error = sync_business_skill_cache(&root, 7).await.unwrap_err();

    assert!(error.to_string().contains("skill:fetch"));
    assert!(error.to_string().contains('7'));
}
