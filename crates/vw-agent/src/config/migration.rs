//! 配置迁移组合入口；业务实现由 agent-config 持有。

use std::path::Path;

use anyhow::Result;
use agent_config::migration::MigrationServices;
#[cfg(test)]
use crate::global_database::GlobalDatabase;

pub use agent_config::migration::MigrationOutcome;

fn services() -> MigrationServices {
    MigrationServices {
        parse_config_contents: super::schema::parse_config_contents,
        decrypt_config_secrets: super::schema::decrypt_config_secrets,
        encrypt_config_secrets: super::schema::encrypt_config_secrets,
        validate_config: super::schema::validate_config,
        initialize_store: |database| super::SystemConfigStore::new(database).map(|_| ()),
    }
}

pub fn migrate_user_config(home: &Path) -> Result<MigrationOutcome> {
    agent_config::migration::migrate_user_config(home, &services())
}

#[cfg(test)]
pub(crate) fn migrate_user_config_with_database(
    home: &Path,
    database: GlobalDatabase,
) -> Result<MigrationOutcome> {
    agent_config::migration::migrate_user_config_with_database(home, database, &services())
}
