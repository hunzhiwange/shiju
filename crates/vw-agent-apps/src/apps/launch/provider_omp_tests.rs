use super::{provider::prepare_provider_launch, request::LaunchMaterial};
use vw_api_types::{
    agent_apps::AppId,
    app_providers::{AppProviderWrite, ProviderProtocol},
};
#[test]
fn cc_apps_omp_provider_private_projection_is_available() {
    let root = tempfile::tempdir().unwrap();
    let profile = AppProviderWrite {
        name: "fixture".into(),
        protocol: ProviderProtocol::OpenaiResponses,
        api_key: "fixture-key".into(),
        base_url: "https://provider.example.test/v1".into(),
        model: "fixture-model".into(),
    };
    let result = prepare_provider_launch(
        AppId::Omp,
        &profile,
        LaunchMaterial {
            program: "/usr/bin/true".into(),
            cwd: root.path().into(),
            args: vec![],
            skip_permissions: false,
        },
        root.path(),
    );
    assert!(result.is_ok(), "reviewed omp OpenAI Responses projection must be reachable");
}

fn profile() -> AppProviderWrite {
    AppProviderWrite {
        name: "fixture".into(),
        protocol: ProviderProtocol::OpenaiResponses,
        api_key: "fixture-key-only".into(),
        base_url: "https://provider.example.test/v1".into(),
        model: "fixture-model".into(),
    }
}
fn material(root: &std::path::Path) -> LaunchMaterial {
    LaunchMaterial {
        program: root.join("omp"),
        cwd: root.into(),
        args: vec!["hello".into()],
        skip_permissions: false,
    }
}
fn executable(root: &std::path::Path, body: &str) {
    use std::os::unix::fs::PermissionsExt;
    let p = root.join("omp");
    std::fs::write(&p, body).unwrap();
    std::fs::set_permissions(p, std::fs::Permissions::from_mode(0o700)).unwrap();
}
#[test]
fn cc_apps_omp_provider_consumes_private_configuration_and_cleans_process_material() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let temp = root.path().join("private");
    std::fs::create_dir(&temp).unwrap();
    executable(
        root.path(),
        "#!/bin/sh\ntest \"$HOME\" = \"$PI_CODING_AGENT_DIR\" || exit 10\ntest \"$SHIJU_PROVIDER_API_KEY\" = fixture-key-only || exit 11\ntest -z \"$OPENAI_API_KEY$ANTHROPIC_API_KEY\" || exit 12\ntest \"$1\" = --model && test \"$2\" = shiju_session/fixture-model || exit 13\ntest \"$3\" = --no-extensions && test \"$4\" = --no-skills && test \"$5\" = hello || exit 14\n",
    );
    let before = std::env::var_os("HOME");
    let launch =
        prepare_provider_launch(AppId::Omp, &profile(), material(root.path()), &temp).unwrap();
    let mut observed = std::path::PathBuf::new();
    launch
        .run_with(|command| {
            let path =
                command.get_envs().find(|(k, _)| *k == "PI_CODING_AGENT_DIR").unwrap().1.unwrap();
            observed = path.into();
            let file = observed.join("models.yml");
            let bytes = std::fs::read(&file).unwrap();
            let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(value["providers"]["shiju_session"]["api"], "openai-responses");
            assert_eq!(value["providers"]["shiju_session"]["apiKey"], "SHIJU_PROVIDER_API_KEY");
            assert_eq!(value["providers"]["shiju_session"]["models"][0]["id"], "fixture-model");
            assert!(!String::from_utf8(bytes).unwrap().contains("fixture-key-only"));
            assert_eq!(std::fs::metadata(&observed).unwrap().permissions().mode() & 0o777, 0o700);
            assert_eq!(std::fs::metadata(&file).unwrap().permissions().mode() & 0o777, 0o600);
            command.status()
        })
        .unwrap();
    assert!(!observed.exists());
    assert!(std::env::var_os("HOME") == before);
}
#[test]
fn cc_apps_omp_provider_rejects_project_overrides_and_wrong_protocol_before_materialization() {
    let root = tempfile::tempdir().unwrap();
    executable(root.path(), "#!/bin/sh\nexit 0\n");
    let temp = root.path().join("private");
    std::fs::create_dir(&temp).unwrap();
    for name in [".omp", ".omp/config.yml", ".omp/config.yaml", ".env"] {
        let p = root.path().join(name);
        let parent = p.parent().unwrap();
        if parent != root.path() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&p, "fixture").unwrap();
        assert!(
            prepare_provider_launch(AppId::Omp, &profile(), material(root.path()), &temp).is_err()
        );
        assert_eq!(std::fs::read_dir(&temp).unwrap().count(), 0);
        std::fs::remove_file(&p).unwrap();
        if parent != root.path() {
            std::fs::remove_dir(parent).unwrap();
        }
    }
    let mut wrong = profile();
    wrong.protocol = ProviderProtocol::Anthropic;
    assert!(prepare_provider_launch(AppId::Omp, &wrong, material(root.path()), &temp).is_err());
    assert_eq!(std::fs::read_dir(&temp).unwrap().count(), 0);
}
#[test]
fn cc_apps_omp_provider_cleans_nonzero_spawn_failure_and_dropped_material() {
    let root = tempfile::tempdir().unwrap();
    let temp = root.path().join("private");
    std::fs::create_dir(&temp).unwrap();
    for body in ["#!/bin/sh\nexit 37\n", "invalid executable"] {
        executable(root.path(), body);
        let launch =
            prepare_provider_launch(AppId::Omp, &profile(), material(root.path()), &temp).unwrap();
        assert!(launch.run().is_err());
        assert_eq!(std::fs::read_dir(&temp).unwrap().count(), 0);
    }
    executable(root.path(), "#!/bin/sh\nexit 0\n");
    let launch =
        prepare_provider_launch(AppId::Omp, &profile(), material(root.path()), &temp).unwrap();
    drop(launch);
    assert_eq!(std::fs::read_dir(&temp).unwrap().count(), 0);
}

#[test]
fn cc_apps_omp_provider_metadata_directory_does_not_override_private_configuration() {
    let root = tempfile::tempdir().unwrap();
    executable(root.path(), "#!/bin/sh\nexit 0\n");
    let metadata = root.path().join(".omp/agent");
    std::fs::create_dir_all(&metadata).unwrap();
    std::fs::write(metadata.join("agent.db"), "fixture-metadata").unwrap();
    let temp = root.path().join("private");
    std::fs::create_dir(&temp).unwrap();
    let launch = prepare_provider_launch(AppId::Omp, &profile(), material(root.path()), &temp)
        .expect("metadata-only .omp directory must not block the explicit private agent directory");
    launch.run().unwrap();
    assert_eq!(std::fs::read_dir(&temp).unwrap().count(), 0);
}
