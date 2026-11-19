use super::*;
use agent_config::schema::AcpAgentConfig;
use model_transport::RelayIdentity;
use std::path::{Path, PathBuf};

const ENDPOINT: &str = "http://127.0.0.1:12345";

fn state_root() -> tempfile::TempDir {
    let mut builder = tempfile::Builder::new();
    builder.prefix("shiju-glm-state-test-");
    #[cfg(windows)]
    return builder.tempdir_in(std::env::var_os("LOCALAPPDATA").unwrap()).unwrap();
    #[cfg(unix)]
    return builder.tempdir().unwrap();
}

fn profile(root: &Path) -> AcpLaunchProfile {
    let mut profile = AcpLaunchProfile::new_for_agent(
        "glm-acp-agent",
        &AcpAgentConfig {
            command: "glm-acp-agent".into(),
            env: HashMap::from([("GLM_TEST_TOKEN".into(), "placeholder-token".into())]),
            ..Default::default()
        },
        RelayIdentity {
            tenant: vw_api_transport::TenantScope::default(),
            project: "fixture-project".into(),
            session: "fixture-session".into(),
            provider: "fixture-provider".into(),
            model: "fixture-model".into(),
            generation: 0,
        },
    )
    .unwrap();
    profile.state_root = Some(root.to_owned());
    profile.model_supports_tool_call = true;
    profile
}

fn configured_home(mut profile: AcpLaunchProfile) -> PathBuf {
    configure(&mut profile, ENDPOINT).unwrap();
    let env = &profile.config().env;
    assert_eq!(env["XDG_CONFIG_HOME"], env["ACP_GLM_SESSION_DIR"]);
    assert!(env["Z_AI_API_KEY"] == profile.binding.bearer_token());
    PathBuf::from(&env["ACP_GLM_SESSION_DIR"])
}

#[test]
fn glm_state_directory_concurrent_binding_reuse() {
    let root = state_root();
    let (first, second) = std::thread::scope(|scope| {
        let first = scope.spawn(|| configured_home(profile(root.path())));
        let second = scope.spawn(|| configured_home(profile(root.path())));
        (first.join().unwrap(), second.join().unwrap())
    });
    assert_eq!(first, second);
    assert_eq!(first.parent().unwrap(), root.path().join("acp-glm-state"));
    assert!(first.is_dir());
    #[cfg(windows)]
    assert!(first.join("glm-acp-agent").is_dir());
    assert_eq!(configured_home(profile(root.path())), first);
}

fn assert_unpublished(mut profile: AcpLaunchProfile) {
    let initial_env = profile.config().env.clone();
    let initial_args = profile.config().args.clone();
    let result = configure(&mut profile, ENDPOINT);
    assert!(result.is_err());
    assert!(profile.config().env == initial_env);
    assert_eq!(profile.config().args, initial_args);
    assert_eq!(profile.config().command, "glm-acp-agent");
}

fn binding_directory(profile: &AcpLaunchProfile) -> PathBuf {
    use super::super::super::profile::REFERENCE_ENV;
    let reference = profile.config().env[REFERENCE_ENV].strip_prefix("vw-session-model:").unwrap();
    profile.state_root.as_ref().unwrap().join("acp-glm-state").join(reference)
}

#[test]
fn glm_state_directory_rejects_insecure_existing_directory() {
    let root = state_root();
    let parent = root.path().join("acp-glm-state");
    ensure_private_directory(&parent).unwrap();
    let launch = profile(root.path());
    let binding = binding_directory(&launch);
    std::fs::create_dir(&binding).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&binding, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    assert_unpublished(launch);
    assert_eq!(std::fs::read_dir(&binding).unwrap().count(), 0);
}

#[test]
fn glm_state_directory_rejects_redirected_directory() {
    let root = state_root();
    let target = state_root();
    let link = root.path().join("acp-glm-state");
    #[cfg(unix)]
    std::os::unix::fs::symlink(target.path(), &link).unwrap();
    #[cfg(windows)]
    create_junction(&link, target.path());
    assert_unpublished(profile(root.path()));
    assert_eq!(std::fs::read_dir(target.path()).unwrap().count(), 0);
}

#[cfg(windows)]
#[test]
fn glm_state_directory_rejects_redirected_credentials_directory() {
    let root = state_root();
    let target = state_root();
    let launch = profile(root.path());
    let binding = binding_directory(&launch);
    ensure_private_directory(binding.parent().unwrap()).unwrap();
    ensure_private_directory(&binding).unwrap();
    create_junction(&binding.join("glm-acp-agent"), target.path());
    assert_unpublished(launch);
    assert_eq!(std::fs::read_dir(target.path()).unwrap().count(), 0);
}

#[cfg(windows)]
fn create_junction(link: &Path, target: &Path) {
    let powershell = PathBuf::from(std::env::var_os("SystemRoot").unwrap())
        .join("System32/WindowsPowerShell/v1.0/powershell.exe");
    assert!(
        std::process::Command::new(powershell)
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "$ErrorActionPreference = 'Stop'; New-Item -ItemType Junction -Path $env:GLM_TEST_LINK -Target $env:GLM_TEST_TARGET | Out-Null",
            ])
            .env("GLM_TEST_LINK", link)
            .env("GLM_TEST_TARGET", target)
            .status()
            .unwrap()
            .success()
    );
}
