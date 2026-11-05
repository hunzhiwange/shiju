use super::*;
use serde_json::{Value, json};

fn request(action: &str) -> Value {
    let target = if action == "install" {
        json!({"kind":"environment", "environment":{"kind":"wsl", "distribution":"Ubuntu"}})
    } else {
        json!({"kind":"installation", "installationId":"instance-1"})
    };
    json!({"appId":"kimi", "action":action, "target":target})
}

#[test]
fn cc_apps_action_contract_roundtrip_fixed_actions_and_preview() {
    for action in ["install", "update", "uninstall"] {
        let wire = request(action);
        let dto: AppActionPreviewRequest = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(serde_json::to_value(dto).unwrap(), wire);
        let wire = json!({"previewId":"preview-1", "request":wire, "source":"native",
            "confirmationRequired":true, "summary":"Review installation change",
            "displayCommand":"display only; never submitted"});
        let dto: AppActionPreview = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(serde_json::to_value(dto).unwrap(), wire);
    }
}

#[test]
fn cc_apps_action_contract_roundtrip_rejects_unknown_and_command_inputs() {
    assert!(serde_json::from_value::<AppActionPreviewRequest>(request("execute")).is_err());
    for field in ["command", "argv", "cwd", "env", "path", "displayCommand"] {
        let mut wire = request("update");
        wire[field] = json!("untrusted");
        assert!(serde_json::from_value::<AppActionPreviewRequest>(wire).is_err(), "{field}");
        let mut wire = json!({"previewId":"preview-1", "confirmed":true});
        wire[field] = json!("untrusted");
        assert!(serde_json::from_value::<AppActionSubmitRequest>(wire).is_err(), "{field}");
    }
    for target in [
        json!({"kind":"installation"}),
        json!({"kind":"path","path":"/bin/sh"}),
        json!({"kind":"environment","environment":{"kind":"wsl"}}),
        json!({"kind":"installation","installationId":"i", "command":"sh"}),
    ] {
        let mut wire = request("update");
        wire["target"] = target;
        assert!(serde_json::from_value::<AppActionPreviewRequest>(wire).is_err());
    }
    assert!(serde_json::from_value::<AppActionSubmitRequest>(json!({"previewId":"p"})).is_err());
}

#[test]
fn cc_apps_action_contract_roundtrip_submission_and_existing_task_result() {
    for confirmed in [true, false] {
        let wire = json!({"previewId":"p", "confirmed":confirmed});
        let dto: AppActionSubmitRequest = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(serde_json::to_value(dto).unwrap(), wire);
    }
    for status in ["queued", "running", "completed", "failed", "cancelled"] {
        let wire = json!({"operationId":"task-1", "status":status});
        let dto: AppActionExecutionResult = serde_json::from_value(wire.clone()).unwrap();
        let _: crate::id::TaskId = dto.operation_id.clone();
        let _: crate::task::TaskStatus = dto.status.clone();
        assert_eq!(serde_json::to_value(dto).unwrap(), wire);
    }
}
