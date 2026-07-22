use vw_api_types::staff::StaffCapabilityKindDto;
use vw_config_types::staff::{
    StaffCapabilityConfig, StaffCapabilityIoSummaryConfig, StaffCapabilityKindConfig, StaffConfig,
};

use super::{builtin_stage_capabilities, capability_by_id, stage_capabilities_for_config};

#[test]
fn capability_catalog_filters_invalid_entries_and_normalizes_custom_builtin() {
    let mut config = StaffConfig::default();
    config.project_state.capabilities = vec![
        StaffCapabilityConfig {
            id: "   ".to_string(),
            label: "Ignored blank id".to_string(),
            capability_type: StaffCapabilityKindConfig::BuiltIn,
            built_in: false,
            input_summary: StaffCapabilityIoSummaryConfig::default(),
            result_summary: StaffCapabilityIoSummaryConfig::default(),
        },
        StaffCapabilityConfig {
            id: "declared_builtin".to_string(),
            label: "Ignored built-in declaration".to_string(),
            capability_type: StaffCapabilityKindConfig::BuiltIn,
            built_in: true,
            input_summary: StaffCapabilityIoSummaryConfig::default(),
            result_summary: StaffCapabilityIoSummaryConfig::default(),
        },
        StaffCapabilityConfig {
            id: "  project_tool  ".to_string(),
            label: "  Project Tool  ".to_string(),
            capability_type: StaffCapabilityKindConfig::BuiltIn,
            built_in: false,
            input_summary: StaffCapabilityIoSummaryConfig {
                description: "   ".to_string(),
                fields: vec!["  task_id  ".to_string()],
            },
            result_summary: StaffCapabilityIoSummaryConfig::default(),
        },
    ];

    let builtin_count = builtin_stage_capabilities().len();
    let catalog = stage_capabilities_for_config(&config);

    assert_eq!(catalog.len(), builtin_count + 1);
    let custom = capability_by_id(&catalog, "  project_tool  ")
        .expect("trimmed custom capability id should resolve");
    assert_eq!(custom.label, "Project Tool");
    assert_eq!(custom.capability_type, StaffCapabilityKindDto::BuiltIn);
    assert_eq!(custom.input_summary.description, "");
    assert_eq!(custom.input_summary.fields, vec!["task_id"]);
    assert_eq!(custom.input_contract, "自定义能力输入由目录配置定义");
    assert_eq!(custom.result_semantics, "自定义能力结果由目录配置定义");
    assert_eq!(custom.permission_hint, "使用自定义内置能力配置声明的权限范围。");
    assert!(!custom.requires_workflow_uuid);
}

#[test]
fn capability_lookup_returns_none_for_unknown_or_blank_id() {
    let catalog = builtin_stage_capabilities();

    assert!(capability_by_id(&catalog, "missing").is_none());
    assert!(capability_by_id(&catalog, "   ").is_none());
}
