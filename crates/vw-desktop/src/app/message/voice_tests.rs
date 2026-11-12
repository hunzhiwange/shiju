use super::voice_draft::{Composer, Draft, Phase, Scope};
fn scope(session: &str) -> Scope {
    Scope {
        endpoint: "http://127.0.0.1:1".into(),
        tenant: "tenant".into(),
        project: Some("project".into()),
        composer: Composer::Session(session.into()),
    }
}
#[test]
fn voice_task_07_dictation_fills_original_draft_once() {
    let mut draft = Draft::default();
    let original = scope("original");
    let generation = draft.start(original.clone(), "前后".into()).unwrap();
    draft.phase = Phase::Transcribing;
    assert_eq!(draft.complete(generation, &original, "前后", "听写".into()), Some("听写".into()));
    assert_eq!(draft.complete(generation, &original, "前后", "重复".into()), None);
}
#[test]
fn voice_task_07_dictation_cancel_ignores_late_result() {
    let mut draft = Draft::default();
    let original = scope("original");
    let generation = draft.start(original.clone(), "草稿".into()).unwrap();
    draft.cancel();
    assert_eq!(draft.complete(generation, &original, "草稿", "迟到".into()), None);
    let next = draft.start(scope("other"), "别的草稿".into()).unwrap();
    assert!(next > generation);
    assert_eq!(draft.complete(generation, &original, "别的草稿", "迟到".into()), None);
    assert_eq!(draft.phase, Phase::Recording);
}
#[test]
fn voice_task_07_edited_draft_retains_result_even_after_undo() {
    let mut draft = Draft::default();
    let original = scope("original");
    let generation = draft.start(original.clone(), "草稿".into()).unwrap();
    draft.observe("用户编辑");
    draft.phase = Phase::Transcribing;
    assert_eq!(draft.complete(generation, &original, "草稿", "结果".into()), None);
    assert_eq!(draft.pending.as_deref(), Some("结果"));
    assert!(draft.start(original, "草稿".into()).is_none());
    draft.cancel();
    assert!(draft.pending.is_none());
}
#[test]
fn voice_task_07_scope_and_phase_reject_unrelated_completion() {
    for changed in 0..4 {
        let original = scope("original");
        let mut other = original.clone();
        match changed {
            0 => other.endpoint.push('2'),
            1 => other.tenant.push('2'),
            2 => other.project = None,
            _ => other.composer = Composer::Session("other".into()),
        }
        let mut draft = Draft::default();
        let generation = draft.start(original.clone(), "草稿".into()).unwrap();
        assert!(draft.start(original.clone(), "草稿".into()).is_none());
        assert_eq!(draft.complete(generation, &original, "草稿", "未完成录音".into()), None);
        draft.phase = Phase::Transcribing;
        assert_eq!(draft.complete(generation, &other, "草稿", "别的上下文".into()), None);
        assert_eq!(draft.complete(generation + 1, &original, "草稿", "旧请求".into()), None);
        assert!(draft.pending.is_none());
    }
}
