use crate::providers::mode_state::StackState;

/// Reserve a routing key in the application state.
pub fn allocate_key(stack: &mut StackState, id: &str, name: &str, icon: Option<&str>) -> String {
    app_aggregation_catalog::member_keys::allocate_key(&mut stack.keys, id, name, icon)
}
