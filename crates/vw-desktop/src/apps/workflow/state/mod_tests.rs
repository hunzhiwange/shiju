use super::*;

#[test]
fn validation_reports_whether_any_field_errors_exist() {
    let empty_validation = WorkflowNodeEditorValidation::default();
    let validation = WorkflowNodeEditorValidation {
        field_errors: vec![WorkflowNodeValidationError {
            path: "basic.title".to_string(),
            message: "请输入标题".to_string(),
        }],
    };

    assert!(!empty_validation.has_errors());
    assert!(validation.has_errors());
}

#[test]
fn validation_returns_first_error_message_for_matching_path() {
    let validation = WorkflowNodeEditorValidation {
        field_errors: vec![
            WorkflowNodeValidationError {
                path: "basic.title".to_string(),
                message: "请输入标题".to_string(),
            },
            WorkflowNodeValidationError {
                path: "advanced_yaml.raw_data".to_string(),
                message: "YAML 格式无效".to_string(),
            },
            WorkflowNodeValidationError {
                path: "basic.title".to_string(),
                message: "标题过长".to_string(),
            },
        ],
    };

    assert_eq!(validation.first_error_for("basic.title"), Some("请输入标题"));
    assert_eq!(validation.first_error_for("advanced_yaml.raw_data"), Some("YAML 格式无效"));
    assert_eq!(validation.first_error_for("visual.unknown"), None);
}
