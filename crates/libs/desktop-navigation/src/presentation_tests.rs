use super::*;
#[test]
fn work_workspace_resource_mode_defaults_to_overview() {
    assert_eq!(WorkResourceMode::default(), WorkResourceMode::Overview);
}
