//! Agent Loop Core Constants
//!
//! This module defines the core constants used throughout the agent execution loop.
//! These constants control various aspects of agent behavior including:
//!
//! - **Stream processing**: Minimum chunk sizes for LLM text streaming
//! - **Loop limits**: Maximum iterations to prevent runaway execution
//! - **Message handling**: History limits and timeout configurations
//! - **Memory management**: Auto-save thresholds
//! - **Error recovery**: Retry prompts for malformed tool calls
//!
//! # Design Principles
//!
//! All constants in this module are `pub(crate)` to ensure they are only accessible
//! within the agent crate. This encapsulation allows for:
//!
//! - Centralized tuning of agent behavior
//! - Easy testing and configuration validation
//! - Consistent values across all agent components
//!
//! # Usage
//!
//! ```ignore
//! use crate::app::agent::agent::loop_::core::constants::*;
//!
//! if iterations >= DEFAULT_MAX_TOOL_ITERATIONS {
//!     warn!("Maximum tool iterations reached");
//! }
//! ```

/// Minimum number of characters per chunk when relaying LLM text to a streaming draft.
///
/// This constant controls the granularity of text streaming from the LLM to the user
/// interface. Smaller values provide more responsive streaming but may increase
/// overhead, while larger values reduce overhead but feel less responsive.
///
/// # Value
///
/// Set to `5` characters as a balance between:
/// - Responsiveness: Users see updates quickly
/// - Efficiency: Not too many small chunks to process
///
/// # Example
///
/// ```ignore
/// let chunk = &text[position..position.saturating_add(STREAM_CHUNK_MIN_CHARS)];
/// if chunk.len() >= STREAM_CHUNK_MIN_CHARS {
///     send_chunk(chunk);
/// }
/// ```
pub(crate) const STREAM_CHUNK_MIN_CHARS: usize = 5;

/// Default maximum agentic tool-use iterations per user message.
///
/// This fallback is used when no positive limit is configured.
///
/// # Value
///
/// Set to `50000` iterations.
///
/// # Configuration
///
/// This value is used as a fallback when:
/// - `max_tool_iterations` is not configured in the agent config
/// - `max_tool_iterations` is explicitly set to `0`
///
/// Users can override this in their agent configuration:
///
/// ```toml
/// [agent]
/// max_tool_iterations = 50000
/// ```
///
/// # See Also
///
/// - [`crate::app::agent::config::schema::AgentConfig::max_tool_iterations`]
pub(crate) const DEFAULT_MAX_TOOL_ITERATIONS: usize = 50_000;

/// Default maximum number of messages to retain in conversation history.
///
/// This limit prevents unbounded memory growth during long conversations.
/// Older messages beyond this limit are pruned to maintain context within
/// reasonable bounds while preserving recent conversation context.
///
/// # Value
///
/// Set to `100` messages, providing:
/// - Sufficient context for multi-turn conversations
/// - Protection against memory bloat
/// - Reasonable context window for most LLM providers
///
/// # Trade-offs
///
/// - **Lower values**: Less memory usage, but may lose important context
/// - **Higher values**: More context preserved, but increased memory and token usage
///
/// # Example
///
/// ```ignore
/// if history.len() > DEFAULT_MAX_HISTORY_MESSAGES {
///     history.drain(..history.len() - DEFAULT_MAX_HISTORY_MESSAGES);
/// }
/// ```
pub(crate) const DEFAULT_MAX_HISTORY_MESSAGES: usize = 100;

/// Minimum timeout duration (in seconds) for channel message operations.
///
/// This serves as a floor value for message timeouts, ensuring that even
/// fast operations have sufficient time to complete, particularly useful
/// for:
/// - Network latency variations
/// - Temporary service unavailability
/// - Rate limiting scenarios
///
/// # Value
///
/// Set to `30` seconds, which accommodates:
/// - Most network operations
/// - Initial connection establishment
/// - Message queuing and delivery
///
/// # Note
///
/// This is a minimum value; actual timeouts may be dynamically scaled
/// based on [`CHANNEL_MESSAGE_TIMEOUT_SCALE_CAP`].
pub(crate) const MIN_CHANNEL_MESSAGE_TIMEOUT_SECS: u64 = 30;

/// Maximum scaling factor for channel message timeouts.
///
/// When dynamic timeout scaling is enabled, the actual timeout is calculated
/// as: `base_timeout * scale_factor`, where `scale_factor` is capped at this value.
///
/// This prevents timeouts from growing indefinitely during repeated retries
/// or degraded network conditions.
///
/// # Value
///
/// Set to `4`, meaning:
/// - Maximum timeout = `MIN_CHANNEL_MESSAGE_TIMEOUT_SECS * 4` = 120 seconds
/// - Allows for exponential backoff with a hard limit
///
/// # Example
///
/// ```ignore
/// let effective_timeout = base_timeout
///     .saturating_mul(scale_factor.min(CHANNEL_MESSAGE_TIMEOUT_SCALE_CAP));
/// ```
pub(crate) const CHANNEL_MESSAGE_TIMEOUT_SCALE_CAP: u64 = 4;

/// Minimum message length (in characters) required to trigger auto-save to memory.
///
/// Messages shorter than this threshold are considered too brief to be
/// meaningfully stored in long-term memory. This prevents:
/// - Noise from very short exchanges (e.g., "ok", "thanks")
/// - Excessive memory storage overhead
/// - Lower quality training data for memory retrieval
///
/// # Value
///
/// Set to `20` characters, matching the equivalent constant in `channels/mod.rs`
/// to ensure consistent behavior across the agent system.
///
/// # Consistency Note
///
/// This value **must** remain synchronized with `channels/mod.rs` to ensure
/// channel-side and agent-side auto-save logic behave identically.
///
/// # Example
///
/// ```ignore
/// if message.len() >= AUTOSAVE_MIN_MESSAGE_CHARS {
///     memory_store.save(message).await?;
/// }
/// ```
///
/// # See Also
///
/// - `channels/mod.rs` for the channel-side equivalent constant
pub const AUTOSAVE_MIN_MESSAGE_CHARS: usize = 20;

/// Prompt template for recovering from missing or malformed tool calls.
///
/// When an agent's response implies it should use a tool or claims action
/// completion but fails to emit a valid tool call, this prompt is injected
/// to guide the agent back to correct behavior.
///
/// # Purpose
///
/// This handles edge cases where:
/// - The LLM "forgets" to emit a tool call despite indicating intent
/// - The tool call format is malformed and rejected
/// - The agent defers action without proper tool invocation
///
/// # Content
///
/// The prompt instructs the agent to either:
/// 1. Emit a proper tool call using the required format, or
/// 2. Provide a complete final answer without deferring action
///
/// # Example
///
/// ```ignore
/// if tool_call.is_none() && implies_tool_use(&response) {
///     messages.push(Message::system(MISSING_TOOL_CALL_RETRY_PROMPT));
/// }
/// ```
pub(crate) const MISSING_TOOL_CALL_RETRY_PROMPT: &str = "Internal correction: your last reply implied a follow-up action or claimed action completion, but no valid tool call was emitted. If a tool is needed, emit it now using the required format. If no tool is needed, provide the complete final answer now and do not defer action.";

/// Prompt appended when a model returns neither tool calls nor final text.
pub(crate) const EMPTY_FINAL_RESPONSE_RETRY_PROMPT: &str = "Internal correction: your last reply returned no final result. Continue working from the existing context. Use tools if more work is required; otherwise provide a non-empty final answer with the result now.";

#[cfg(test)]
#[path = "constants_tests.rs"]
mod constants_tests;
