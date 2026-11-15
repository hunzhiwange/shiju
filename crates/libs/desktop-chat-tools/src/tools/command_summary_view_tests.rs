use super::*;
#[test]
fn command_tool_recognizes_canonical_command_aliases() {
    for name in ["bash", "shell", "execute", "terminal"] {
        assert!(is_command_tool(&format!("tool {name}\n{{}}")), "expected {name} to be a command");
    }

    assert!(!is_command_tool("tool read\n{}"));
    assert!(!is_command_tool("not a tool"));
}
