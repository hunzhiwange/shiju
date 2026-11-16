//! 验证代理工具权限、分组展示与选择消息的边界。
//! 未知工具、继承权限和显式覆盖用于约束权限展示的优先级。

use iced::advanced::widget::Tree;

#[test]
fn tools_view_distinguishes_empty_and_all_grouped_tool_lists() {
    let mut app = crate::agents_state::AgentsSettingsState::default();
    let mut entry = app.entries[0].clone();
    app.available_tools.clear();

    let empty_tree = {
        let empty = super::tools_view(&app, &entry);
        Tree::new(empty.as_widget())
    };
    assert_eq!(empty_tree.children.len(), 2);
    assert!(empty_tree.children[1].children.is_empty());

    app.available_tools =
        ["read", "grep", "bash", "web_search", "AgentTool", "memory_store", "custom_tool"]
            .map(str::to_string)
            .to_vec();
    entry.allowed_tools = vec!["read".to_string(), "custom_tool".to_string()];

    let grouped = super::tools_view(&app, &entry);
    let grouped_tree = Tree::new(grouped.as_widget());
    let tools_list = &grouped_tree.children[1];

    // tabs + select-all + preset section + presets + optional groups
    assert_eq!(tools_list.children.len(), 5);
    assert_eq!(tools_list.children[0].children.len(), 2);
    assert_eq!(tools_list.children[1].children.len(), 3);
    assert_eq!(tools_list.children[3].children.len(), 5);
    // 可选项分别归入联网、协作、记忆和其他分组。
    assert_eq!(tools_list.children[4].children.len(), 4);
}

#[test]
fn tools_view_keeps_zero_match_presets_visible_without_grouping_them() {
    let mut app = crate::agents_state::AgentsSettingsState::default();
    let entry = app.entries[0].clone();
    app.available_tools = vec!["custom_tool".to_string()];

    let view = super::tools_view(&app, &entry);
    let tree = Tree::new(view.as_widget());
    let tools_list = &tree.children[1];

    assert_eq!(tools_list.children.len(), 5);
    assert_eq!(tools_list.children[3].children.len(), 5);
    assert_eq!(tools_list.children[4].children.len(), 1);
    assert_eq!(tools_list.children[4].children[0].children[2].children.len(), 1);
}

#[test]
fn tools_view_preserves_duplicate_available_tools_and_selection_state() {
    let mut app = crate::agents_state::AgentsSettingsState::default();
    let mut entry = app.entries[0].clone();
    app.available_tools =
        vec!["pdf_read".to_string(), "pdf_read".to_string(), "notebook_edit".to_string()];
    entry.allowed_tools = vec!["pdf_read".to_string(), "ghost_tool".to_string()];

    let view = super::tools_view(&app, &entry);
    let tree = Tree::new(view.as_widget());
    let tools_list = &tree.children[1];
    let file_cards = &tools_list.children[4].children[0].children[2];

    assert_eq!(tools_list.children[1].children.len(), 3);
    assert_eq!(file_cards.children.len(), 3);
}

#[test]
fn tools_view_splits_optional_and_builtin_tabs() {
    use crate::state::{AGENT_TOOLS_BUILTIN_TAB, AGENT_TOOLS_OPTIONAL_TAB};
    use config_tools::is_builtin_required_tool_id;

    let mut app = crate::agents_state::AgentsSettingsState::default();
    let entry = app.entries[0].clone();
    app.available_tools = vec![
        "bash".to_string(),
        "custom_optional".to_string(),
        "AgentTool".to_string(),
        "create_thread".to_string(),
    ];

    assert_eq!(app.active_tools_list_tab, AGENT_TOOLS_OPTIONAL_TAB);

    let optional_ids =
        crate::tools::tools_for_list_tab(&app.available_tools, &app.active_tools_list_tab);
    assert_eq!(optional_ids, vec!["custom_optional", "AgentTool", "create_thread"]);
    assert!(optional_ids.iter().all(|id| !is_builtin_required_tool_id(id)));

    {
        let optional_view = super::tools_view(&app, &entry);
        let optional_tree = Tree::new(optional_view.as_widget());
        let optional_list = &optional_tree.children[1];
        // tabs + select-all row + preset section + preset buttons + grouped cards
        assert!(optional_list.children.len() >= 4, "optional tab should keep preset controls");
        assert_eq!(optional_list.children[0].children.len(), 2, "optional/builtin tab buttons");
    }

    app.active_tools_list_tab = AGENT_TOOLS_BUILTIN_TAB.to_string();
    let builtin_ids =
        crate::tools::tools_for_list_tab(&app.available_tools, &app.active_tools_list_tab);
    assert_eq!(builtin_ids, vec!["bash"]);
    assert!(builtin_ids.iter().all(|id| is_builtin_required_tool_id(id)));

    let builtin_view = super::tools_view(&app, &entry);
    let builtin_tree = Tree::new(builtin_view.as_widget());
    let builtin_list = &builtin_tree.children[1];
    assert_eq!(
        builtin_list.children.len(),
        2,
        "builtin tab should only keep tabs row and grouped cards (no presets)"
    );
}

#[test]
fn builtin_tool_cards_are_locked_and_badged() {
    use crate::message::AgentsMessage;
    use crate::message::Event as Message;
    use crate::state::AGENT_TOOLS_BUILTIN_TAB;

    let mut app = crate::agents_state::AgentsSettingsState::default();
    let mut entry = app.entries[0].clone();
    app.available_tools = vec!["bash".to_string(), "custom_optional".to_string()];
    app.active_tools_list_tab = AGENT_TOOLS_BUILTIN_TAB.to_string();
    entry.allowed_tools = vec!["bash".to_string()];

    assert!(crate::tools::tool_card_is_locked("bash"));
    assert!(!crate::tools::tool_card_is_locked("custom_optional"));
    assert_eq!(crate::tools::builtin_tool_badge_label(), "内置");

    assert!(
        crate::tools::tool_card_press_message("main".to_string(), "bash".to_string(), true, true)
            .is_none(),
        "locked builtin cards must not emit cancel toggle"
    );
    let optional_press = crate::tools::tool_card_press_message(
        "main".to_string(),
        "custom_optional".to_string(),
        false,
        false,
    );
    assert!(matches!(
        optional_press,
        Some(Message::Agents(
            AgentsMessage::AllowedToolToggled(ref agent, ref tool, true)
        )) if agent == "main" && tool == "custom_optional"
    ));

    let view = super::tools_view(&app, &entry);
    let tree = Tree::new(view.as_widget());
    let tools_list = &tree.children[1];
    // tabs + groups; first group card title row includes badge child
    let groups = &tools_list.children[1];
    let first_group_cards = &groups.children[0].children[2];
    assert_eq!(first_group_cards.children.len(), 1);
    let card_content = &first_group_cards.children[0].children[0];
    let title_row = &card_content.children[0];
    assert_eq!(title_row.children.len(), 2, "builtin card title should include name + 内置 badge");
}

#[test]
fn shell_companions_are_builtin_and_optional_capabilities_can_be_cancelled() {
    for id in ["bash", "bash_wait", "bash_cancel", "file_edit", "edit"] {
        assert!(crate::tools::tool_card_is_locked(id));
        assert!(matches!(
            crate::tools::tool_meta(id).group,
            crate::tools::ToolGroup::Execute | crate::tools::ToolGroup::Files
        ));
    }
    for id in ["pdf_read", "AgentTool", "create_thread", "memory_store", "browser"] {
        assert!(!crate::tools::tool_card_is_locked(id));
        assert!(
            crate::tools::tool_card_press_message("main".into(), id.into(), true, false).is_some()
        );
    }
}
