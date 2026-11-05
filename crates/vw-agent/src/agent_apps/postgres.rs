//! Tenant icon mutations under a PostgreSQL row lock, including first writes.
use super::store::{ICON_KEY, ICON_NAMESPACE, IconLibraryStore, parse_icons};
use vw_api_types::{
    agent_apps::{AgentAppsErrorCode, CustomIconEntry},
    id::TenantScope,
};

impl IconLibraryStore {
    pub(super) fn mutate_postgres<T>(
        &self,
        tenant: &TenantScope,
        operation: impl FnOnce(
            &[CustomIconEntry],
        ) -> Result<(T, Option<Vec<CustomIconEntry>>), AgentAppsErrorCode>,
    ) -> Result<T, AgentAppsErrorCode> {
        let mut business_error = None;
        let result = (|| -> anyhow::Result<T> {
            let mut connection = self.database.connect_postgres()?;
            let mut transaction = connection.transaction()?;
            let key: &[&(dyn postgres::types::ToSql + Sync)] = &[
                &tenant.platform_id().as_ref(),
                &tenant.company_id().as_ref(),
                &ICON_NAMESPACE,
                &ICON_KEY,
            ];
            // Locking a missing row is insufficient: insert first so contenders
            // wait for the same full tenant key even in an empty library.
            let inserted = transaction.execute(
                "INSERT INTO gateway_settings (platform_id,company_id,namespace,config_key,payload_json,updated_at_ms) VALUES ($1,$2,$3,$4,'[]',0) ON CONFLICT (platform_id,company_id,namespace,config_key) DO NOTHING", key,
            )?;
            let raw: String = transaction.query_one(
                "SELECT payload_json FROM gateway_settings WHERE platform_id=$1 AND company_id=$2 AND namespace=$3 AND config_key=$4 FOR UPDATE", key,
            )?.try_get(0)?;
            let (value, updated) = operation(&parse_icons(&raw)).map_err(|error| {
                business_error = Some(error);
                anyhow::anyhow!("icon mutation rejected")
            })?;
            if let Some(updated) = updated {
                let payload = serde_json::to_string(&updated)?;
                let updated_at = i64::try_from(
                    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_millis(),
                )?;
                transaction.execute(
                    "UPDATE gateway_settings SET payload_json=$5,updated_at_ms=$6 WHERE platform_id=$1 AND company_id=$2 AND namespace=$3 AND config_key=$4",
                    &[key[0],key[1],key[2],key[3],&payload,&updated_at],
                )?;
            } else if inserted != 0 {
                // Preserve SQLite's no-op behavior: do not persist a placeholder.
                transaction.rollback()?;
                return Ok(value);
            }
            transaction.commit()?;
            Ok(value)
        })();
        result.map_err(|_| business_error.unwrap_or(AgentAppsErrorCode::StorageUnavailable))
    }
}
