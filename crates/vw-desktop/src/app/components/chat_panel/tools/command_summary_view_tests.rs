use super::command_summary_view::{
    COMMAND_LIST_TOP_SPACING, is_command_tool, tool_command_summary_view,
};
use super::types::CommandItem;
use crate::app::{App, Message};
use iced::Element;
use iced::advanced::widget::Tree;

fn command(tool_idx: usize, name: &str, input: &str) -> CommandItem {
    CommandItem {
        tool_idx,
        raw: format!("tool {name}\n{{\"input\":\"{input}\",\"status\":\"completed\"}}"),
    }
}

#[test]
fn command_tool_recognizes_canonical_command_aliases() {
    for name in ["bash", "shell", "execute", "terminal"] {
        assert!(is_command_tool(&format!("tool {name}\n{{}}")), "expected {name} to be a command");
    }

    assert!(!is_command_tool("tool read\n{}"));
    assert!(!is_command_tool("not a tool"));
}

#[test]
fn command_summary_returns_none_for_empty_group() {
    let app = App::new().0;

    assert!(tool_command_summary_view(&app, 0, &[]).is_none());
}

#[test]
fn command_summary_builds_collapsed_and_expanded_rows() {
    assert_eq!(COMMAND_LIST_TOP_SPACING, 10.0);

    let mut app = App::new().0;
    let items = vec![
        command(3, "bash", "printf 中文"),
        command(4, "shell", "cargo clippy --all-targets --all-features"),
    ];

    {
        let collapsed =
            tool_command_summary_view(&app, 2, &items).expect("collapsed command summary");
        let collapsed_tree = Tree::new(collapsed.as_widget());
        assert_eq!(collapsed_tree.children[0].children.len(), 1);
    }

    app.chat_tool_expanded.insert((2_u64 << 32) | 3);
    let expanded = tool_command_summary_view(&app, 2, &items).expect("expanded command summary");
    let expanded_tree = Tree::new(expanded.as_widget());
    assert_eq!(expanded_tree.children[0].children.len(), 2);

    let element: Element<'_, Message> = expanded;
    std::hint::black_box(element);
}
