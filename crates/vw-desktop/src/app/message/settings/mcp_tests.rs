//! Host adapter contract: local configuration effects become desktop tasks.
use super::*;
#[test]
fn configuration_refresh_maps_to_desktop_tasks() {
    let mut app = App::new().0;
    let task = update(&mut app, SettingsMessage::Mcp(McpMessage::Refresh));
    assert!(app.mcp_settings.loading);
    assert!(app.mcp_settings.inspect_loading);
    assert!(task.units() > 0);
}
