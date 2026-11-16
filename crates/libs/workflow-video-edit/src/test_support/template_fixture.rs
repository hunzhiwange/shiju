use agent_database::GlobalDatabase;
use vw_api_transport::TenantScope;
use workflow_core::templates::*;
use serde_json::{Value, json};
pub struct TemplateEdit {
    pub digest: String,
    original_yaml: String,
    updated_at_ms: u64,
}
impl TemplateEdit {
    pub async fn restore(self, database: GlobalDatabase, tenant: TenantScope) {
        save_template(database, tenant, vw_api_workflow::WorkflowRecordUpsertBody {
            uuid: Some(VIDEO_EDIT_TEMPLATE_UUID.into()), name: VIDEO_EDIT_TEMPLATE_NAME.into(),
            description: String::new(), workflow_yaml: self.original_yaml,
        }, self.updated_at_ms).await.unwrap();
    }
}
pub async fn edit(database: &GlobalDatabase, tenant: &TenantScope) -> TemplateEdit {
    let seed = VideoEditTemplateSeed::production().unwrap();
    let original = seed.load((*database).clone(), (*tenant).clone()).await.unwrap();
    let mut yaml: Value = serde_yaml::from_str(&original.record.workflow_yaml).unwrap();
    let brief = yaml["workflow"]["graph"]["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|n| n["id"] == "brief")
        .unwrap();
    let text = brief["data"]["prompt_template"][0]["text"].as_str().unwrap();
    brief["data"]["prompt_template"][0]["text"] = json!(format!("{text} E2E_TEMPLATE_EDIT"));
    let body = vw_api_workflow::WorkflowRecordUpsertBody {
        uuid: Some(VIDEO_EDIT_TEMPLATE_UUID.into()),
        name: VIDEO_EDIT_TEMPLATE_NAME.into(),
        description: String::new(),
        workflow_yaml: serde_json::to_string(&yaml).unwrap(),
    };
    let edited =
        save_template((*database).clone(), (*tenant).clone(), body, original.record.updated_at_ms)
            .await
            .unwrap();
    let digest = seed.load((*database).clone(), (*tenant).clone()).await.unwrap().digest;
    assert_ne!(digest, original.digest);
    let other = TenantScope::new(format!("e2e-{}", uuid::Uuid::new_v4()), "other-company").unwrap();
    assert!(
        !seed
            .load((*database).clone(), other)
            .await
            .unwrap()
            .record
            .workflow_yaml
            .contains("E2E_TEMPLATE_EDIT")
    );
    TemplateEdit { digest, original_yaml: original.record.workflow_yaml, updated_at_ms: edited.updated_at_ms }
}
