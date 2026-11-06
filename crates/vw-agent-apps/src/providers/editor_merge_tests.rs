use super::editor_merge::*;
use serde_json::json;
use vw_api_types::app_providers::editor_merge::*;
fn edit() -> EditorEdit {
    EditorEdit { path: vec!["x".into()], value: EditorValue::Set { value: json!(2) } }
}
#[test]
fn cc04_editor_merge_keeps_external_unedited_fields() {
    let bytes = merge(
        &json!({"x":1,"z":false}),
        br#"{"x":1,"z":true,"secret":"fixture"}"#,
        &[edit()],
        EditorChoice::Refuse,
        true,
    )
    .unwrap();
    let result: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(result, json!({"x":2,"z":true,"secret":"fixture"}));
}
#[test]
fn cc04_editor_merge_conflict_requires_explicit_choice() {
    let base = json!({"x":1});
    let current = br#"{"x":3,"z":true}"#;
    assert_eq!(
        merge(&base, current, &[edit()], EditorChoice::Refuse, true),
        Err(MergeError::Conflict(vec![vec!["x".into()]]))
    );
    for (choice, x) in [(EditorChoice::KeepMine, 2), (EditorChoice::KeepTheirs, 3)] {
        let value: serde_json::Value =
            serde_json::from_slice(&merge(&base, current, &[edit()], choice, true).unwrap())
                .unwrap();
        assert_eq!(value, json!({"x":x,"z":true}));
    }
}
#[test]
fn cc04_editor_merge_distinguishes_null_removal_and_unchanged_values() {
    let base = json!({"x":1,"null":null,"nested":{"edited":1,"other":false}});
    let edits = vec![
        edit(),
        EditorEdit { path: vec!["null".into()], value: EditorValue::Remove },
        EditorEdit {
            path: vec!["nested".into(), "edited".into()],
            value: EditorValue::Set { value: json!(2) },
        },
    ];
    let bytes = merge(
        &base,
        br#"{"x":2,"null":null,"nested":{"edited":1,"other":true}}"#,
        &edits,
        EditorChoice::Refuse,
        true,
    )
    .unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&bytes).unwrap(),
        json!({"x":2,"nested":{"edited":2,"other":true}})
    );
    let same = EditorEdit { path: vec!["x".into()], value: EditorValue::Set { value: json!(1) } };
    assert_eq!(
        merge(&base, br#"{"x":9}"#, &[same], EditorChoice::Refuse, true).unwrap(),
        br#"{"x":9}"#
    );
}
#[test]
fn cc04_editor_merge_rejects_owned_overlapping_and_concealed_edits() {
    for path in [
        vec!["env"],
        vec!["env", "ANTHROPIC_API_KEY"],
        vec!["env", "ENABLE_TOOL_SEARCH"],
        vec!["model"],
        vec!["apiFormat"],
    ] {
        let e = EditorEdit {
            path: path.into_iter().map(str::to_owned).collect(),
            value: EditorValue::Set { value: json!("new") },
        };
        assert_eq!(
            merge(&json!({}), b"{}", &[e], EditorChoice::Refuse, true),
            Err(MergeError::Invalid)
        );
    }
    for value in [json!("[redacted]"), json!({"nested":"<redacted>"})] {
        let e = EditorEdit { path: vec!["x".into()], value: EditorValue::Set { value } };
        assert_eq!(
            merge(&json!({}), b"{}", &[e], EditorChoice::Refuse, false),
            Err(MergeError::Invalid)
        );
    }
    assert_eq!(
        merge(&json!({}), b"{}", &[edit(), edit()], EditorChoice::Refuse, true),
        Err(MergeError::Invalid)
    );
    assert_eq!(
        merge(&json!({}), b"{broken", &[], EditorChoice::Refuse, true),
        Err(MergeError::Invalid)
    );
}
