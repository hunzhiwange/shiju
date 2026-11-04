//! 系统模板使用原子插入与条件更新，禁止初始化覆盖租户编辑。
use super::*;

pub(in crate::workflow) async fn write_template_record(
    database: GlobalDatabase,
    tenant: TenantScope,
    body: WorkflowRecordUpsertBody,
    expected_updated_at_ms: Option<u64>,
) -> Result<WorkflowRecord, WorkflowStoreError> {
    validate_body(&body)?;
    super::super::templates::validate_template_yaml(&body.workflow_yaml)?;
    if expected_updated_at_ms.is_some_and(|version| version >= i64::MAX as u64) {
        return Err(WorkflowStoreError::bad_request("invalid template version"));
    }
    spawn_store_task(database, move |database| {
        let uuid = super::super::templates::VIDEO_EDIT_TEMPLATE_UUID;
        let now = now_ms();
        match database.provider() {
            DatabaseProvider::Sqlite => {
                let mut connection = open_db(&database)?;
                let transaction = connection
                    .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
                    .map_err(sql_error)?;
                let changed = if let Some(version) = expected_updated_at_ms {
                    transaction.execute(
                        "UPDATE workflow_workflows SET name = ?4, description = ?5,
                         workflow_yaml = ?6, updated_at_ms = MAX(?7, updated_at_ms + 1)
                         WHERE platform_id = ?1 AND company_id = ?2 AND uuid = ?3
                           AND updated_at_ms = ?8",
                        params![
                            tenant.platform_id().as_ref(),
                            tenant.company_id().as_ref(),
                            uuid,
                            body.name.trim(),
                            body.description.trim(),
                            body.workflow_yaml,
                            now,
                            version
                        ],
                    )
                } else {
                    transaction.execute(
                        "INSERT INTO workflow_workflows
                         (platform_id, company_id, uuid, name, description, workflow_yaml,
                          created_at_ms, updated_at_ms) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)
                         ON CONFLICT(platform_id, company_id, uuid) DO NOTHING",
                        params![
                            tenant.platform_id().as_ref(),
                            tenant.company_id().as_ref(),
                            uuid,
                            body.name.trim(),
                            body.description.trim(),
                            body.workflow_yaml,
                            now
                        ],
                    )
                }
                .map_err(sql_error)?;
                if expected_updated_at_ms.is_some() && changed != 1 {
                    return Err(version_conflict());
                }
                let record = query_record(&transaction, &tenant, uuid)?
                    .ok_or_else(|| WorkflowStoreError::internal("template record was not saved"))?;
                transaction.commit().map_err(sql_error)?;
                Ok(record)
            }
            DatabaseProvider::Postgres => {
                let store = PostgresWorkflowStore::new(database).map_err(database_error)?;
                store
                    .write_template_record(&tenant, uuid, &body, now, expected_updated_at_ms)
                    .map_err(database_error)?
                    .ok_or_else(version_conflict)
            }
        }
    })
    .await
}

fn version_conflict() -> WorkflowStoreError {
    WorkflowStoreError::Conflict("模板已被修改，请重新读取后保存".into())
}
