use crate::management_import::result_labels;
use gateway_mcp_distribution::*;
use vw_api_mcp_management::*;
use vw_api_app_identity::{AppId, AppInstallationId};
use vw_api_mcp_management::*;
fn target() -> McpAppTarget {
    McpAppTarget {
        app_id: AppId::Codex,
        installation_id: AppInstallationId("trusted-instance".into()),
        target_host_id: "host".into(),
    }
}
fn response() -> McpImportResponse {
    let record = |id: &str| McpRecordRef { id: McpRecordId(id.into()), revision: 2 };
    McpImportResponse {
        sources: vec![McpImportSourceResult {
            target: target(),
            code: None,
            entries: Some(vec![
                McpImportEntryResult {
                    id: Some(McpRecordId("new".into())),
                    outcome: McpImportEntryOutcome::Added { record: record("new") },
                },
                McpImportEntryResult {
                    id: Some(McpRecordId("existing".into())),
                    outcome: McpImportEntryOutcome::Unchanged { record: record("existing") },
                },
                McpImportEntryResult {
                    id: Some(McpRecordId("merged".into())),
                    outcome: McpImportEntryOutcome::Merged { record: record("merged") },
                },
                McpImportEntryResult {
                    id: Some(McpRecordId("conflict".into())),
                    outcome: McpImportEntryOutcome::Failed { code: McpFailureCode::Conflict },
                },
            ]),
        }],
    }
}
#[test]
fn cc_mcp_t47_results_preserve_conflicts_imported_and_unchanged_items() {
    let labels = result_labels(response(), &target()).unwrap().join("\n");
    assert!(labels.contains("new · 已导入"));
    assert!(labels.contains("existing · 已导入，无需变更"));
    assert!(labels.contains("conflict · 冲突，未覆盖"));
    assert!(labels.contains("merged · 已合并"));
    let mut malformed = response();
    malformed.sources[0].target.target_host_id = "other".into();
    assert!(result_labels(malformed, &target()).is_err());
    let mut malformed = response();
    malformed.sources[0].code = Some(McpFailureCode::Conflict);
    assert!(result_labels(malformed, &target()).is_err());
    let mut malformed = response();
    malformed.sources.clear();
    assert!(result_labels(malformed, &target()).is_err());
}
