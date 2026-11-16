// Tests migrated from desktop host state; production types retain their original owners.
mod behavior_0 {
use crate::state::*;
#[test]
fn mcp_new_server_default_is_enabled_stdio_with_empty_inputs() {
    let server = McpNewServerState::default();

    assert!(server.name_input.is_empty());
    assert_eq!(server.transport, McpNewServerTransport::Stdio);
    assert!(server.command_input.is_empty());
    assert!(server.args_input.is_empty());
    assert!(server.url_input.is_empty());
    assert_eq!(server.timeout_input, "30");
    assert!(server.enabled);
    assert!(!server.required);
}
}

mod behavior_1 {
use crate::state::*;
#[test]
fn mcp_transports_expose_stable_labels_and_display_values() {
    let expected = ["stdio", "http", "sse"];

    for (transport, label) in McpNewServerTransport::ALL.into_iter().zip(expected) {
        assert_eq!(transport.label(), label);
        assert_eq!(transport.to_string(), label);
    }
}
}
