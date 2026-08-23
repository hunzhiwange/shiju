//! 验证网关密钥的租户隔离、原子替换、哈希与展示掩码。

use super::*;
use crate::global_database::GlobalDatabase;
use crate::global_database::test_support::PostgresTestDatabase;
use vw_api_types::id::TenantScope;

fn skey(name: &str, hash: &str) -> GatewaySkey {
    GatewaySkey {
        enabled: true,
        skey: None,
        skey_hash: hash.to_string(),
        masked_skey: format!("{name}-masked"),
        name: name.to_string(),
        expires_at: None,
    }
}

#[test]
fn gateway_skey_store_isolates_tenants_and_replaces_only_target_scope() -> Result<()> {
    for_each_gateway_skey_store_provider(gateway_skey_store_isolation)
}

fn gateway_skey_store_isolation(store: &GatewaySkeyStore) -> Result<()> {
    let tenant_a = TenantScope::new("platform-a", "company-a")?;
    let tenant_b = TenantScope::new("platform-b", "company-b")?;

    assert!(store.load(&tenant_a)?.is_empty());
    store.replace_all(
        &tenant_a,
        &[skey("a-one", &"a".repeat(64)), skey("a-two", &"b".repeat(64))],
    )?;
    store.replace_all(&tenant_b, &[skey("b-one", &"c".repeat(64))])?;

    store.replace_all(&tenant_a, &[skey("a-replaced", &"d".repeat(64))])?;

    let tenant_a_skeys = store.load(&tenant_a)?;
    let tenant_b_skeys = store.load(&tenant_b)?;
    assert_eq!(tenant_a_skeys.len(), 1);
    assert_eq!(tenant_a_skeys[0].name, "a-replaced");
    assert_eq!(tenant_b_skeys.len(), 1);
    assert_eq!(tenant_b_skeys[0].name, "b-one");
    Ok(())
}

#[test]
fn gateway_skey_store_replace_all_rolls_back_target_on_insert_failure() -> Result<()> {
    for_each_gateway_skey_store_provider(gateway_skey_store_replace_all_rollback)
}

fn gateway_skey_store_replace_all_rollback(store: &GatewaySkeyStore) -> Result<()> {
    let tenant = TenantScope::new("platform-a", "company-a")?;
    let original = skey("original", &"a".repeat(64));
    store.replace_all(&tenant, std::slice::from_ref(&original))?;

    let duplicate_hash = "b".repeat(64);
    let error = store
        .replace_all(
            &tenant,
            &[skey("duplicate-one", &duplicate_hash), skey("duplicate-two", &duplicate_hash)],
        )
        .expect_err("duplicate hashes must roll back the replacement");

    assert!(error.to_string().contains("replace tenant gateway skeys"));
    assert_eq!(store.load(&tenant)?, vec![original]);
    Ok(())
}

#[test]
fn gateway_skey_store_hashes_and_masks_raw_skeys_without_returning_plaintext() -> Result<()> {
    for_each_gateway_skey_store_provider(gateway_skey_store_normalizes_raw_skey)
}

fn gateway_skey_store_normalizes_raw_skey(store: &GatewaySkeyStore) -> Result<()> {
    let tenant = TenantScope::default();
    let raw_skey = "sk-1234567890abcdef1234567890abcdef111111111";
    let input = GatewaySkey {
        enabled: true,
        skey: Some(raw_skey.to_string()),
        skey_hash: "f".repeat(64),
        masked_skey: raw_skey.to_string(),
        name: format!("credential-{raw_skey}"),
        expires_at: Some("  2026-12-31T23:59:59Z  ".to_string()),
    };

    store.replace_all(&tenant, &[input])?;

    let loaded = store.load(&tenant)?;
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].skey_hash, hash_skey(raw_skey));
    assert_eq!(loaded[0].masked_skey, "sk-1234567890abc***************111111111");
    assert_eq!(loaded[0].name, "sk-1***1111");
    assert_eq!(loaded[0].expires_at.as_deref(), Some("2026-12-31T23:59:59Z"));
    assert!(loaded[0].skey.is_none());
    Ok(())
}

#[test]
fn gateway_skey_store_sqlite_does_not_persist_plaintext() {
    let temp = tempfile::tempdir().expect("temp dir");
    let database = GlobalDatabase::for_home(temp.path()).expect("global database");
    let database_path = database.path().to_path_buf();
    let store = GatewaySkeyStore::new(database).expect("gateway skey store");
    let tenant = TenantScope::default();
    let raw_skey = "sk-1234567890abcdef1234567890abcdef111111111";
    let input = GatewaySkey {
        enabled: true,
        skey: Some(raw_skey.to_string()),
        skey_hash: String::new(),
        masked_skey: String::new(),
        name: String::new(),
        expires_at: None,
    };

    store.replace_all(&tenant, &[input]).expect("replace raw skey");

    let database_bytes = std::fs::read(database_path).expect("read shared database");
    assert!(!database_bytes.windows(raw_skey.len()).any(|window| window == raw_skey.as_bytes()));
}

#[test]
fn gateway_skey_store_ignores_conflicting_legacy_sqlite() {
    let temp = tempfile::tempdir().expect("temp dir");
    let legacy_path =
        vw_config_types::paths::home_config_dir(temp.path()).join("gateway").join("skeys.sqlite");
    std::fs::create_dir_all(legacy_path.parent().expect("legacy parent"))
        .expect("create legacy parent");
    let legacy = rusqlite::Connection::open(&legacy_path).expect("legacy sqlite");
    legacy
        .execute_batch(
            r#"
            CREATE TABLE gateway_skeys (
                skey_hash TEXT PRIMARY KEY NOT NULL,
                masked_skey TEXT NOT NULL,
                name TEXT NOT NULL,
                enabled INTEGER NOT NULL,
                expires_at TEXT,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            INSERT INTO gateway_skeys (skey_hash, masked_skey, name, enabled)
            VALUES ('aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
                    'legacy-mask', 'legacy-conflict', 1);
            "#,
        )
        .expect("seed legacy sqlite");
    drop(legacy);

    let database = GlobalDatabase::for_home(temp.path()).expect("global database");
    let store = GatewaySkeyStore::new(database).expect("gateway skey store");
    let tenant = TenantScope::default();
    store.replace_all(&tenant, &[skey("shared-only", &"b".repeat(64))]).expect("seed shared store");

    let loaded = store.load(&tenant).expect("load shared store");
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].name, "shared-only");
    assert!(legacy_path.exists(), "legacy input must remain untouched");
}

#[test]
fn normalize_skey_ignores_blank_hash_without_raw_secret() {
    let input = skey("blank", "   ");

    assert_eq!(normalize_skey(&input).unwrap(), None);
}

#[test]
fn normalize_skey_rejects_malformed_hash_and_expiry() {
    let short_hash_error =
        normalize_skey(&skey("short", "abc")).expect_err("short hashes must be rejected");
    assert!(short_hash_error.to_string().contains("SHA-256 hex hash"));

    let non_hex_error = normalize_skey(&skey("non-hex", &"g".repeat(64)))
        .expect_err("non-hex hashes must be rejected");
    assert!(non_hex_error.to_string().contains("SHA-256 hex hash"));

    let mut invalid_expiry = skey("expiry", &"A".repeat(64));
    invalid_expiry.expires_at = Some("not-rfc3339".into());
    let expiry_error =
        normalize_skey(&invalid_expiry).expect_err("invalid expiries must be rejected");
    assert!(expiry_error.to_string().contains("expiry must be RFC3339"));
}

#[test]
fn normalize_skey_preserves_safe_metadata_without_plaintext() {
    let input = GatewaySkey {
        enabled: false,
        skey: None,
        skey_hash: format!("  {}  ", "A".repeat(64)),
        masked_skey: "  masked-value  ".into(),
        name: "  deployment key  ".into(),
        expires_at: Some("   ".into()),
    };

    let normalized = normalize_skey(&input).unwrap().expect("valid metadata");

    assert!(!normalized.enabled);
    assert_eq!(normalized.skey_hash, "a".repeat(64));
    assert_eq!(normalized.masked_skey, "masked-value");
    assert_eq!(normalized.name, "deployment key");
    assert_eq!(normalized.expires_at, None);
    assert_eq!(normalized.skey, None);
}

#[test]
fn normalize_skey_masks_short_raw_secret_and_keeps_safe_name() {
    let input = GatewaySkey {
        enabled: true,
        skey: Some("  short-secret  ".into()),
        skey_hash: String::new(),
        masked_skey: String::new(),
        name: "  operator key  ".into(),
        expires_at: None,
    };

    let normalized = normalize_skey(&input).unwrap().expect("raw secret is normalized");

    assert_eq!(normalized.skey_hash, hash_skey("short-secret"));
    assert_eq!(normalized.masked_skey, "************");
    assert_eq!(normalized.name, "operator key");
    assert_eq!(normalized.skey, None);
}

#[test]
fn mask_skey_for_display_switches_at_long_secret_boundary() {
    assert_eq!(mask_skey_for_display(&"x".repeat(25)), "*".repeat(25));
    assert_eq!(
        mask_skey_for_display("abcdefghijklmnopQqrstuvwxyz"),
        "abcdefghijklmnop***************rstuvwxyz"
    );
}

fn for_each_gateway_skey_store_provider(
    mut check: impl FnMut(&GatewaySkeyStore) -> Result<()>,
) -> Result<()> {
    let sqlite_home = tempfile::tempdir()?;
    let sqlite_store = GatewaySkeyStore::new(GlobalDatabase::for_home(sqlite_home.path())?)?;
    check(&sqlite_store)?;

    let postgres_fixture = crate::global_database::test_support::postgres_test_database!();
    let postgres_store = GatewaySkeyStore::new(postgres_fixture.database().clone())?;
    check(&postgres_store)?;
    postgres_fixture.cleanup()?;
    Ok(())
}
