//! Domain contracts moved from the executable tool host.
pub(crate) mod host;
mod catalog;
mod context;
mod external_directory;
use crate::{Tool, ToolResult, ToolSpec};
use tool_context::context::WorkflowState;
use tool_context::read_state::{FileReadStateCache, FileSnapshot};
use std::{path::{Path, PathBuf}, sync::{Arc, Mutex}};
type ToolUseContext = tool_context::context::ToolUseContext<host::Host>;
use tool_context::context::{scope_tool_use_context, current_tool_use_context, normalize_path_for_context};
fn current_read_state_for_path(path: impl AsRef<Path>) -> Option<tool_context::read_state::FileReadStateEntry> { tool_context::context::current_read_state_for_path::<host::Host>(path) }
fn note_current_read_state(path: impl AsRef<Path>, bytes: usize, partial: bool, offset: Option<usize>, limit: Option<usize>, snapshot: Option<FileSnapshot>) -> Option<PathBuf> { tool_context::context::note_current_read_state::<host::Host>(path, bytes, partial, offset, limit, snapshot) }
fn invalidate_current_read_state(path: impl AsRef<Path>) -> Option<tool_context::read_state::FileReadStateEntry> { tool_context::context::invalidate_current_read_state::<host::Host>(path) }
mod decision;
