use super::*;
use crate::providers::{
    live::commit::{ExecutionHostId, PendingTarget, ProviderTransactions},
    mode_state::StackState,
};
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};
use vw_api_types::{agent_apps::AppId, id::TenantScope};

struct Fixture {
    home: tempfile::TempDir,
    _root: tempfile::TempDir,
    tx: ProviderTransactions,
    tenant: TenantScope,
}

impl Fixture {
    fn new() -> Self {
        let home = tempfile::tempdir().unwrap();
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let tx =
            ProviderTransactions::new(root.path(), ExecutionHostId::new("local").unwrap()).unwrap();
        Self { home, _root: root, tx, tenant: TenantScope::new("platform", "company").unwrap() }
    }

    fn files(&self) -> ClaudeDesktopFiles {
        ClaudeDesktopFiles::open(self.home.path()).unwrap()
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.home.path().join("Library/Application Support").join(relative)
    }
}

#[test]
fn aggregation_t31_desktop_publish_preserves_contract() {
    let fixture = Fixture::new();
    let mut files = fixture.files();
    let other = TenantScope::new("platform", "other").unwrap();
    fixture
        .tx
        .commit(
            &other,
            &mut files.guard,
            &[],
            PendingTarget {
                stack: Some(StackState {
                    members: vec!["other-provider".into()],
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();

    let state = fixture.tx.state(AppId::ClaudeDesktop).unwrap();
    let result = files.apply(
        &fixture.tx,
        &fixture.tenant,
        &state,
        "official",
        &ClaudeDesktopProfile { name: "Official".into(), config: ClaudeDesktopConfig::Official {} },
        || true,
    );
    assert!(matches!(result, Err(WriteError::Conflict)));
    assert!(!fixture.path("Claude/claude_desktop_config.json").exists());
    assert!(fixture.tx.state(AppId::ClaudeDesktop).unwrap().pointer.is_none());
}

use super::publication::{
    DesktopInstallation, DesktopMode, DesktopPublicationError, DesktopTarget, get_status, run,
};
use crate::{
    apps::discovery::read::{LocalReadEnvironment, read},
    providers::{
        desktop_mapping::catalog::{DesktopMappingMember, DesktopModelCatalog},
        live::commit::{AppState, HOOK, Phase},
    },
};
use vw_api_types::{
    agent_apps::AppDesktopObservation,
    app_providers::desktop_mapping::{DesktopMapping, DesktopMappingMode, DesktopModelRoute},
};

const APP: AppId = AppId::ClaudeDesktop;
const NORMAL: &str = "Claude/claude_desktop_config.json";
const THIRD: &str = "Claude-3p/claude_desktop_config.json";
const META: &str = "Claude-3p/configLibrary/_meta.json";

impl Fixture {
    fn install(&self) {
        fs::create_dir_all(self.home.path().join("Applications/Claude.app")).unwrap();
    }

    fn installation(&self) -> DesktopInstallation {
        let home = self.home.path().to_path_buf();
        let observation = read(
            APP,
            &LocalReadEnvironment {
                host_id: "local".into(),
                applications: home.join("SystemApplications"),
                home,
                search_paths: vec![],
            },
        )
        .unwrap()
        .desktop
        .unwrap();
        DesktopInstallation::from(&observation)
    }

    fn profile_path(&self) -> PathBuf {
        self.path(&format!("Claude-3p/configLibrary/{PROFILE_ID}.json"))
    }

    fn status(&self) -> publication::DesktopStatus {
        get_status(self.home.path(), self.installation(), Some(&self.tx), &self.tenant).unwrap()
    }

    fn state(&self) -> AppState {
        self.tx.state(APP).unwrap()
    }

    fn run(
        &self,
        state: &AppState,
        target: DesktopTarget<'_>,
        unchanged: impl Fn() -> bool,
    ) -> Result<(), DesktopPublicationError> {
        run(self.home.path(), self.installation(), &self.tx, &self.tenant, state, target, unchanged)
    }
}

fn profile(base_url: &str) -> ClaudeDesktopProfile {
    ClaudeDesktopProfile {
        name: "Fixture".into(),
        config: ClaudeDesktopConfig::ThirdParty {
            base_url: base_url.into(),
            auth_token: "fixture-token".into(),
        },
    }
}

fn catalog() -> DesktopModelCatalog {
    let mapping = DesktopMapping {
        mode: DesktopMappingMode::Proxy,
        routes: [(
            "claude-sonnet-5".into(),
            DesktopModelRoute {
                model: "fixture-sonnet".into(),
                label_override: Some("Fixture Sonnet".into()),
                supports_1m: true,
            },
        )]
        .into(),
    };
    DesktopModelCatalog::new(
        DesktopMappingMember {
            provider_id: "primary",
            key: "primary",
            name: "Primary",
            mapping: &mapping,
        },
        &[DesktopMappingMember {
            provider_id: "secondary",
            key: "secondary",
            name: "Secondary",
            mapping: &mapping,
        }],
    )
    .unwrap()
}

fn aggregate<'a>(
    gateway: &'a ClaudeDesktopProfile,
    catalog: &'a DesktopModelCatalog,
) -> DesktopTarget<'a> {
    DesktopTarget::Route {
        gateway,
        catalog,
        stack: Some(StackState {
            enabled: true,
            members: vec!["primary".into(), "secondary".into()],
            keys: [("secondary".into(), "secondary".into())].into(),
        }),
    }
}

fn read_json(path: PathBuf) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

#[test]
fn aggregation_t31_publish_and_restore_official_or_saved_gateway() {
    for saved in [None, Some(profile("https://saved.test"))] {
        let fixture = Fixture::new();
        fixture.install();
        let mut files = fixture.files();
        if let Some(saved) = &saved {
            files
                .apply(&fixture.tx, &fixture.tenant, &fixture.state(), "saved", saved, || true)
                .unwrap();
        }
        drop(files);
        fs::write(fixture.profile_path(), br#"{"userAutoMode":true}"#).unwrap();
        fs::write(
            fixture.path(META),
            br#"{"entries":[{"id":"user","name":"Personal"}],"appliedId":"user"}"#,
        )
        .unwrap();
        let catalog = catalog();
        let gateway = profile("http://127.0.0.1:31911/claude-desktop");

        fixture.run(&fixture.state(), aggregate(&gateway, &catalog), || true).unwrap();
        assert_eq!(read_json(fixture.path(NORMAL))["deploymentMode"], "3p");
        assert_eq!(read_json(fixture.path(THIRD))["deploymentMode"], "3p");
        assert_eq!(read_json(fixture.path(META))["appliedId"], PROFILE_ID);
        let projected = read_json(fixture.profile_path());
        assert_eq!(projected["modelDiscoveryEnabled"], true);
        assert_eq!(projected["inferenceModels"][0]["prefer1m"], true);
        for (projection, model) in
            projected["inferenceModels"].as_array().unwrap().iter().zip(catalog.models())
        {
            assert_eq!(projection["name"], model.id);
            assert_eq!(projection["labelOverride"], model.label);
        }
        assert_eq!(projected["inferenceModels"].as_array().unwrap().len(), 2);
        assert_eq!(
            fs::metadata(fixture.profile_path()).unwrap().permissions().mode() & 0o777,
            0o600
        );
        let status = fixture.status();
        assert_eq!(status.mode, DesktopMode::Aggregation);
        assert_eq!(status.applied_provider_id.as_deref(), Some("primary"));
        assert!(status.configured);
        assert!(!status.recovery_pending);
        let active = fixture.state();
        assert_eq!(active.mode.as_ref().unwrap().contract.as_ref().unwrap().version, 2);

        fixture.run(&active, DesktopTarget::Restore(saved.as_ref()), || true).unwrap();
        let restored = read_json(fixture.profile_path());
        assert_eq!(restored["userAutoMode"], true);
        assert!(restored.get("inferenceModels").is_none());
        assert!(restored.get("modelDiscoveryEnabled").is_none());
        let state = fixture.state();
        assert!(!state.stack.enabled);
        assert_eq!(state.stack.members, active.stack.members);
        assert_eq!(state.stack.keys, active.stack.keys);
        assert!(state.pointer == active.pointer);
        let status = fixture.status();
        let meta = read_json(fixture.path(META));
        assert_eq!(meta["entries"][0]["id"], "user");
        if saved.is_some() {
            assert_eq!(status.mode, DesktopMode::DirectGateway);
            assert_eq!(status.applied_provider_id.as_deref(), Some("saved"));
            assert_eq!(restored["inferenceGatewayBaseUrl"], "https://saved.test");
            assert_eq!(meta["appliedId"], PROFILE_ID);
        } else {
            assert_eq!(status.mode, DesktopMode::Official);
            assert!(!status.configured);
            assert!(restored.get("inferenceGatewayApiKey").is_none());
            assert_eq!(meta["appliedId"], "user");
            assert_eq!(meta["entries"].as_array().unwrap().len(), 1);
            assert_eq!(read_json(fixture.path(NORMAL))["deploymentMode"], "1p");
            assert_eq!(read_json(fixture.path(THIRD))["deploymentMode"], "1p");
        }
    }
}

#[test]
fn aggregation_t31_status_and_rejections_do_not_provision_client_files() {
    let fixture = Fixture::new();
    let catalog = catalog();
    let gateway = profile("http://127.0.0.1:31911/claude-desktop");
    assert_eq!(fixture.status().installation, DesktopInstallation::NotInstalled);
    assert_eq!(
        fixture.run(&fixture.state(), aggregate(&gateway, &catalog), || true),
        Err(DesktopPublicationError::UnsupportedTarget)
    );
    let unsupported = DesktopInstallation::from(&AppDesktopObservation {
        supported: false,
        app_bundles: None,
        config_directories: vec![],
    });
    let unsupported_status =
        get_status(fixture.home.path(), unsupported, None, &fixture.tenant).unwrap();
    assert_eq!(unsupported_status.installation, DesktopInstallation::Unsupported);
    assert_eq!(
        run(
            fixture.home.path(),
            unsupported,
            &fixture.tx,
            &fixture.tenant,
            &fixture.state(),
            aggregate(&gateway, &catalog),
            || true,
        ),
        Err(DesktopPublicationError::UnsupportedTarget)
    );

    fixture.install();
    assert_eq!(fixture.status().installation, DesktopInstallation::Installed);
    assert_eq!(fixture.status().mode, DesktopMode::Official);
    assert_eq!(
        fixture.run(&fixture.state(), aggregate(&gateway, &catalog), || false),
        Err(DesktopPublicationError::RevisionConflict)
    );
    let remote = profile("https://remote.test");
    assert_eq!(
        fixture.run(&fixture.state(), aggregate(&remote, &catalog), || true),
        Err(DesktopPublicationError::InvalidConfiguration)
    );
    assert!(!fixture.home.path().join("Library").exists());
}

#[test]
fn aggregation_t31_malformed_metadata_and_late_revision_preserve_preimage() {
    let fixture = Fixture::new();
    fixture.install();
    drop(fixture.files());
    fs::write(fixture.path(NORMAL), br#"{"deploymentMode":"1p"}"#).unwrap();
    fs::write(fixture.path(META), br#"{"entries":{}}"#).unwrap();
    let gateway = profile("http://127.0.0.1:31911/claude-desktop");
    let catalog = catalog();
    let state = fixture.state();
    assert_eq!(
        fixture.run(&state, aggregate(&gateway, &catalog), || true),
        Err(DesktopPublicationError::InvalidConfiguration)
    );
    assert_eq!(fs::read(fixture.path(META)).unwrap(), br#"{"entries":{}}"#);

    fs::remove_file(fixture.path(META)).unwrap();
    let checks = std::cell::Cell::new(0);
    assert_eq!(
        fixture.run(&state, aggregate(&gateway, &catalog), || {
            checks.set(checks.get() + 1);
            checks.get() < 3
        }),
        Err(DesktopPublicationError::RevisionConflict)
    );
    assert!(fixture.status().recovery_pending);
    assert!(fixture.tx.recover_on_startup()[0].1.is_ok());
    assert!(!fixture.status().recovery_pending);
    assert!(fixture.state() == state);
    assert_eq!(fs::read(fixture.path(NORMAL)).unwrap(), br#"{"deploymentMode":"1p"}"#);
    assert!(!fixture.path(THIRD).exists());
    assert!(!fixture.profile_path().exists());
    assert!(!fixture.path(META).exists());
}

#[test]
fn aggregation_t31_partial_publication_reports_failure_and_recovers_as_one_intent() {
    let fixture = Fixture::new();
    fixture.install();
    let gateway = profile("http://127.0.0.1:31911/claude-desktop");
    let catalog = catalog();
    let state = fixture.state();
    HOOK.with_borrow_mut(|hook| {
        *hook = Some(Box::new(|_, _, phase| {
            if phase == Phase::FilePublished { Err(WriteError::Io) } else { Ok(()) }
        }));
    });
    let result = fixture.run(&state, aggregate(&gateway, &catalog), || true);
    HOOK.with_borrow_mut(|hook| *hook = None);
    assert_eq!(result, Err(DesktopPublicationError::TargetFailed));
    assert!(fixture.state() == state);
    assert!(fixture.status().recovery_pending);
    assert_eq!(
        fixture.run(&state, aggregate(&gateway, &catalog), || true),
        Err(DesktopPublicationError::RecoveryPending)
    );
    let other = TenantScope::new("platform", "other").unwrap();
    assert_eq!(
        get_status(fixture.home.path(), fixture.installation(), Some(&fixture.tx), &other),
        Err(DesktopPublicationError::Forbidden)
    );
    assert!(fixture.tx.recover_on_startup()[0].1.is_ok());
    let status = fixture.status();
    assert!(!status.recovery_pending);
    assert_eq!(status.mode, DesktopMode::Aggregation);
    assert_eq!(read_json(fixture.path(NORMAL))["deploymentMode"], "3p");
    assert_eq!(read_json(fixture.path(THIRD))["deploymentMode"], "3p");
    assert_eq!(read_json(fixture.path(META))["appliedId"], PROFILE_ID);
}
