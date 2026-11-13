use super::templates::TEMPLATES;
use crate::app::state::CronJobDraft;

#[test]
fn templates_prefill_agent_content_and_valid_schedules_without_granting_access() {
    assert_eq!(TEMPLATES.len(), 12);
    for template in TEMPLATES {
        let mut draft = CronJobDraft::default();
        template.apply(&mut draft);
        assert_eq!(draft.name, template.title);
        assert_eq!(draft.prompt_editor.text(), draft.prompt);
        assert!(!draft.prompt.is_empty());
        assert!(!draft.full_access);
        assert!(!draft.delivery_enabled);
        assert!(draft.preview.is_none());
        assert!(draft.schedule_draft.as_ref().unwrap().request().is_ok());
    }
}
