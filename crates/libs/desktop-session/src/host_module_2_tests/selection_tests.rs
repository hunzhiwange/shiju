use super::{ChatSendBehavior, QueueAction, dispatch, queue_item, update_queue};

#[test]
fn session_agent_selection_normalizes_value_and_resets_tool_context() {
    let mut runtime = crate::session::agent::SessionRuntimeState::<()>::default();
    {
        let selector = &mut runtime.tool_selector;
        selector.toggle_manual_tool("read");
        selector.toggle_manual_skill("review");
        selector.select_tab(desktop_chat_tools::state::SessionToolSelectorTab::Skills);
    }

    crate::message::chat::preferences::select_agent(
        &mut runtime,
        Some(" worker ".into()),
        Some("worker".into()),
        None,
        false,
    );
    assert_eq!(runtime.agent.as_deref(), Some("worker"));
    assert_eq!(
        runtime.tool_selector.active_tab(),
        desktop_chat_tools::state::SessionToolSelectorTab::Skills
    );
    assert!(runtime.tool_selector.selected_manual_tools().is_empty());
    assert!(runtime.tool_selector.selected_manual_skills().is_empty());

    crate::message::chat::preferences::select_agent(
        &mut runtime,
        Some("main".into()),
        Some("main".into()),
        None,
        false,
    );
    assert_eq!(runtime.agent, None);
}

#[test]
fn queue_guide_removes_item_and_dispatches_with_stop_and_send() {
    let mut runtime = crate::session::agent::SessionRuntimeState::<()>::default();
    crate::session::agent::clear_pending_guide_handoff(42);
    {
        runtime.request_lifecycle.is_requesting = true;
        runtime.request_lifecycle.active_agent_request =
            Some(std::sync::Arc::new(queue_item("", ChatSendBehavior::Queue).into_request(
                42,
                "s1".into(),
                String::new(),
                Vec::new(),
                None,
            )));
        runtime.request_lifecycle.queue = vec![
            queue_item("waiting", ChatSendBehavior::Queue),
            queue_item("guide now", ChatSendBehavior::Queue),
        ];
    }

    let crate::message::chat::request::QueueEffect::Send(item) =
        update_queue(&mut runtime.request_lifecycle.queue, QueueAction::Guide(1))
    else {
        panic!("expected send");
    };
    let crate::message::chat::lifecycle::DispatchEffect::Start { item, .. } =
        dispatch(&mut runtime, item, true)
    else {
        panic!("expected replacement");
    };
    let prepared = crate::message::chat::lifecycle::prepare_request(
        &mut runtime,
        item,
        &mut Vec::new(),
        &mut Vec::new(),
        100,
    );
    crate::message::chat::lifecycle::begin_request(
        &mut runtime,
        prepared.item,
        43,
        "s1".into(),
        prepared.request_query,
        prepared.request_history,
        prepared.is_draft,
        100,
        false,
    );

    assert_eq!(runtime.request_lifecycle.queue.len(), 1);
    assert_eq!(runtime.request_lifecycle.queue[0].query, "waiting");
    assert!(!crate::session::agent::take_pending_guide_handoff(42));

    update_queue(&mut runtime.request_lifecycle.queue, QueueAction::Guide(9));
    assert_eq!(runtime.request_lifecycle.queue.len(), 1);
    assert!(!crate::session::agent::take_pending_guide_handoff(42));
}
