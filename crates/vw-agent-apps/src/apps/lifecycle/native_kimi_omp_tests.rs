use super::native_kimi_omp::{install_command, kimi_installer_update_command, update_command};
use super::native_primary::InstallEnvironment::{Posix, Windows, Wsl};
use vw_api_types::agent_apps::AppId::{Kimi, Omp, Pi};
#[test]
fn cc_apps_kimi_omp_install_rules() {
    for env in [Posix, Windows, Wsl] {
        for (app, name, args) in
            [(Kimi, "kimi", vec!["upgrade", "--yes"]), (Omp, "omp", vec!["update"])]
        {
            let install = install_command(app, env).unwrap();
            let script = install.args.last().unwrap();
            assert!(script.contains(if app == Kimi {
                "https://code.kimi.com/kimi-code/install."
            } else {
                "https://omp.sh/install"
            }));
            assert!(!script.contains("npm"));
            let update = update_command(app, env, "/opt/bin/tool").unwrap();
            assert_eq!(update.program, name);
            assert_eq!(update.args, args);
        }
        assert!(update_command(Omp, env, "/nix/store/hash/bin/omp").is_err());
        assert!(install_command(Pi, env).is_err());
        assert!(update_command(Pi, env, "/bin/pi").is_err());
        assert!(kimi_installer_update_command(env, "").is_err());
        let legacy = kimi_installer_update_command(env, "/tmp/kimi user's dir").unwrap();
        assert!(legacy.args.last().unwrap().contains("KIMI_INSTALL_DIR"));
    }
}
#[cfg(unix)]
#[test]
fn cc_apps_kimi_omp_install_process_results() {
    use std::{fs, os::unix::fs::PermissionsExt, process::Command};
    for app in [Kimi, Omp] {
        for env in [Posix, Wsl] {
            for (download, installer) in [(0, 0), (22, 0), (0, 8)] {
                let dir = tempfile::tempdir().unwrap();
                let temporary = dir.path().join("temporary files");
                fs::create_dir(&temporary).unwrap();
                let curl = dir.path().join("curl");
                fs::write(&curl, "#!/bin/bash\nprintf '%s\\n' \"$@\" > \"$LOG/download\"\nprintf 'echo \"$KIMI_INSTALL_DIR\" > \"$LOG/installed\"; exit %s\\n' \"$INSTALLER\" > \"$4\"\nexit \"$DOWNLOAD\"\n").unwrap();
                fs::set_permissions(&curl, fs::Permissions::from_mode(0o700)).unwrap();
                for legacy in [false, true] {
                    if legacy && app != Kimi {
                        continue;
                    }
                    let root = "/tmp/user's kimi dir";
                    let command = if legacy {
                        kimi_installer_update_command(env, root)
                    } else {
                        install_command(app, env)
                    }
                    .unwrap();
                    let status = Command::new(command.program)
                        .args(command.args)
                        .current_dir(dir.path())
                        .env("PATH", format!("{}:/usr/bin:/bin", dir.path().display()))
                        .env("TMPDIR", &temporary)
                        .env("LOG", dir.path())
                        .env("DOWNLOAD", download.to_string())
                        .env("INSTALLER", installer.to_string())
                        .status()
                        .unwrap();
                    assert_eq!(
                        status.code(),
                        Some(if download != 0 { download } else { installer })
                    );
                    assert_eq!(dir.path().join("installed").exists(), download == 0);
                    if download == 0 {
                        if legacy {
                            assert_eq!(
                                fs::read_to_string(dir.path().join("installed")).unwrap(),
                                format!("{root}\n")
                            );
                        }
                        fs::remove_file(dir.path().join("installed")).unwrap();
                    }
                    assert_eq!(fs::read_dir(&temporary).unwrap().count(), 0);
                }
            }
        }
    }
}

#[cfg(unix)]
#[test]
fn cc_apps_kimi_omp_install_update_exit_codes() {
    use std::{fs, os::unix::fs::PermissionsExt, process::Command};
    for (app, name, expected) in [(Kimi, "kimi", "upgrade\n--yes\n"), (Omp, "omp", "update\n")] {
        for env in [Posix, Wsl] {
            for code in [0, 9] {
                let dir = tempfile::tempdir().unwrap();
                let exe = dir.path().join(name);
                fs::write(
                    &exe,
                    "#!/bin/bash\nprintf '%s\\n' \"$@\" > arguments\nexit \"$RESULT\"\n",
                )
                .unwrap();
                fs::set_permissions(&exe, fs::Permissions::from_mode(0o700)).unwrap();
                let recipe = update_command(app, env, exe.to_str().unwrap()).unwrap();
                let status = Command::new(&exe)
                    .args(recipe.args)
                    .env("RESULT", code.to_string())
                    .current_dir(dir.path())
                    .status()
                    .unwrap();
                assert_eq!(status.code(), Some(code));
                assert_eq!(fs::read_to_string(dir.path().join("arguments")).unwrap(), expected);
                assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 2);
            }
        }
    }
}
