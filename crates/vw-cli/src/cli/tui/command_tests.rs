use vw_tui::V3Action;

use super::bridge::GatewayV3Bridge;
use super::commands::{CommandRoute, command_palette_items};
use super::controller::{TuiController, TuiControllerCommand};
use super::state::TuiState;

#[test]
fn selected_command_is_never_silently_ignored() {
    let mut bridge = GatewayV3Bridge::default();
    let mut state = TuiState::default();
    let mut controller = TuiController;

    let command = bridge
        .controller_command(
            &mut state,
            &mut controller,
            V3Action::Select {
                surface: "command_palette".to_string(),
                item_id: "help".to_string(),
            },
        )
        .expect("a known stable command id must be routed");
    assert_eq!(command, TuiControllerCommand::Command(CommandRoute::Help));

    let unknown = bridge
        .controller_command(
            &mut state,
            &mut controller,
            V3Action::Select {
                surface: "command_palette".to_string(),
                item_id: "not-registered".to_string(),
            },
        )
        .expect_err("an unknown command id must be visible to the user");
    assert!(unknown.contains("not-registered"));
}

#[test]
fn command_catalog_exposes_stable_ids_and_disabled_reasons() {
    let items = command_palette_items();
    let ids = items.iter().map(|item| item.id.as_str()).collect::<Vec<_>>();
    assert_eq!(
        &ids[..3],
        &["help", "status", "exit"],
        "the locally implemented command ids remain stable"
    );
    assert!(items.iter().any(|item| item.id == "resume" && !item.disabled));
    assert!(items.iter().any(|item| item.id == "continue" && !item.disabled));
    assert!(items.iter().any(|item| item.id == "new" && !item.disabled));
}

#[test]
fn unknown_slash_command_is_rejected_without_a_gateway_request() {
    let mut bridge = GatewayV3Bridge::default();
    let mut state = TuiState::default();
    let mut controller = TuiController;

    let error = bridge
        .controller_command(
            &mut state,
            &mut controller,
            V3Action::Submit { text: "/does-not-exist".to_string(), attachments: Vec::new() },
        )
        .expect_err("unknown slash input must not be submitted as chat");
    assert!(error.contains("does-not-exist"));

    let local = TuiControllerCommand::Command(CommandRoute::Status);
    assert!(GatewayV3Bridge::request_for_command(&state, &local).is_none());
}
