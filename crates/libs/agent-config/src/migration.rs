//! 用户级基础配置到共享数据库的一次性迁移。

use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use serde_json::Value;
use config_aggregate::Config;
use vw_api_transport::id::TenantScope;

use agent_database::GlobalDatabase;
use agent_database::config::DatabaseProvider;

/// 尚未迁移的配置编解码与表初始化由组合层提供；不持有宿主状态。
///
/// 配置 schema 和存储迁入本包后，组合层可直接绑定其真实实现。
pub struct MigrationServices {
    pub parse_config_contents: fn(&str) -> Result<Config>,
    pub decrypt_config_secrets: fn(&mut Config, &Path) -> Result<()>,
    pub encrypt_config_secrets: fn(&mut Config) -> Result<()>,
    pub validate_config: fn(&Config) -> Result<()>,
    pub initialize_store: fn(GlobalDatabase) -> Result<()>,
}

const SOURCE_FILE: &str = "vibewindow.json";
const BACKUP_DIRECTORY: &str = "database/migration-backups/system-config";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationOutcome {
    NoSource,
    Migrated,
    AlreadyImported,
}

/// 将指定 home 下的旧用户配置迁入共享库的 `local/local` 行。
///
/// 数据库写入和回读核验在同一事务内完成。只有提交成功且语义核验一致后，来源文件才会移动到迁移备份目录。
pub fn migrate_user_config(home: &Path, services: &MigrationServices) -> Result<MigrationOutcome> {
    let source_path = config_runtime_paths::home_config_dir(home).join(SOURCE_FILE);
    if !source_path.exists() {
        return Ok(MigrationOutcome::NoSource);
    }
    let database = GlobalDatabase::for_home(home)?;
    migrate_user_config_with_database(home, database, services)
}

pub fn migrate_user_config_with_database(
    home: &Path,
    database: GlobalDatabase,
    services: &MigrationServices,
) -> Result<MigrationOutcome> {
    let config_dir = config_runtime_paths::home_config_dir(home);
    let source_path = config_dir.join(SOURCE_FILE);
    if !source_path.exists() {
        return Ok(MigrationOutcome::NoSource);
    }

    let source_contents = fs::read_to_string(&source_path)
        .with_context(|| format!("read user system config: {}", source_path.display()))?;
    let source_config = decode_and_validate(&source_contents, &source_path, &config_dir, services)
        .context("parse and validate user system config")?;
    let source_semantics = semantic_value(&source_config)?;
    let payload_json = encrypted_payload(&source_config, services)?;

    (services.initialize_store)(database.clone())?;
    let tenant = TenantScope::default();
    let outcome = match database.provider() {
        DatabaseProvider::Sqlite => migrate_in_sqlite_transaction(
            &database,
            &tenant,
            &payload_json,
            &source_semantics,
            &source_path,
            &config_dir,
            services,
        )?,
        DatabaseProvider::Postgres => migrate_in_postgres_transaction(
            &database,
            &tenant,
            &payload_json,
            &source_semantics,
            &source_path,
            &config_dir,
            services,
        )?,
    };

    move_source_to_backup(&source_path, &config_dir)?;
    tracing::info!(
        source = %source_path.display(),
        status = ?outcome,
        "User system config migration completed"
    );
    Ok(outcome)
}

fn migrate_in_sqlite_transaction(
    database: &GlobalDatabase,
    tenant: &TenantScope,
    payload_json: &str,
    source_semantics: &Value,
    source_path: &Path,
    config_dir: &Path,
    services: &MigrationServices,
) -> Result<MigrationOutcome> {
    database.with_sqlite_transaction(TransactionBehavior::Immediate, |transaction| {
        let existing = transaction
            .query_row(
                "SELECT payload_json FROM system_configs
                 WHERE platform_id = ?1 AND company_id = ?2",
                params![tenant.platform_id().as_ref(), tenant.company_id().as_ref()],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .context("read local SQLite system config during migration")?;

        if let Some(existing_payload) = existing {
            verify_existing_payload(
                &existing_payload,
                source_semantics,
                source_path,
                config_dir,
                services,
            )?;
            return Ok(MigrationOutcome::AlreadyImported);
        }

        transaction
            .execute(
                "INSERT INTO system_configs (
                     platform_id, company_id, payload_json, updated_at_ms
                 ) VALUES (?1, ?2, ?3, ?4)",
                params![
                    tenant.platform_id().as_ref(),
                    tenant.company_id().as_ref(),
                    payload_json,
                    current_timestamp_ms()?,
                ],
            )
            .context("insert migrated local SQLite system config")?;

        let inserted_payload: String = transaction
            .query_row(
                "SELECT payload_json FROM system_configs
                 WHERE platform_id = ?1 AND company_id = ?2",
                params![tenant.platform_id().as_ref(), tenant.company_id().as_ref()],
                |row| row.get(0),
            )
            .context("read back migrated local SQLite system config")?;
        verify_inserted_payload(
            &inserted_payload,
            source_semantics,
            source_path,
            config_dir,
            services,
        )?;
        Ok(MigrationOutcome::Migrated)
    })
}

fn migrate_in_postgres_transaction(
    database: &GlobalDatabase,
    tenant: &TenantScope,
    payload_json: &str,
    source_semantics: &Value,
    source_path: &Path,
    config_dir: &Path,
    services: &MigrationServices,
) -> Result<MigrationOutcome> {
    let mut connection = database.connect_postgres()?;
    let mut transaction =
        connection.transaction().context("begin PostgreSQL system config migration")?;
    let existing_payload = transaction
        .query_opt(
            "SELECT payload_json FROM system_configs
             WHERE platform_id = $1 AND company_id = $2
             FOR UPDATE",
            &[&tenant.platform_id().as_ref(), &tenant.company_id().as_ref()],
        )
        .context("read local PostgreSQL system config during migration")?
        .map(|row| row.try_get::<_, String>(0))
        .transpose()
        .context("decode local PostgreSQL system config during migration")?;

    let outcome = if let Some(existing_payload) = existing_payload {
        verify_existing_payload(
            &existing_payload,
            source_semantics,
            source_path,
            config_dir,
            services,
        )?;
        MigrationOutcome::AlreadyImported
    } else {
        let timestamp_ms = current_timestamp_ms()?;
        let changed = transaction
            .execute(
                "INSERT INTO system_configs (
                     platform_id, company_id, payload_json, updated_at_ms
                 ) VALUES ($1, $2, $3, $4)",
                &[
                    &tenant.platform_id().as_ref(),
                    &tenant.company_id().as_ref(),
                    &payload_json,
                    &timestamp_ms,
                ],
            )
            .context("insert migrated local PostgreSQL system config")?;
        if changed != 1 {
            bail!("PostgreSQL system config migration insert affected an unexpected row count");
        }

        let inserted_payload: String = transaction
            .query_one(
                "SELECT payload_json FROM system_configs
                 WHERE platform_id = $1 AND company_id = $2",
                &[&tenant.platform_id().as_ref(), &tenant.company_id().as_ref()],
            )
            .context("read back migrated local PostgreSQL system config")?
            .try_get(0)
            .context("decode migrated local PostgreSQL system config")?;
        verify_inserted_payload(
            &inserted_payload,
            source_semantics,
            source_path,
            config_dir,
            services,
        )?;
        MigrationOutcome::Migrated
    };

    transaction.commit().context("commit PostgreSQL system config migration")?;
    Ok(outcome)
}

fn verify_existing_payload(
    existing_payload: &str,
    source_semantics: &Value,
    source_path: &Path,
    config_dir: &Path,
    services: &MigrationServices,
) -> Result<()> {
    let existing_config = decode_and_validate(existing_payload, source_path, config_dir, services)
        .context("verify existing local system config")?;
    if semantic_value(&existing_config)? != *source_semantics {
        bail!("system config migration conflict for local/local: source and target differ");
    }
    Ok(())
}

fn verify_inserted_payload(
    inserted_payload: &str,
    source_semantics: &Value,
    source_path: &Path,
    config_dir: &Path,
    services: &MigrationServices,
) -> Result<()> {
    let inserted_config = decode_and_validate(inserted_payload, source_path, config_dir, services)
        .context("verify migrated local system config")?;
    if semantic_value(&inserted_config)? != *source_semantics {
        bail!("migrated local system config failed semantic verification");
    }
    Ok(())
}

fn decode_and_validate(
    contents: &str,
    source_path: &Path,
    config_dir: &Path,
    services: &MigrationServices,
) -> Result<Config> {
    let mut config = (services.parse_config_contents)(contents)?;
    config.config_path = source_path.to_path_buf();
    config.workspace_dir = config_dir.join("workspace");
    (services.decrypt_config_secrets)(&mut config, config_dir)?;
    (services.validate_config)(&config)?;
    Ok(config)
}

fn semantic_value(config: &Config) -> Result<Value> {
    serde_json::to_value(config).context("serialize system config for semantic verification")
}

fn encrypted_payload(config: &Config, services: &MigrationServices) -> Result<String> {
    let mut encrypted = config.clone();
    (services.encrypt_config_secrets)(&mut encrypted)?;
    serde_json::to_string(&encrypted).context("serialize encrypted system config")
}

fn move_source_to_backup(source_path: &Path, config_dir: &Path) -> Result<()> {
    let backup_dir = config_dir.join(BACKUP_DIRECTORY);
    let backup_path = backup_dir.join(SOURCE_FILE);
    if backup_path.exists() {
        bail!("system config migration backup conflict: {} already exists", backup_path.display());
    }
    fs::create_dir_all(&backup_dir).with_context(|| {
        format!("create system config migration backup: {}", backup_dir.display())
    })?;
    fs::rename(source_path, &backup_path).with_context(|| {
        format!(
            "move migrated system config into backup: {} -> {}",
            source_path.display(),
            backup_path.display()
        )
    })
}

fn current_timestamp_ms() -> Result<i64> {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before Unix epoch")?;
    i64::try_from(elapsed.as_millis()).context("system timestamp exceeds database integer range")
}
