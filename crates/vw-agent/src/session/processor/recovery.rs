//! Recovery implementation belongs to the session engine.
pub(crate) use session_engine::recovery::RetryBudget;
#[cfg(test)]
pub(crate) use session_engine::recovery::retry_allowed;

// Pure recovery tests follow the implementation in session-engine.
