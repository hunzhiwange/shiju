//! 配置迁移组合入口；业务实现由 agent-config 持有。

use std::path::Path;

use anyhow::Result;
use crate::migration::MigrationServices;

pub use crate::migration::MigrationOutcome;

fn services() -> MigrationServices {
    MigrationServices {
        parse_config_contents: crate::runtime_schema::parse_config_contents,
        decrypt_config_secrets: crate::runtime_schema::decrypt_config_secrets,
        encrypt_config_secrets: crate::runtime_schema::encrypt_config_secrets,
        validate_config: crate::runtime_schema::validate_config,
        initialize_store: |database| crate::store::SystemConfigStore::new(database).map(|_| ()),
    }
}

pub fn migrate_user_config(home: &Path) -> Result<MigrationOutcome> {
    crate::migration::migrate_user_config(home, &services())
}

#[cfg(test)]
pub(crate) fn migrate_user_config_with_database(
    home: &Path,
    database: agent_database::GlobalDatabase,
) -> Result<MigrationOutcome> {
    crate::migration::migrate_user_config_with_database(home, database, &services())
}
