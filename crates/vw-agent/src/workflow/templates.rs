//! 租户可编辑的内置入口目录。正式 YAML 由装配方提供，目录不生成业务流程。
use sha2::{Digest, Sha256};
use vw_api_types::{
    TenantScope,
    workflow::{VideoEditTemplateOptions, WorkflowRecord, WorkflowRecordUpsertBody},
};

pub use super::store::WorkflowStoreError;
use crate::global_database::GlobalDatabase;
pub use vw_api_types::workflow::VIDEO_EDIT_TEMPLATE_UUID;

pub const VIDEO_EDIT_TEMPLATE_NAME: &str = "通用云端视频剪辑";

/// 随程序分发的初始资源；通过 Extension 注入应用路由，也供原生入口使用。
#[derive(Clone, Debug)]
pub struct VideoEditTemplateSeed {
    body: WorkflowRecordUpsertBody,
}

impl VideoEditTemplateSeed {
    pub fn production() -> Result<Self, WorkflowStoreError> {
        Self::new(super::video_edit::DEFAULT_VIDEO_EDIT_YAML.into())
    }

    pub fn new(workflow_yaml: String) -> Result<Self, WorkflowStoreError> {
        validate_template_yaml(&workflow_yaml)?;
        Ok(Self {
            body: WorkflowRecordUpsertBody {
                uuid: Some(VIDEO_EDIT_TEMPLATE_UUID.into()),
                name: VIDEO_EDIT_TEMPLATE_NAME.into(),
                description: String::new(),
                workflow_yaml,
            },
        })
    }

    /// 原子注册后读取当前租户记录；已有编辑永远优先于种子。
    pub async fn load(
        &self,
        database: GlobalDatabase,
        tenant: TenantScope,
    ) -> Result<TemplateSnapshot, WorkflowStoreError> {
        let record =
            super::store::write_template_record(database, tenant, self.body.clone(), None).await?;
        TemplateSnapshot::from_record(record)
    }

    /// 仅显式确认且持有当前版本时恢复默认。
    pub async fn restore(
        &self,
        database: GlobalDatabase,
        tenant: TenantScope,
        expected_updated_at_ms: u64,
        confirmed: bool,
    ) -> Result<WorkflowRecord, WorkflowStoreError> {
        if !confirmed {
            return Err(WorkflowStoreError::bad_request("恢复默认模板需要明确确认"));
        }
        save_template(database, tenant, self.body.clone(), expected_updated_at_ms).await
    }
}

/// 一轮运行持有自己的内容、摘要与参数；后续编辑只影响下一次读取。
#[derive(Debug, Clone)]
pub struct TemplateSnapshot {
    pub record: WorkflowRecord,
    pub digest: String,
    pub options: VideoEditTemplateOptions,
}

impl TemplateSnapshot {
    fn from_record(record: WorkflowRecord) -> Result<Self, WorkflowStoreError> {
        let options = validate_template_yaml(&record.workflow_yaml)?;
        let digest = hex::encode(Sha256::digest(record.workflow_yaml.as_bytes()));
        Ok(Self { record, digest, options })
    }
}

pub async fn save_template(
    database: GlobalDatabase,
    tenant: TenantScope,
    body: WorkflowRecordUpsertBody,
    expected_updated_at_ms: u64,
) -> Result<WorkflowRecord, WorkflowStoreError> {
    if body.uuid.as_deref().is_some_and(|uuid| uuid != VIDEO_EDIT_TEMPLATE_UUID) {
        return Err(WorkflowStoreError::bad_request("template uuid does not match entry"));
    }
    super::store::write_template_record(database, tenant, body, Some(expected_updated_at_ms)).await
}

pub(super) fn validate_template_yaml(
    source: &str,
) -> Result<VideoEditTemplateOptions, WorkflowStoreError> {
    super::model::parse_workflow_yaml(source).map_err(WorkflowStoreError::bad_request)?;
    super::video_edit::validate_entry_contract(source).map_err(WorkflowStoreError::bad_request)?;
    let root: serde_json::Value = serde_yaml::from_str(source)
        .map_err(|error| WorkflowStoreError::bad_request(error.to_string()))?;
    let options = root.get("workflow").and_then(|workflow| workflow.get("video_edit"));
    match options {
        Some(value) => serde_json::from_value(value.clone())
            .map_err(|error| WorkflowStoreError::bad_request(error.to_string())),
        None => Ok(VideoEditTemplateOptions::default()),
    }
}
