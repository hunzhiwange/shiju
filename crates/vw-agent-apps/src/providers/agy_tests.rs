use super::agy::*;
use crate::apps::launch::request::LaunchMaterial;
use std::{fs, os::unix::fs::PermissionsExt, path::Path};
use vw_api_types::{app_providers::agy::AgyProfile, id::TenantScope};
fn profile() -> AgyProfile {
    AgyProfile {
        name: "Relay".into(),
        model: Some("Gemini Display Name".into()),
        api_key: Some("fake-agy-key".into()),
        base_url: Some("https://example.com".into()),
    }
}
fn material(home: &Path) -> LaunchMaterial {
    LaunchMaterial {
        program: home.join("agy"),
        cwd: home.into(),
        args: vec![],
        skip_permissions: false,
    }
}
fn fixture() -> tempfile::TempDir {
    let t = tempfile::tempdir().unwrap();
    fs::set_permissions(t.path(), fs::Permissions::from_mode(0o700)).unwrap();
    fs::create_dir_all(t.path().join(".gemini/config")).unwrap();
    fs::create_dir_all(t.path().join(".gemini/antigravity-cli")).unwrap();
    fs::write(t.path().join(".gemini/antigravity-cli/settings.json"), br#"{"model":"native"}"#)
        .unwrap();
    fs::write(t.path().join(".gemini/config/AGENTS.md"), b"native prompt").unwrap();
    fs::write(t.path().join("credentials"), b"native-login").unwrap();
    fs::write(t.path().join("agy"), "#!/bin/sh\nexit 0\n").unwrap();
    fs::set_permissions(t.path().join("agy"), fs::Permissions::from_mode(0o700)).unwrap();
    t
}
#[test]
fn cc04_agy_switch_preserves_native_profile() {
    let t = fixture();
    let tenant = TenantScope::new("p", "a").unwrap();
    let scope = Scope {
        home: t.path(),
        private_root: t.path(),
        host: "host",
        tenant: &tenant,
        provider: "one",
    };
    let launch = prepare(&scope, &profile(), material(t.path())).unwrap();
    let path = scope.directory();
    assert!(
        path.join("antigravity-cli/settings.json").is_file(),
        "real Agy profile layout required"
    );
    let doc: serde_json::Value =
        serde_json::from_slice(&fs::read(path.join("antigravity-cli/settings.json")).unwrap())
            .unwrap();
    assert_eq!(doc["modelProvider"], "gemini");
    assert_eq!(doc["model"], "Gemini Display Name");
    assert_eq!(fs::read_link(path.join("config")).unwrap(), t.path().join(".gemini/config"));
    launch
        .run_with(|cmd| {
            assert_eq!(
                cmd.get_envs().find(|(k, _)| *k == "GEMINI_API_KEY").unwrap().1.unwrap(),
                "fake-agy-key"
            );
            fs::write(path.join("session.json"), b"session").unwrap();
            std::process::Command::new("/usr/bin/true").status()
        })
        .unwrap();
    assert_eq!(fs::read(path.join("session.json")).unwrap(), b"session");
    assert_eq!(fs::read(t.path().join("credentials")).unwrap(), b"native-login");
    assert_eq!(
        fs::read(t.path().join(".gemini/antigravity-cli/settings.json")).unwrap(),
        br#"{"model":"native"}"#
    );
}
#[test]
fn cc04_agy_failed_apply_keeps_original_binding() {
    let t = fixture();
    let tenant = TenantScope::new("p", "a").unwrap();
    let scope = Scope {
        home: t.path(),
        private_root: t.path(),
        host: "host",
        tenant: &tenant,
        provider: "one",
    };
    let path = scope.directory();
    fs::create_dir_all(path.join("config")).unwrap();
    for dir in path.ancestors().take_while(|p| *p != t.path()) {
        fs::set_permissions(dir, fs::Permissions::from_mode(0o700)).unwrap();
    }
    fs::write(path.join("config/user"), b"keep").unwrap();
    assert!(
        prepare(&scope, &profile(), material(t.path())).is_err(),
        "must refuse existing native config directory"
    );
    assert_eq!(fs::read(path.join("config/user")).unwrap(), b"keep");
    assert_eq!(fs::read(t.path().join("credentials")).unwrap(), b"native-login");
    assert!(!path.join("antigravity-cli/settings.json").exists());
}

#[test]
fn cc04_agy_profiles_are_isolated_and_sessions_survive_failure_and_edit() {
    let t = fixture();
    let tenant = TenantScope::new("p", "a").unwrap();
    let other = TenantScope::new("p", "b").unwrap();
    let scope = Scope {
        home: t.path(),
        private_root: t.path(),
        host: "host",
        tenant: &tenant,
        provider: "one",
    };
    let launch = prepare(&scope, &profile(), material(t.path())).unwrap();
    let path = scope.directory();
    let file = path.join("antigravity-cli/settings.json");
    assert_eq!(fs::metadata(&file).unwrap().permissions().mode() & 0o777, 0o600);
    assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o700);
    assert!(
        prepare(&scope, &profile(), material(t.path())).is_err(),
        "active profile has an exclusive lease"
    );
    fs::write(path.join("session.json"), b"saved-session").unwrap();
    assert!(launch.run_with(|_| Err(std::io::Error::other("fake launch failure"))).is_err());
    fs::write(&file, b"{\n// native option\n\"theme\":\"dark\",\"model\":\"old\"\n}").unwrap();
    let mut p = profile();
    p.model = None;
    p.base_url = None;
    let launch = prepare(&scope, &p, material(t.path())).unwrap();
    launch
        .run_with(|cmd| {
            assert!(!cmd.get_envs().any(|(k, _)| k == "GOOGLE_GEMINI_BASE_URL"));
            std::process::Command::new("/usr/bin/true").status()
        })
        .unwrap();
    let bytes = fs::read_to_string(&file).unwrap();
    assert!(bytes.contains("// native option"));
    assert!(bytes.contains("\"theme\":\"dark\""));
    assert!(!bytes.contains("\"model\""));
    assert!(!bytes.contains("fake-agy-key"));
    assert_eq!(fs::read(path.join("session.json")).unwrap(), b"saved-session");
    for (host, tenant, provider) in
        [("other-host", &tenant, "one"), ("host", &other, "one"), ("host", &tenant, "two")]
    {
        let other = Scope { home: t.path(), private_root: t.path(), host, tenant, provider };
        assert_ne!(other.directory(), path);
        drop(prepare(&other, &profile(), material(t.path())).unwrap());
        assert!(!other.directory().join("session.json").exists());
    }
}
#[test]
fn cc04_agy_native_login_uses_only_session_model_and_invalid_inputs_do_not_write() {
    let t = fixture();
    let tenant = TenantScope::new("p", "a").unwrap();
    let scope = Scope {
        home: t.path(),
        private_root: t.path(),
        host: "host",
        tenant: &tenant,
        provider: "one",
    };
    let p = AgyProfile {
        name: "Google login".into(),
        model: Some("Display Name".into()),
        api_key: None,
        base_url: None,
    };
    prepare(&scope, &p, material(t.path()))
        .unwrap()
        .run_with(|cmd| {
            assert!(
                catalog_lease(&scope, true).is_err(),
                "native login process retains catalog reference"
            );
            assert_eq!(
                cmd.get_args().collect::<Vec<_>>(),
                vec![std::ffi::OsStr::new("--model=Display Name")]
            );
            assert!(
                !cmd.get_envs()
                    .any(|(k, _)| k == "GEMINI_API_KEY" || k == "GOOGLE_GEMINI_BASE_URL")
            );
            std::process::Command::new("/usr/bin/true").status()
        })
        .unwrap();
    assert!(catalog_lease(&scope, true).is_ok());
    assert!(!scope.directory().exists());
    for (key, url, model) in [
        (Some("bad\nkey"), None, None),
        (None, Some("https://example.com"), None),
        (Some("[redacted]"), None, None),
        (Some("key"), None, Some("--unsafe")),
    ] {
        let p = AgyProfile {
            name: "Fixture".into(),
            model: model.map(str::to_string),
            api_key: key.map(str::to_string),
            base_url: url.map(str::to_string),
        };
        assert!(prepare(&scope, &p, material(t.path())).is_err());
        assert!(!scope.directory().exists());
    }
    let mut m = material(t.path());
    m.skip_permissions = true;
    assert!(prepare(&scope, &profile(), m).is_err());
    assert!(!scope.directory().exists());
}
#[test]
fn cc04_agy_wrong_link_and_directory_replacement_are_rejected() {
    let t = fixture();
    let tenant = TenantScope::new("p", "a").unwrap();
    let scope = Scope {
        home: t.path(),
        private_root: t.path(),
        host: "host",
        tenant: &tenant,
        provider: "one",
    };
    let launch = prepare(&scope, &profile(), material(t.path())).unwrap();
    drop(launch);
    let path = scope.directory();
    let settings = path.join("antigravity-cli/settings.json");
    let before = fs::read(&settings).unwrap();
    fs::remove_file(path.join("config")).unwrap();
    std::os::unix::fs::symlink(t.path().join("credentials"), path.join("config")).unwrap();
    assert!(prepare(&scope, &profile(), material(t.path())).is_err());
    assert_eq!(fs::read(&settings).unwrap(), before);
    assert_eq!(fs::read_link(path.join("config")).unwrap(), t.path().join("credentials"));
    fs::remove_file(path.join("config")).unwrap();
    let launch = prepare(&scope, &profile(), material(t.path())).unwrap();
    fs::rename(path.join("antigravity-cli"), path.join("moved")).unwrap();
    fs::create_dir(path.join("antigravity-cli")).unwrap();
    let ran = std::cell::Cell::new(false);
    assert!(
        launch
            .run_with(|_| {
                ran.set(true);
                std::process::Command::new("/usr/bin/true").status()
            })
            .is_err()
    );
    assert!(!ran.get());
    assert_eq!(fs::read(t.path().join("credentials")).unwrap(), b"native-login");
}
#[test]
fn cc04_agy_pending_recovers_settings_without_global_pointer_or_native_writes() {
    use super::live::commit::{ExecutionHostId, HOOK, Phase, ProviderTransactions};
    for phase in
        [Phase::Intent, Phase::Marked, Phase::FilePublished, Phase::Published, Phase::Settled]
    {
        let t = fixture();
        let tenant = TenantScope::new("p", "a").unwrap();
        let scope = Scope {
            home: t.path(),
            private_root: t.path(),
            host: "host",
            tenant: &tenant,
            provider: "one",
        };
        drop(prepare(&scope, &profile(), material(t.path())).unwrap());
        let path = scope.directory();
        fs::write(path.join("session.json"), b"keep").unwrap();
        let mut p = profile();
        p.model = Some("New Model".into());
        HOOK.with_borrow_mut(|h| {
            *h = Some(Box::new(move |_, _, at| {
                if at == phase { Err(super::live::write_guard::WriteError::Io) } else { Ok(()) }
            }))
        });
        let failed = prepare(&scope, &p, material(t.path()));
        HOOK.with_borrow_mut(|h| *h = None);
        assert!(failed.is_err());
        assert!(
            prepare(&scope, &p, material(t.path())).is_err(),
            "recovery requires fresh request"
        );
        drop(prepare(&scope, &p, material(t.path())).unwrap());
        let tx = ProviderTransactions::new(&path, ExecutionHostId::new("host").unwrap()).unwrap();
        assert!(tx.pending(vw_api_types::agent_apps::AppId::Agy).unwrap().is_none());
        let state = tx.state(vw_api_types::agent_apps::AppId::Agy).unwrap();
        assert!(state.pointer.is_none());
        assert!(state.mode.is_none());
        assert_eq!(fs::read(path.join("session.json")).unwrap(), b"keep");
        assert_eq!(fs::read(t.path().join("credentials")).unwrap(), b"native-login");
        let doc: serde_json::Value =
            serde_json::from_slice(&fs::read(path.join("antigravity-cli/settings.json")).unwrap())
                .unwrap();
        assert_eq!(doc["model"], "New Model");
    }
}
