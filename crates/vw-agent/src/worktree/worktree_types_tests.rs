use super::{CreateInput, Error, Info, RemoveInput, ResetInput};

#[test]
fn error_display_uses_inner_message_for_named_variants() {
    assert_eq!(Error::NotGit("no git".into()).to_string(), "no git");
    assert_eq!(Error::MissingProject("missing".into()).to_string(), "missing");
    assert_eq!(Error::Invalid("bad".into()).to_string(), "bad");
}

#[test]
fn io_error_conversion_preserves_kind_and_display_message() {
    let source =
        std::io::Error::new(std::io::ErrorKind::PermissionDenied, "worktree access denied");
    let error = Error::from(source);

    assert_eq!(error.to_string(), "worktree access denied");
    let Error::Io(inner) = error else {
        panic!("I/O conversion must produce Error::Io");
    };
    assert_eq!(inner.kind(), std::io::ErrorKind::PermissionDenied);
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn join_error_conversion_preserves_task_failure() {
    let join_error = tokio::spawn(async {
        panic!("expected task panic");
    })
    .await
    .expect_err("spawned task must fail");
    let display = join_error.to_string();

    let error = Error::from(join_error);

    assert_eq!(error.to_string(), display);
    let Error::Join(inner) = error else {
        panic!("join conversion must produce Error::Join");
    };
    assert!(inner.is_panic());
}

#[test]
fn error_trait_exposes_no_nested_source() {
    let error = Error::Invalid("invalid operation".to_string());

    assert!(std::error::Error::source(&error).is_none());
}

#[test]
fn info_roundtrip_preserves_all_metadata() {
    let info = Info {
        name: "brave-eagle".to_string(),
        branch: "vibewindow/brave-eagle".to_string(),
        directory: "/workspace/brave-eagle".to_string(),
    };

    let encoded = serde_json::to_value(&info).expect("serialize info");
    assert_eq!(
        encoded,
        serde_json::json!({
            "name": "brave-eagle",
            "branch": "vibewindow/brave-eagle",
            "directory": "/workspace/brave-eagle"
        })
    );

    let decoded: Info = serde_json::from_value(encoded).expect("deserialize info");
    assert_eq!(decoded.name, info.name);
    assert_eq!(decoded.branch, info.branch);
    assert_eq!(decoded.directory, info.directory);
}

#[test]
fn create_input_defaults_leave_both_options_unset() {
    let defaulted = CreateInput::default();
    let decoded: CreateInput =
        serde_json::from_value(serde_json::json!({})).expect("deserialize empty create input");

    assert!(defaulted.name.is_none());
    assert!(defaulted.start_command.is_none());
    assert!(decoded.name.is_none());
    assert!(decoded.start_command.is_none());
}

#[test]
fn create_input_roundtrip_uses_start_command_wire_name() {
    let input = CreateInput {
        name: Some("demo".to_string()),
        start_command: Some("cargo run".to_string()),
    };

    let encoded = serde_json::to_value(&input).expect("serialize create input");
    assert_eq!(encoded["name"], "demo");
    assert_eq!(encoded["startCommand"], "cargo run");
    assert!(encoded.get("start_command").is_none());

    let decoded: CreateInput = serde_json::from_value(encoded).expect("deserialize create input");
    assert_eq!(decoded.name.as_deref(), Some("demo"));
    assert_eq!(decoded.start_command.as_deref(), Some("cargo run"));
}

#[test]
fn remove_input_defaults_force_and_preserves_explicit_true() {
    let defaulted: RemoveInput =
        serde_json::from_value(serde_json::json!({"directory": "/workspace/demo"}))
            .expect("deserialize remove input without force");
    assert!(!defaulted.force);

    let explicit: RemoveInput = serde_json::from_value(serde_json::json!({
        "directory": "/workspace/demo",
        "force": true
    }))
    .expect("deserialize forced remove input");
    assert!(explicit.force);
    assert_eq!(explicit.directory, "/workspace/demo");

    let encoded = serde_json::to_value(&explicit).expect("serialize forced remove input");
    assert_eq!(encoded["directory"], "/workspace/demo");
    assert_eq!(encoded["force"], true);
}

#[test]
fn reset_input_roundtrip_uses_base_ref_wire_name() {
    let input = ResetInput {
        directory: "/workspace/demo".to_string(),
        base_ref: Some("origin/main".to_string()),
    };

    let encoded = serde_json::to_value(&input).expect("serialize reset input");
    assert_eq!(encoded["baseRef"], "origin/main");
    assert!(encoded.get("base_ref").is_none());

    let decoded: ResetInput = serde_json::from_value(encoded).expect("deserialize reset input");
    assert_eq!(decoded.directory, input.directory);
    assert_eq!(decoded.base_ref, input.base_ref);

    let without_base: ResetInput =
        serde_json::from_value(serde_json::json!({"directory": "/workspace/demo"}))
            .expect("deserialize reset input without base ref");
    assert!(without_base.base_ref.is_none());
}

#[test]
fn required_fields_and_invalid_types_are_rejected() {
    assert!(
        serde_json::from_value::<Info>(serde_json::json!({
            "name": "demo",
            "branch": "vibewindow/demo"
        }))
        .is_err()
    );
    assert!(serde_json::from_value::<RemoveInput>(serde_json::json!({"force": false})).is_err());
    assert!(serde_json::from_value::<ResetInput>(serde_json::json!({"baseRef": "main"})).is_err());
    assert!(
        serde_json::from_value::<RemoveInput>(serde_json::json!({
            "directory": "/workspace/demo",
            "force": "yes"
        }))
        .is_err()
    );
}
