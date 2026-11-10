use super::read_native_directory;
use std::{ffi::OsStr, fs, os::unix::fs::symlink, path::Path};
use vw_api_types::app_providers::catalog::ProviderReadUnavailable::{
    InvalidConfiguration, Unreadable,
};

#[test]
fn aggregation_t35_pi_read_preserves_contract() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    let dir = home.join(".pi/agent");
    fs::create_dir_all(&dir).unwrap();
    let settings = br#"{
      // Pi owns the global default. It need not be a custom provider.
      "defaultProvider": "builtin", "defaultModel": "model-a", "future": true,
    }"#;
    let models = br#"{"providers":{"custom":{"apiKey":"fixture-only"}},"other":true}"#;
    fs::write(dir.join("settings.json"), settings).unwrap();
    fs::write(dir.join("models.json"), models).unwrap();
    symlink("missing-login-file", dir.join("auth.json")).unwrap();

    let state = read_native_directory(home, None, None).unwrap();
    assert_eq!(state.members, ["custom"]);
    assert_eq!(state.default_provider_id.as_deref(), Some("builtin"));
    assert_eq!(state.default_model_id.as_deref(), Some("model-a"));
    assert_eq!(fs::read(dir.join("settings.json")).unwrap(), settings);
    assert_eq!(fs::read(dir.join("models.json")).unwrap(), models);
    assert_eq!(fs::read_link(dir.join("auth.json")).unwrap(), Path::new("missing-login-file"));
    assert_eq!(fs::read_dir(&dir).unwrap().count(), 3);

    fs::write(dir.join("models.json"), br#"{"providers":{"native.oauth":{},"custom":{}}}"#)
        .unwrap();
    assert_eq!(
        read_native_directory(home, None, None).unwrap().members,
        ["custom", "native.oauth"]
    );
}

#[test]
fn aggregation_t35_pi_directory_uses_controlled_override_then_environment_then_default() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    for (directory, id) in [(".pi/agent", "default"), ("env", "env"), ("managed", "managed")] {
        let directory = home.join(directory);
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join("models.json"), format!(r#"{{"providers":{{"{id}":{{}}}}}}"#))
            .unwrap();
    }
    let env = home.join("env");
    let managed = home.join("managed");
    for (configured, env, id) in [
        (Some(managed.as_path()), Some(env.as_os_str()), "managed"),
        (Some(managed.as_path()), Some(OsStr::new("ignored-relative")), "managed"),
        (None, Some(env.as_os_str()), "env"),
        (None, Some(OsStr::new("")), "default"),
        (None, None, "default"),
    ] {
        assert_eq!(read_native_directory(home, configured, env).unwrap().members, [id]);
    }
    assert_eq!(
        read_native_directory(home, Some(Path::new("relative")), Some(env.as_os_str())),
        Err(InvalidConfiguration)
    );
    for path in [Path::new("relative"), Path::new("/outside"), &home.join("../escape")] {
        assert_eq!(
            read_native_directory(home, None, Some(path.as_os_str())),
            Err(InvalidConfiguration)
        );
    }
}

#[test]
fn aggregation_t35_pi_reads_missing_files_without_creating_them() {
    let home = tempfile::tempdir().unwrap();
    let state = read_native_directory(home.path(), None, None).unwrap();
    assert!(state.members.is_empty());
    assert!(state.default_provider_id.is_none());
    assert!(state.default_model_id.is_none());
    assert!(!home.path().join(".pi").exists());
}

#[test]
fn aggregation_t35_pi_rejects_invalid_or_oversized_files() {
    let home = tempfile::tempdir().unwrap();
    let dir = home.path().join(".pi/agent");
    fs::create_dir_all(&dir).unwrap();
    for (filename, invalid) in [
        ("settings.json", br#"{"defaultModel": 7}"#.as_slice()),
        ("models.json", br#"{"providers":[]}"#.as_slice()),
    ] {
        let path = dir.join(filename);
        fs::write(&path, invalid).unwrap();
        assert_eq!(read_native_directory(home.path(), None, None), Err(InvalidConfiguration));
        let mut boundary = b"{}".to_vec();
        boundary.resize(1024 * 1024, b' ');
        fs::write(&path, &boundary).unwrap();
        assert!(read_native_directory(home.path(), None, None).is_ok());
        boundary.push(b' ');
        fs::write(&path, boundary).unwrap();
        assert_eq!(read_native_directory(home.path(), None, None), Err(Unreadable));
        fs::remove_file(path).unwrap();
    }
}

#[test]
fn aggregation_t35_pi_rejects_symlinked_config_paths() {
    let home = tempfile::tempdir().unwrap();
    let foreign = tempfile::tempdir().unwrap();
    symlink(foreign.path(), home.path().join(".pi")).unwrap();
    assert_eq!(read_native_directory(home.path(), None, None), Err(Unreadable));
    fs::remove_file(home.path().join(".pi")).unwrap();
    let dir = home.path().join(".pi/agent");
    fs::create_dir_all(&dir).unwrap();
    for file in ["settings.json", "models.json"] {
        let path = dir.join(file);
        symlink(foreign.path().join(file), &path).unwrap();
        assert_eq!(read_native_directory(home.path(), None, None), Err(Unreadable));
        fs::remove_file(path).unwrap();
    }
}
