use super::native_primary::{InstallEnvironment, install_command};
use vw_api_types::agent_apps::AppId::*;

#[test]
fn cc_apps_primary_native_install_windows_and_rejections() {
    for app in [Claude, OpenCode] {
        let command = install_command(app, InstallEnvironment::Windows).unwrap();
        assert_eq!(command.program, "npm");
        assert_eq!(command.args, super::npm::install_args(app).unwrap());
    }
    for app in [Codex, Gemini, ClaudeDesktop, Hermes, GrokBuild, Mcode, Kimi, Omp, Agy] {
        assert!(install_command(app, InstallEnvironment::Posix).is_err());
    }
}

#[cfg(unix)]
#[test]
fn cc_apps_primary_native_install_short_circuit_and_cleanup() {
    use std::{fs, os::unix::fs::PermissionsExt, process::Command};
    for (app, url) in
        [(Claude, "https://claude.ai/install.sh"), (OpenCode, "https://opencode.ai/install")]
    {
        for environment in [InstallEnvironment::Posix, InstallEnvironment::Wsl] {
            for (download, installer, npm) in [(0, 0, 9), (0, 4, 0), (22, 0, 0), (22, 0, 7)] {
                let dir = tempfile::tempdir().unwrap();
                let bin = dir.path().join("bin");
                fs::create_dir(&bin).unwrap();
                let temporary = dir.path().join("temporary files");
                fs::create_dir(&temporary).unwrap();
                for (name, script) in [
                    (
                        "curl",
                        "#!/bin/bash\nprintf '%s\\n' \"$@\" > \"$LOG/curl\"\nprintf 'echo ran > \"$LOG/installer\"; exit %s\\n' \"$INSTALLER_EXIT\" > \"$4\"\nexit \"$DOWNLOAD_EXIT\"\n",
                    ),
                    (
                        "npm",
                        "#!/bin/bash\nprintf '%s\\n' \"$@\" > \"$LOG/npm\"\npwd > \"$LOG/cwd\"\nexit \"$NPM_EXIT\"\n",
                    ),
                ] {
                    let path = bin.join(name);
                    fs::write(&path, script).unwrap();
                    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
                }
                let recipe = install_command(app, environment).unwrap();
                let result = Command::new(recipe.program)
                    .args(recipe.args)
                    .current_dir(dir.path())
                    .env("PATH", format!("{}:/usr/bin:/bin", bin.display()))
                    .env("TMPDIR", &temporary)
                    .env("LOG", dir.path())
                    .env("DOWNLOAD_EXIT", download.to_string())
                    .env("INSTALLER_EXIT", installer.to_string())
                    .env("NPM_EXIT", npm.to_string())
                    .status()
                    .unwrap();
                let fallback = download != 0 || installer != 0;
                assert_eq!(result.code(), Some(if fallback { npm } else { 0 }));
                assert_eq!(dir.path().join("installer").exists(), download == 0);
                assert_eq!(dir.path().join("npm").exists(), fallback);
                assert_eq!(
                    fs::read_to_string(dir.path().join("curl"))
                        .unwrap()
                        .lines()
                        .take(3)
                        .collect::<Vec<_>>(),
                    ["-fsSL", url, "-o"]
                );
                if fallback {
                    assert_eq!(
                        fs::read_to_string(dir.path().join("npm"))
                            .unwrap()
                            .lines()
                            .collect::<Vec<_>>(),
                        super::npm::install_args(app).unwrap()
                    );
                    assert_eq!(
                        fs::read_to_string(dir.path().join("cwd")).unwrap().trim(),
                        dir.path().canonicalize().unwrap().to_str().unwrap()
                    );
                }
                assert_eq!(fs::read_dir(&temporary).unwrap().count(), 0);
            }
        }
    }
}
