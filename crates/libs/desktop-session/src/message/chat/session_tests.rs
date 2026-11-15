#[test]
fn compaction_uses_latest_user_boundary_independent_of_gateway_order() {
    let newest_first = vec![persisted_user_message("user-3"), persisted_user_message("user-1")];
    let oldest_first = vec![persisted_user_message("user-1"), persisted_user_message("user-3")];

    assert_eq!(super::latest_persisted_user_message_id(&newest_first).as_deref(), Some("user-3"));
    assert_eq!(super::latest_persisted_user_message_id(&oldest_first).as_deref(), Some("user-3"));
}

#[test]
fn message_id_for_index_uses_previous_known_id_when_current_is_missing() {
    let ids = vec![Some("msg-1".to_string()), None];

    assert_eq!(super::message_id_for_index_in_ids(&ids, 1), Some("msg-1".to_string()));
}

#[test]
fn message_id_for_index_handles_gateway_rows_shorter_than_local_chat() {
    let ids = vec![Some("msg-1".to_string())];

    assert_eq!(super::message_id_for_index_in_ids(&ids, 3), Some("msg-1".to_string()));
}

#[test]
fn message_id_for_index_returns_current_id_before_previous_fallback() {
    let ids = vec![Some("msg-1".to_string()), Some("msg-2".to_string())];

    assert_eq!(super::message_id_for_index_in_ids(&ids, 1), Some("msg-2".to_string()));
    assert_eq!(super::message_id_for_index_in_ids(&ids, 0), Some("msg-1".to_string()));
    assert_eq!(super::message_id_for_index_in_ids(&[], 0), None);
}

#[test]
fn message_id_for_index_returns_none_when_current_and_previous_ids_are_missing() {
    let ids = vec![None, None, Some("future".to_string())];

    assert_eq!(super::message_id_for_index_in_ids(&ids, 1), None);
}

#[test]
fn parse_inline_mode_command_accepts_known_commands_case_insensitively() {
    assert_eq!(
        super::parse_inline_mode_command("/task build"),
        Some(super::InlineModeCommand::TaskMode)
    );
    assert_eq!(
        super::parse_inline_mode_command(" /NEW topic"),
        Some(super::InlineModeCommand::ChatMode)
    );
    assert_eq!(
        super::parse_inline_mode_command("/clear"),
        Some(super::InlineModeCommand::ChatMode)
    );
    assert_eq!(
        super::parse_inline_mode_command("/session x"),
        Some(super::InlineModeCommand::ChatMode)
    );
    assert_eq!(super::parse_inline_mode_command("/unknown"), None);
    assert_eq!(super::parse_inline_mode_command(""), None);
}

#[test]
fn normalize_delegate_agent_trims_and_drops_main_agent() {
    assert_eq!(
        super::normalize_delegate_agent(Some(" worker ".to_string())),
        Some("worker".to_string())
    );
    assert_eq!(
        super::normalize_delegate_agent(Some(desktop_settings_agents::state::MAIN_AGENT_KEY.to_string())),
        None
    );
    assert_eq!(super::normalize_delegate_agent(Some("   ".to_string())), None);
    assert_eq!(super::normalize_delegate_agent(None), None);
    assert_eq!(
        super::normalize_delegate_agent(Some(format!("  {}  ", desktop_settings_agents::state::MAIN_AGENT_KEY))),
        None
    );
}

#[test]
fn request_delegate_agent_defaults_to_main_agent() {
    assert_eq!(
        super::request_delegate_agent(Some("worker".to_string())),
        Some("worker".to_string())
    );
    assert_eq!(
        super::request_delegate_agent(None),
        Some(desktop_settings_agents::state::MAIN_AGENT_KEY.to_string())
    );
}

#[test]
fn compose_query_with_session_context_appends_selected_tools_and_skills() {
    assert_eq!(super::compose_query_with_session_context("  hi  ", &[], &[], &[]), "hi");

    let composed = super::compose_query_with_session_context(
        "run this",
        &["shell".to_string(), "git".to_string()],
        &["rust".to_string()],
        &[],
    );

    assert!(composed.starts_with("run this\n\n<session_control_selection>"));
    assert!(composed.contains("工具：shell, git"));
    assert!(composed.contains("技能：rust"));
    assert!(composed.ends_with("</session_control_selection>"));
}

#[test]
fn compose_query_with_session_context_can_stand_alone_without_query() {
    let composed =
        super::compose_query_with_session_context("   ", &["shell".to_string()], &[], &[]);

    assert!(composed.starts_with("<session_control_selection>"));
    assert!(composed.contains("工具：shell"));
}

#[test]
fn compose_query_with_session_context_supports_skills_without_tools() {
    let composed =
        super::compose_query_with_session_context(" inspect ", &[], &["rust".to_string()], &[]);

    assert!(composed.starts_with("inspect\n\n<session_control_selection>"));
    assert!(!composed.contains("工具："));
    assert!(composed.contains("技能：rust"));
}

#[test]
fn compose_query_maps_multiple_shiju_functions_to_skill_context() {
    let selections = [
        desktop_chat_tools::state::ShijuSkillSelection::new(
            42,
            "document-review".to_string(),
            "文档审查".to_string(),
        ),
        desktop_chat_tools::state::ShijuSkillSelection::new(
            43,
            "meeting-notes".to_string(),
            "会议纪要".to_string(),
        ),
    ];
    let composed = super::compose_query_with_session_context(
        "   ",
        &[],
        &["shiju".to_string(), "rust".to_string()],
        &selections,
    );

    assert!(composed.starts_with("<session_control_selection>"));
    assert_eq!(composed.matches("技能：shiju, rust").count(), 1);
    assert!(composed.contains("识矩功能：document-review, meeting-notes"));
    assert!(composed.contains(
        "默认将用户的问题理解为针对上述已选识矩功能；不要回答成对识矩本身的介绍或使用说明"
    ));
    assert!(composed.ends_with("</session_control_selection>"));
}

#[test]
fn compose_query_omits_shiju_default_subject_hint_without_shiju_functions() {
    let composed =
        super::compose_query_with_session_context("inspect", &[], &["rust".to_string()], &[]);

    assert!(!composed.contains("默认将用户的问题理解为针对上述已选识矩功能"));
}

#[test]
fn is_supported_image_attachment_matches_known_extensions_case_insensitively() {
    assert!(super::is_supported_image_attachment("/tmp/a.PNG"));
    assert!(super::is_supported_image_attachment("/tmp/a.jpeg"));
    assert!(super::is_supported_image_attachment("/tmp/a.webp"));
    assert!(!super::is_supported_image_attachment("/tmp/a.pdf"));
    assert!(!super::is_supported_image_attachment("/tmp/no-extension"));
}

#[test]
fn compose_query_with_attachments_adds_document_and_image_markers() {
    assert_eq!(super::compose_query_with_attachments("  hello  ", &[]), "hello");
    assert_eq!(
        super::compose_query_with_attachments(
            "",
            &["/tmp/a.png".to_string(), "/tmp/b.pdf".to_string()]
        ),
        "[IMAGE:/tmp/a.png]\n[DOCUMENT:/tmp/b.pdf]"
    );
    assert_eq!(
        super::compose_query_with_attachments("look", &["/tmp/a.gif".to_string()]),
        "look\n\n[IMAGE:/tmp/a.gif]"
    );
}

#[test]
fn compose_query_with_attachments_returns_empty_for_blank_query_without_attachments() {
    assert_eq!(super::compose_query_with_attachments("   ", &[]), "");
}

#[test]
fn push_recovered_message_id_only_emits_pending_content() {
    let mut ids = Vec::new();
    let message_id = Some("msg-1".to_string());
    let mut has_content = false;

    super::push_recovered_message_id(&mut ids, &mut has_content, &message_id);
    assert!(ids.is_empty());

    has_content = true;
    super::push_recovered_message_id(&mut ids, &mut has_content, &message_id);
    assert_eq!(ids, vec![message_id]);
    assert!(!has_content);
}

fn persisted_user_message(id: &str) -> message_types::WithParts {
    use message_types as message;

    message::WithParts {
        info: message::Info::User(Box::new(message::UserInfo {
            id: id.to_string(),
            session_id: "session-1".to_string(),
            time: message::UserTime { created: 1 },
            summary: None,
            agent: desktop_settings_agents::state::MAIN_AGENT_KEY.to_string(),
            model: message::ModelRef {
                provider_id: "provider".to_string(),
                model_id: "model".to_string(),
            },
            system: None,
            tools: None,
            variant: None,
        })),
        parts: Vec::new(),
    }
}
