use super::native_hermes_agy::{install_command, update_command};
use super::native_primary::InstallEnvironment::{Posix, Windows, Wsl};
use vw_api_types::agent_apps::AppId::{Agy, Codex, Hermes};

#[test]
fn cc_apps_hermes_agy_install_platform_rules() {
    for app in [Hermes, Agy] {
        let install = install_command(app, Windows).unwrap();
        assert_eq!(install.program, "powershell");
        let script = install.args.last().unwrap();
        assert!(script.contains("install.ps1"));
        assert!(!script.contains("npm"));
        let update = update_command(app, Windows).unwrap();
        if app == Agy {
            assert_eq!(update.program, "agy");
            assert_eq!(update.args, ["update"]);
        } else {
            assert_eq!(update.program, "powershell");
            assert!(update.args.last().unwrap().contains("hermes update"));
            assert!(update.args.last().unwrap().contains("install.ps1"));
        }
    }
    assert!(install_command(Codex, Posix).is_err());
    assert!(update_command(Codex, Windows).is_err());
}

#[cfg(unix)]
#[test]
fn cc_apps_hermes_agy_install_process_results() {
    use std::{fs, os::unix::fs::PermissionsExt, process::Command};
    for app in [Hermes, Agy] {
        for environment in [Posix, Wsl] {
            for update in [false, true] {
                for (download, installer, updater) in [(0, 0, 0), (0, 8, 5), (22, 0, 5), (0, 0, 5)]
                {
                    let dir = tempfile::tempdir().unwrap();
                    let temporary = dir.path().join("temporary files");
                    fs::create_dir(&temporary).unwrap();
                    for (name, script) in [
                        (
                            "curl",
                            "#!/bin/bash\nprintf '%s\\n' \"$@\" > \"$LOG/download\"\nprintf 'echo ran > \"$LOG/installed\"; exit %s\\n' \"$INSTALLER\" > \"$4\"\nexit \"$DOWNLOAD\"\n",
                        ),
                        (
                            "hermes",
                            "#!/bin/bash\necho \"$*\" > \"$LOG/updated\"; exit \"$UPDATER\"\n",
                        ),
                        ("agy", "#!/bin/bash\necho \"$*\" > \"$LOG/updated\"; exit \"$UPDATER\"\n"),
                        ("npm", "#!/bin/bash\ntouch \"$LOG/npm-called\"; exit 0\n"),
                    ] {
                        let path = dir.path().join(name);
                        fs::write(&path, script).unwrap();
                        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
                    }
                    let recipe = if update {
                        update_command(app, environment)
                    } else {
                        install_command(app, environment)
                    }
                    .unwrap();
                    let status = Command::new(recipe.program)
                        .args(recipe.args)
                        .current_dir(dir.path())
                        .env("PATH", format!("{}:/usr/bin:/bin", dir.path().display()))
                        .env("TMPDIR", &temporary)
                        .env("LOG", dir.path())
                        .env("DOWNLOAD", download.to_string())
                        .env("INSTALLER", installer.to_string())
                        .env("UPDATER", updater.to_string())
                        .status()
                        .unwrap();
                    let installs = !update || (app == Hermes && updater != 0);
                    let expected = if installs {
                        if download != 0 { download } else { installer }
                    } else {
                        updater
                    };
                    assert_eq!(status.code(), Some(expected));
                    assert_eq!(dir.path().join("download").exists(), installs);
                    assert_eq!(dir.path().join("installed").exists(), installs && download == 0);
                    assert!(!dir.path().join("npm-called").exists());
                    if update {
                        assert_eq!(
                            fs::read_to_string(dir.path().join("updated")).unwrap(),
                            "update\n"
                        );
                    }
                    if installs {
                        let url = if app == Hermes {
                            "https://raw.githubusercontent.com/NousResearch/hermes-agent/main/scripts/install.sh"
                        } else {
                            "https://antigravity.google/cli/install.sh"
                        };
                        assert!(
                            fs::read_to_string(dir.path().join("download")).unwrap().contains(url)
                        );
                    }
                    assert_eq!(fs::read_dir(&temporary).unwrap().count(), 0);
                }
            }
        }
    }
}
