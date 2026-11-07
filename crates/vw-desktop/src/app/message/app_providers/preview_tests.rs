use crate::app::state::app_providers::preview::{Confirmation, Conflict, SaveOutcome};
use vw_gateway_client::vw_api_types::app_providers::editor_merge::{
    EditorChoice, EditorEdit, EditorResult, EditorValue, EditorView,
};

fn draft() -> Vec<EditorEdit> {
    vec![EditorEdit { path: vec!["显示选项".into()], value: EditorValue::Remove }]
}

fn editor(context: &str) -> EditorView {
    EditorView { context: context.into(), settings: serde_json::json!({}), current: true }
}

#[test]
fn cc04_ui_preview_stale_confirmation_reloads() {
    let mut state = Confirmation {
        editor: Some(editor("old-context")),
        conflict: Some(Conflict {
            paths: vec![vec!["显示选项".into()]], resolution: "old".into()
        }),
    };
    assert_eq!(state.finish(EditorChoice::KeepMine, Err(409)), SaveOutcome::Reload);
    assert!(state.request("host".into(), draft(), EditorChoice::KeepMine).is_none());
    state.editor = Some(editor("fresh-context"));
    let fresh = state.request("host".into(), draft(), EditorChoice::Refuse).unwrap();
    assert_eq!(fresh.context, "fresh-context");
    assert!(fresh.resolution.is_none());
    assert!(state.request("host".into(), draft(), EditorChoice::KeepMine).is_none());
}

#[test]
fn cc04_ui_preview_conflict_choice_is_explicit() {
    let mut state = Confirmation { editor: Some(editor("context")), conflict: None };
    assert!(state.request("host".into(), draft(), EditorChoice::KeepTheirs).is_none());
    let initial = state.request("host".into(), draft(), EditorChoice::Refuse).unwrap();
    assert_eq!(initial.choice, EditorChoice::Refuse);
    assert!(initial.resolution.is_none());
    let paths = vec![vec!["显示选项".to_owned()]];
    assert_eq!(
        state.finish(
            EditorChoice::Refuse,
            Ok(EditorResult {
                saved: false,
                conflicts: paths.clone(),
                resolution: Some("approve".into())
            }),
        ),
        SaveOutcome::Conflict,
    );
    assert_eq!(state.conflict.as_ref().unwrap().paths, paths);
    assert!(state.request("host".into(), draft(), EditorChoice::Refuse).is_none());
    let chosen = state.request("host".into(), draft(), EditorChoice::KeepTheirs).unwrap();
    assert_eq!(chosen.choice, EditorChoice::KeepTheirs);
    assert_eq!(chosen.resolution.as_deref(), Some("approve"));
}
