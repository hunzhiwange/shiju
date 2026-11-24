use super::*;
use std::{fs, os::unix::fs::PermissionsExt, path::Path};

fn executable(path: &Path) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, b"fixture").unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
}

#[test]
fn bundled_codex_is_discoverable_without_a_gateway_path_entry() {
    let root = tempfile::tempdir().unwrap();
    let mut env = LocalReadEnvironment {
        host_id: "fixture-host".into(),
        home: root.path().to_owned(),
        search_paths: vec![root.path().join("bin")],
        applications: root.path().join("Applications"),
    };
    let bundled = env.applications.join(
        "ChatGPT.app/Contents/Resources/codex-cli/CodexCLI.app/Contents/MacOS/codex",
    );
    executable(&bundled);

    assert_eq!(locate_cli(AppId::Codex, &env).unwrap(), bundled.canonicalize().unwrap());
    let id = installation_id(&env.host_id, "codex", bundled.to_str().unwrap());
    assert!(resolve_cli_installation(AppId::Codex, &env, &id).unwrap());

    let path_install = env.search_paths[0].join("codex");
    executable(&path_install);
    env.search_paths.push(bundled.parent().unwrap().to_owned());
    let candidates = cli_entries(AppId::Codex, "codex", &env).collect::<io::Result<Vec<_>>>().unwrap();
    assert_eq!(candidates, [path_install, bundled]);
}
