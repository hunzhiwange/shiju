use super::*;
use crate::agent_apps::auth_accounts::{AuthAccountMetadata, AuthAccountScope, AuthAccountStore};
use serde_json::{Value, json};
use std::{fs, os::unix::fs::PermissionsExt, process::Command, sync::Arc};
use vw_agent_apps::{
    auth::{
        credential_file::{AccountCredential, CredentialFileStore, KimiCredential},
        kimi_account::KimiNativeTarget,
    },
    providers::{
        kimi::{self, KimiFiles},
        live::commit::{ExecutionHostId, ProviderTransactions},
    },
};
use vw_api_types::{
    TimestampMs,
    agent_apps_auth::{AuthErrorCode, AuthRegion, AuthService},
    app_providers::kimi::KimiActivation,
};

const HOST: &str = "host";
const PROVIDER: &str = "managed:kimi-code";
const ACCOUNT: &str = "account";
const FUTURE: i64 = 4_102_444_800;
const CONFIG: &str = r#"# preserve this comment
default_model = "original"

[providers.other]
type = "kimi"

[providers."managed:kimi-code"]
type = "kimi"
base_url = "https://api.kimi.ai/coding/v1"
[providers."managed:kimi-code".oauth]
storage = "file"
key = "oauth/kimi-code-fixture"
oauth_host = "https://auth.kimi.ai"

[models.original]
provider = "other"
model = "other-model"

[models.chosen]
provider = "managed:kimi-code"
model = "kimi-for-coding"
max_context_size = 32000
"#;

struct Fixture {
    home: tempfile::TempDir,
    root: PathBuf,
    tenant: TenantScope,
    store: AppProviderStore,
    tx: Arc<ProviderTransactions>,
}

impl Fixture {
    fn new() -> Self {
        let home = tempfile::tempdir().unwrap();
        let database = GlobalDatabase::for_home(home.path()).unwrap();
        Self::with_database(home, database)
    }

    fn with_database(home: tempfile::TempDir, database: GlobalDatabase) -> Self {
        let root = home.path().join("private");
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        let store = AppProviderStore::new(database).unwrap();
        let tenant = TenantScope::new("platform", "company").unwrap();
        let tx = Arc::new(
            ProviderTransactions::new(&root, ExecutionHostId::new(HOST).unwrap()).unwrap(),
        );
        let fixture = Self { home, root, tenant, store, tx };
        let config = fixture.config();
        fs::create_dir_all(config.with_file_name("credentials")).unwrap();
        fs::create_dir_all(config.with_file_name("oauth")).unwrap();
        fs::write(&config, CONFIG).unwrap();
        let profile =
            kimi::declarations(Some(CONFIG.as_bytes())).unwrap().remove(PROVIDER).unwrap();
        fixture.store.kimi_save(&fixture.tenant, PROVIDER, &profile, None, None).unwrap();
        fixture.native_grant("fixture");
        fixture
    }

    fn scope(&self) -> AuthAccountScope<'_> {
        AuthAccountScope {
            tenant: &self.tenant,
            principal: "owner",
            host: HOST,
            service: AuthService::KimiOauth,
        }
    }

    fn target(&self) -> KimiNativeTarget<'_> {
        KimiNativeTarget { home: self.home.path(), transactions: &self.tx, provider_id: PROVIDER }
    }

    fn config(&self) -> PathBuf {
        self.home.path().join(".kimi-code/config.toml")
    }

    fn native_grant(&self, prefix: &str) {
        let path = self.config().with_file_name("credentials").join("kimi-code-fixture.json");
        fs::write(
            &path,
            json!({
                "access_token": format!("{prefix}-access"),
                "refresh_token": format!("{prefix}-refresh"),
                "expires_at": FUTURE,
            })
            .to_string(),
        )
        .unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
    }

    fn register(&self, scope: AuthAccountScope<'_>, id: &str, prefix: &str, region: AuthRegion) {
        let accounts = AuthAccountStore::new(self.store.database.clone(), &self.root).unwrap();
        let mut metadata = AuthAccountMetadata::new("fixture-identity", TimestampMs(1)).unwrap();
        metadata.region = Some(region);
        accounts.insert_metadata(&scope, id, &metadata).unwrap();
        CredentialFileStore::new(&self.root)
            .unwrap()
            .lease_kimi(scope, id, true)
            .unwrap()
            .save(&AccountCredential::KimiOauth(KimiCredential {
                access_token: format!("{prefix}-access"),
                refresh_token: format!("{prefix}-refresh"),
                expires_at: TimestampMs(FUTURE * 1_000),
                device_id: "d8c8a37b-175c-4e59-9501-07351b0da278".into(),
                region,
            }))
            .unwrap();
    }

    async fn bind(
        &self,
        scope: AuthAccountScope<'_>,
        expected: Option<&str>,
        account: &str,
    ) -> Result<(), AuthErrorCode> {
        self.store.bind_kimi_account(scope, &self.root, &self.target(), expected, account).await
    }

    fn binding(&self, scope: AuthAccountScope<'_>) -> Result<Option<String>, AuthErrorCode> {
        self.store.kimi_account_binding(scope, &self.root, &self.target())
    }
}

#[tokio::test]
async fn provider_kimi_binding_commits_and_survives_ordinary_edits() {
    let f = Fixture::new();
    let scope = f.scope();
    f.register(scope, ACCOUNT, "fixture", AuthRegion::Global);
    // A managed account exists, but an older host state has no explicit binding.
    assert_eq!(f.binding(scope), Ok(None));
    assert_eq!(
        f.store.use_kimi_account_binding(scope, &f.root, f.target()).await,
        Err(AuthErrorCode::NotFound)
    );
    f.bind(scope, None, ACCOUNT).await.unwrap();
    assert_eq!(
        f.store.with_kimi_provider_unreferenced(scope, &f.root, ACCOUNT, |_| Ok(())),
        Err(AuthErrorCode::AccountInUse)
    );
    assert_eq!(f.binding(scope), Ok(Some(ACCOUNT.into())));
    let selected = CONFIG.replacen("\"original\"", "\"chosen\"", 1);
    assert_eq!(fs::read_to_string(f.config()).unwrap(), selected);

    let before = f.store.kimi_record(&f.tenant, PROVIDER).unwrap().unwrap();
    let mut profile =
        kimi::declarations(Some(selected.as_bytes())).unwrap().remove(PROVIDER).unwrap();
    profile.name = "New label".into();
    profile.declaration["models"]["chosen"]["display_name"] = json!("New model label");
    f.store.kimi_save(&f.tenant, PROVIDER, &profile, Some(&before), None).unwrap();
    let mut files = KimiFiles::open(f.home.path()).unwrap();
    let (name, pre) = files.snapshot().unwrap();
    let next =
        kimi::patch(pre.as_deref(), PROVIDER, Some(&profile.declaration), KimiActivation::IfMember)
            .unwrap();
    files
        .apply(
            &f.tx,
            &f.tenant,
            &f.tx.state(AppId::Kimi).unwrap(),
            name,
            pre.as_deref(),
            next.clone(),
            || true,
        )
        .unwrap();
    drop(files);
    f.bind(scope, None, ACCOUNT).await.unwrap();
    assert_eq!(f.binding(scope), Ok(Some(ACCOUNT.into())));
    assert_eq!(fs::read(f.config()).unwrap(), next);
    assert!(
        !serde_json::to_string(&f.tx.state(AppId::Kimi).unwrap())
            .unwrap()
            .contains("fixture-access")
    );
}

#[tokio::test]
async fn provider_kimi_binding_rejects_foreign_service_grant_and_stale_requests() {
    let f = Fixture::new();
    let scope = f.scope();
    f.register(scope, ACCOUNT, "fixture", AuthRegion::Global);
    let foreign = TenantScope::new("platform", "foreign").unwrap();
    let profile = kimi::declarations(Some(CONFIG.as_bytes())).unwrap().remove(PROVIDER).unwrap();
    f.store.kimi_save(&foreign, PROVIDER, &profile, None, None).unwrap();
    for (scope, expected) in [
        (AuthAccountScope { tenant: &foreign, ..scope }, AuthErrorCode::NotFound),
        (AuthAccountScope { principal: "foreign", ..scope }, AuthErrorCode::NotFound),
        (AuthAccountScope { host: "foreign", ..scope }, AuthErrorCode::Forbidden),
        (
            AuthAccountScope { service: AuthService::CodexOauth, ..scope },
            AuthErrorCode::Unsupported,
        ),
    ] {
        assert_eq!(f.bind(scope, None, ACCOUNT).await, Err(expected));
    }
    f.register(scope, "other", "different", AuthRegion::Global);
    f.register(scope, "region", "fixture", AuthRegion::MainlandCn);
    assert_eq!(f.bind(scope, None, "region").await, Err(AuthErrorCode::IdentityMismatch));
    let mut altered: Value = profile.declaration.clone();
    altered["provider"]["base_url"] = json!("https://untrusted.invalid");
    let altered_profile =
        vw_api_types::app_providers::kimi::KimiProfile { declaration: altered, ..profile.clone() };
    let row = f.store.kimi_record(&f.tenant, PROVIDER).unwrap().unwrap();
    let altered_row =
        f.store.kimi_save(&f.tenant, PROVIDER, &altered_profile, Some(&row), None).unwrap();
    assert_eq!(f.bind(scope, None, ACCOUNT).await, Err(AuthErrorCode::Conflict));
    assert_eq!(fs::read_to_string(f.config()).unwrap(), CONFIG);
    f.store.kimi_save(&f.tenant, PROVIDER, &profile, Some(&altered_row), None).unwrap();

    f.bind(scope, None, ACCOUNT).await.unwrap();
    assert_eq!(
        f.binding(AuthAccountScope { principal: "foreign", ..scope }),
        Err(AuthErrorCode::Forbidden)
    );
    f.native_grant("different");
    assert_eq!(f.bind(scope, None, "other").await, Err(AuthErrorCode::Conflict));
    assert_eq!(f.binding(scope), Ok(Some(ACCOUNT.into())));
    f.bind(scope, Some(ACCOUNT), "other").await.unwrap();
    assert_eq!(f.binding(scope), Ok(Some("other".into())));
}

#[tokio::test]
async fn provider_kimi_binding_recovers_publication_and_preserves_external_edits() {
    for external in [false, true] {
        let f = Fixture::new();
        let scope = f.scope();
        f.register(scope, ACCOUNT, "fixture", AuthRegion::Global);
        let state = f.root.join("provider-live/aG9zdA/kimi.state.json");
        // Older states must deserialize without a binding.
        fs::write(&state, r#"{"pointer":null,"mode":null,"mode_tenant":null}"#).unwrap();
        fs::set_permissions(&state, fs::Permissions::from_mode(0o600)).unwrap();
        assert_eq!(f.binding(scope), Ok(None));
        chflags("uchg", &state);
        let result = f.bind(scope, None, ACCOUNT).await;
        chflags("nouchg", &state);
        assert_eq!(result, Err(AuthErrorCode::StorageUnavailable));
        assert_eq!(f.binding(scope), Err(AuthErrorCode::Conflict));
        assert_eq!(
            fs::read_to_string(f.config()).unwrap(),
            CONFIG.replacen("\"original\"", "\"chosen\"", 1)
        );
        let edited = format!("# external edit\n{CONFIG}");
        if external {
            fs::write(f.config(), &edited).unwrap();
        }
        assert_eq!(f.bind(scope, None, ACCOUNT).await, Err(AuthErrorCode::Conflict));
        assert!(f.tx.pending(AppId::Kimi).unwrap().is_none());
        if external {
            assert_eq!(fs::read_to_string(f.config()).unwrap(), edited);
            assert_eq!(f.binding(scope), Ok(None));
        } else {
            assert_eq!(f.binding(scope), Ok(Some(ACCOUNT.into())));
        }
        f.bind(scope, None, ACCOUNT).await.unwrap();
        assert_eq!(f.binding(scope), Ok(Some(ACCOUNT.into())));
    }
}

#[test]
fn provider_kimi_binding_scope_postgres() {
    use crate::global_database::test_support::PostgresTestDatabase;

    let database = PostgresTestDatabase::from_environment().unwrap();
    let f = Fixture::with_database(tempfile::tempdir().unwrap(), database.database().clone());
    let scope = f.scope();
    let foreign = TenantScope::new("platform", "other-company").unwrap();
    let profile = kimi::declarations(Some(CONFIG.as_bytes())).unwrap().remove(PROVIDER).unwrap();
    f.store.kimi_save(&foreign, PROVIDER, &profile, None, None).unwrap();
    f.register(scope, ACCOUNT, "fixture", AuthRegion::Global);
    tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
        assert_eq!(f.binding(scope), Ok(None));
        assert_eq!(
            f.store.use_kimi_account_binding(scope, &f.root, f.target()).await,
            Err(AuthErrorCode::NotFound)
        );
        assert_eq!(
            f.bind(AuthAccountScope { tenant: &foreign, ..scope }, None, ACCOUNT).await,
            Err(AuthErrorCode::NotFound)
        );
        assert_eq!(
            f.bind(AuthAccountScope { host: "other-host", ..scope }, None, ACCOUNT).await,
            Err(AuthErrorCode::Forbidden)
        );
        assert_eq!(
            f.bind(AuthAccountScope { service: AuthService::CodexOauth, ..scope }, None, ACCOUNT)
                .await,
            Err(AuthErrorCode::Unsupported)
        );
        assert_eq!(fs::read_to_string(f.config()).unwrap(), CONFIG);
        f.bind(scope, None, ACCOUNT).await.unwrap();
        assert_eq!(f.binding(scope), Ok(Some(ACCOUNT.into())));
        assert_eq!(
            f.binding(AuthAccountScope { principal: "other-owner", ..scope }),
            Err(AuthErrorCode::Forbidden)
        );
    });
    drop(f);
    database.cleanup().unwrap();
}

fn chflags(flag: &str, path: &Path) {
    assert!(Command::new("/usr/bin/chflags").arg(flag).arg(path).status().unwrap().success());
}
