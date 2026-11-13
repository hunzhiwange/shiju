use crate::app::state::CRON_TEMPLATES;

#[test]
fn templates_prefill_agent_content_and_valid_schedules_without_granting_access() {
    assert_eq!(CRON_TEMPLATES.len(), 12);
    for template in CRON_TEMPLATES {
        let draft = template.draft(chrono::Local::now()).unwrap();
        assert_eq!(draft.name, template.title);
        assert_eq!(draft.prompt_editor.text(), draft.prompt);
        assert!(!draft.full_access);
        assert!(!draft.delivery_enabled);
    }
}
