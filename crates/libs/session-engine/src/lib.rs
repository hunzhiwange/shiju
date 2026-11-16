//! Session identity snapshots and trusted capability policy resolution.

pub mod agent_binding;
pub mod instruction;
pub mod instruction_discovery;
pub mod runner_admission;
pub mod child_agent_access;
#[cfg(not(target_arch = "wasm32"))]
pub mod acp_defaults;
pub mod capability_policy;
pub mod session_memory;
pub mod context_governance;
pub mod llm_messages;

pub mod child_agent;
pub mod context_injection;
pub mod compaction_budget;
pub mod invoked_skill_context;
pub mod request_overflow;
#[cfg(not(target_arch = "wasm32"))]
pub mod native_history;
pub mod processor_events;
pub mod recovery;
pub mod llm_step;
pub mod compaction_summary;
pub mod compaction;
pub mod compaction_checkpoint;
pub mod skill_checkpoint;
pub mod retry_delay;
pub mod retry;
pub mod prompt;
pub mod system_prompt;
pub mod system;
pub mod summary;
pub mod title;
pub mod task_session;
pub mod session_io;
pub mod session_lifecycle;
pub mod status;
pub mod session_fork;
pub mod todo;
pub mod turn_fork;
pub mod revert;
pub mod runtime;
pub mod replay_safety;
pub mod fork_execution;
pub mod binding_store;
pub mod admission;
pub mod admission_host;

pub mod message;
pub mod llm_stream;
pub mod llm_runner;
pub mod processor_clock;
pub mod processor_docs;
pub mod artifacts;
pub mod helpers;
pub mod planning;
pub mod tool_parsing;
pub mod prefetch;
pub mod prompting;
pub mod runner_protocol;
pub mod runner;
pub mod runner_host;
pub mod runner_types;
pub mod runner_messages;
pub mod runner_ingest;
pub mod runner_todos;
pub mod runner_todo_updates;
pub mod turn_policy;
pub mod tools_exec;
pub mod tool_output;
#[cfg(test)]
mod utils_tests;
pub mod tool_file_links;
#[cfg(not(target_arch = "wasm32"))]
pub mod native_call_collector;

#[cfg(test)]
mod host_split_7_tests;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_split_8_tests;

#[cfg(test)]
mod host_migration_19_tests;

#[cfg(test)]
mod host_migration_22_tests;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_migration_23_tests;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_migration_24_tests;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_migration_25_tests;

pub mod delegated_tools;
#[cfg(test)]
mod host_migration_33_tests;
