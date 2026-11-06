use super::*;
use std::os::unix::fs::PermissionsExt;
fn fixture() -> (tempfile::TempDir, LocalReadEnvironment, WriteEnvironment) {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let prefix = root.path().join("prefix with spaces");
    let bin = prefix.join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    for name in ["node", "npm"] {
        let p = bin.join(name);
        std::fs::write(&p, "#!/bin/sh\nexit 99\n").unwrap();
        std::fs::set_permissions(p, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    let env = LocalReadEnvironment {
        host_id: "host".into(),
        home: root.path().into(),
        search_paths: vec![bin.clone()],
        applications: root.path().join("Applications"),
    };
    (root, env, WriteEnvironment { prefix, npm: bin.join("npm") })
}
#[test]
fn cc_apps_agy_managed_install_preview_accepts_authorized_empty_prefix() {
    let (_root, env, write) = fixture();
    let request = AppActionPreviewRequest {
        app_id: AppId::Agy,
        action: AppLifecycleAction::Install,
        target: AppActionTarget::Environment { environment: AppInstallationEnvironment::Macos },
    };
    let p = preview(request, &env, &write).expect("reviewed agy installer must be reachable");
    assert_eq!(p.source, AppInstallationSource::System);
    assert!(!write.prefix.join(".shiju-agy").exists());
}
#[test]
fn cc_apps_agy_managed_manual_install_does_not_acquire_ownership() {
    let (_root, env, write) = fixture();
    let binary = write.prefix.join("bin/agy");
    std::fs::write(&binary, "#!/bin/sh\necho 1.5.2\n").unwrap();
    std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o700)).unwrap();
    let report = scoped_discover(AppId::Agy, &env, &write).unwrap();
    assert_eq!(report.installations[0].source, AppInstallationSource::Unknown);
    for action in [AppLifecycleAction::Update, AppLifecycleAction::Uninstall] {
        let r = preview(
            AppActionPreviewRequest {
                app_id: AppId::Agy,
                action,
                target: AppActionTarget::Installation {
                    installation_id: report.installations[0].id.clone(),
                },
            },
            &env,
            &write,
        );
        assert!(r.is_err());
    }
}
#[test]
fn cc_apps_agy_managed_receipt_rejects_replacement_and_symlinks() {
    use std::os::unix::fs::symlink;
    let (_root, _env, write) = fixture();
    let binary = write.prefix.join("bin/agy");
    std::fs::copy("/usr/bin/true", &binary).unwrap();
    super::super::local_agy::record(&write.prefix).unwrap();
    assert!(super::super::local_agy::owns(&binary));
    let original = std::fs::read(&binary).unwrap();
    let mut replaced = original.clone();
    replaced.push(0);
    std::fs::write(&binary, replaced).unwrap();
    assert!(!super::super::local_agy::owns(&binary));
    std::fs::write(&binary, original).unwrap();
    let receipt = write.prefix.join(".shiju-agy/receipt");
    let other = write.prefix.join("saved-receipt");
    std::fs::rename(&receipt, &other).unwrap();
    symlink(&other, &receipt).unwrap();
    assert!(!super::super::local_agy::owns(&binary));
    std::fs::remove_file(receipt).unwrap();
    std::fs::remove_file(&binary).unwrap();
    symlink("/usr/bin/true", &binary).unwrap();
    assert!(super::super::local_agy::record(&write.prefix).is_err());
}
#[tokio::test]
async fn cc_apps_agy_managed_install_update_and_selected_uninstall_use_real_processes() {
    let (root, env, write) = fixture();
    let source = root.path().join("fixture.c");
    std::fs::write(
        &source,
        r#"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
int main(int argc, char **argv) {
 if (argc > 1 && strcmp(argv[1], "update") == 0) {
  char p[4096]; snprintf(p,sizeof(p),"%s/updated",getenv("HOME"));
  FILE *f=fopen(p,"w"); if (!f) return 5; fputs(getenv("HOME"),f); fclose(f);
 }
 puts("1.5.2"); return 0;
}
"#,
    )
    .unwrap();
    let artifact = root.path().join("native-agy");
    assert!(
        std::process::Command::new("/usr/bin/cc")
            .arg(&source)
            .arg("-o")
            .arg(&artifact)
            .status()
            .unwrap()
            .success()
    );
    // Replace only the official downloader boundary. The production command,
    // bash installer argv, child environment, discovery and removal all run.
    let downloader = write.prefix.join("bin/curl");
    std::fs::write(&downloader, format!(r##"#!/bin/sh
[ "$1" = '-fsSL' ] && [ "$2" = 'https://antigravity.google/cli/install.sh' ] && [ "$3" = '-o' ] || exit 77
cat > "$4" <<'INSTALL'
[ "$1" = '--dir' ] || exit 78
cp '{}' "$2/agy"
INSTALL
"##, artifact.display())).unwrap();
    std::fs::set_permissions(&downloader, std::fs::Permissions::from_mode(0o700)).unwrap();
    let request = AppActionPreviewRequest {
        app_id: AppId::Agy,
        action: AppLifecycleAction::Install,
        target: AppActionTarget::Environment { environment: AppInstallationEnvironment::Macos },
    };
    let plan = preview(request, &env, &write).unwrap();
    run(
        plan,
        LocalReadEnvironment {
            host_id: env.host_id.clone(),
            home: env.home.clone(),
            search_paths: env.search_paths.clone(),
            applications: env.applications.clone(),
        },
        write.clone(),
    )
    .unwrap();
    let report = scoped_discover(AppId::Agy, &env, &write).unwrap();
    assert_eq!(report.installations[0].source, AppInstallationSource::System);
    let id = report.installations[0].id.clone();
    let update = preview(
        AppActionPreviewRequest {
            app_id: AppId::Agy,
            action: AppLifecycleAction::Update,
            target: AppActionTarget::Installation { installation_id: id.clone() },
        },
        &env,
        &write,
    )
    .unwrap();
    run(
        update,
        LocalReadEnvironment {
            host_id: env.host_id.clone(),
            home: env.home.clone(),
            search_paths: env.search_paths.clone(),
            applications: env.applications.clone(),
        },
        write.clone(),
    )
    .unwrap();
    assert_eq!(
        std::fs::read_to_string(write.prefix.join(".shiju-agy/updated")).unwrap(),
        write.prefix.join(".shiju-agy").to_string_lossy()
    );
    let remaining = root.path().join("other-agy");
    std::fs::copy(&artifact, &remaining).unwrap();
    let removal = preview(
        AppActionPreviewRequest {
            app_id: AppId::Agy,
            action: AppLifecycleAction::Uninstall,
            target: AppActionTarget::Installation { installation_id: id },
        },
        &env,
        &write,
    )
    .unwrap();
    uninstall(removal, env, write.clone(), || Ok(())).await.unwrap();
    assert!(!write.prefix.join("bin/agy").exists());
    assert!(remaining.exists());
}
#[test]
fn cc_apps_agy_managed_install_rejects_redirected_bin_before_writing() {
    let (root, env, write) = fixture();
    let bin = write.prefix.join("bin");
    let other = root.path().join("other-bin");
    std::fs::rename(&bin, &other).unwrap();
    std::os::unix::fs::symlink(other, bin).unwrap();
    assert!(
        preview(
            AppActionPreviewRequest {
                app_id: AppId::Agy,
                action: AppLifecycleAction::Install,
                target: AppActionTarget::Environment {
                    environment: AppInstallationEnvironment::Macos
                }
            },
            &env,
            &write
        )
        .is_err()
    );
    assert!(!write.prefix.join(".shiju-agy").exists());
}
#[tokio::test]
async fn cc_apps_agy_managed_cancel_reaps_installer_without_source_receipt() {
    use super::super::operations::{Operation, Phase};
    let (_root, env, write) = fixture();
    let downloader = write.prefix.join("bin/curl");
    std::fs::write(&downloader, "#!/bin/sh\necho $$ > \"$HOME/download.pid\"\nsleep 30\n").unwrap();
    std::fs::set_permissions(&downloader, std::fs::Permissions::from_mode(0o700)).unwrap();
    let plan = preview(
        AppActionPreviewRequest {
            app_id: AppId::Agy,
            action: AppLifecycleAction::Install,
            target: AppActionTarget::Environment { environment: AppInstallationEnvironment::Macos },
        },
        &env,
        &write,
    )
    .unwrap();
    let directory = write.prefix.clone();
    let op = Operation::new();
    op.start(vec![AppId::Agy], move || run(plan, env, write)).unwrap();
    let marker = directory.join(".shiju-agy/download.pid");
    for _ in 0..500 {
        if marker.exists() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    let pid: i32 = std::fs::read_to_string(marker).unwrap().trim().parse().unwrap();
    op.cancel_and_wait().await;
    assert_eq!(op.snapshot().phase, Phase::Terminal);
    assert!(!directory.join(".shiju-agy/receipt").exists());
    // SAFETY: signal zero checks only the observed disposable installer process.
    assert_eq!(unsafe { libc::kill(pid, 0) }, -1);
}
