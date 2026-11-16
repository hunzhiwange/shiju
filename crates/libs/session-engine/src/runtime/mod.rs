//! Native session runtime services.
pub use session_port::SessionScope;
pub mod dynamic_calls;
#[cfg(test)]
mod dynamic_calls_tests;

pub mod lifecycle;
pub mod prompt;
#[cfg(not(target_arch = "wasm32"))]
pub mod dynamic_hosts;
pub fn is_successful_finish_reason(reason: Option<&str>) -> bool {
    !matches!(
        reason,
        Some(
            "tool_calls"
                | "tool_use"
                | "content_filter"
                | "error"
                | "cancelled"
                | "waiting_for_input"
                | "requires_action"
                | "refusal"
                | "max_turn_requests"
                | "length"
                | "max_tokens"
        )
    )
}
