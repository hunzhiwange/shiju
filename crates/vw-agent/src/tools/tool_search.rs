//! Client-side discovery. Only runtime-owned state can expose deferred tools.

use super::context::current_tool_use_context;
use super::discovery::{MAX_SEARCH_LIMIT, ToolDiscoveryCatalog, ToolExposureState};
use super::{Tool, ToolResult, ToolSpec};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Mutex;

pub(crate) const TOOL_SEARCH_ID: &str = "tool_search";

/// One authorized user turn. Sharing this reference never grants extra tools.
/// Callers capture exposed_specs before executing a batch, then refresh next round.
pub(crate) struct ToolDiscoverySession {
    catalog: ToolDiscoveryCatalog,
    exposure: Mutex<ToolExposureState>,
    search_enabled: bool,
}

impl ToolDiscoverySession {
    pub(crate) fn new(authorized_specs: Vec<ToolSpec>) -> Self {
        let search_allowed = authorized_specs.iter().any(|spec| spec.id == TOOL_SEARCH_ID);
        let catalog = ToolDiscoveryCatalog::new(authorized_specs);
        let search_enabled = search_allowed && catalog.has_deferred_tools();
        Self { catalog, exposure: Mutex::new(ToolExposureState::default()), search_enabled }
    }

    pub(crate) fn authorizes(&self, id: &str) -> bool {
        self.catalog.contains(id) && (id != TOOL_SEARCH_ID || self.search_enabled)
    }

    pub(crate) fn search_tool(&self) -> ToolSearchTool {
        ToolSearchTool::new(&self.catalog)
    }

    pub(crate) fn exposed_specs(&self) -> Vec<ToolSpec> {
        let exposure = self.exposure.lock().unwrap_or_else(|error| error.into_inner());
        self.catalog
            .exposed_specs(&exposure, self.search_enabled)
            .into_iter()
            .filter(|spec| spec.id != TOOL_SEARCH_ID || self.search_enabled)
            .map(|spec| {
                if spec.id == TOOL_SEARCH_ID {
                    ToolSearchTool::new(&self.catalog).spec()
                } else {
                    spec
                }
            })
            .collect()
    }

    fn search(&self, args: Args) -> anyhow::Result<String> {
        if !self.search_enabled {
            anyhow::bail!("tool_search is not authorized for this request");
        }
        let tools = self.catalog.search(&args.query, args.limit)?;
        // Prepare the model result before changing visibility. No JSON from a
        // tool result, hook, history or context_updates is accepted as state.
        let output = serde_json::to_string(&json!({"tools": tools}))?;
        self.exposure
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .record_loaded(&self.catalog, tools.iter().map(|spec| spec.id.as_str()));
        Ok(output)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Args {
    query: String,
    #[serde(default)]
    limit: Option<usize>,
}

fn parse_args(input: Value) -> anyhow::Result<Args> {
    let args: Args = serde_json::from_value(input)?;
    if args.query.trim().is_empty() {
        anyhow::bail!("query must contain a searchable word or character");
    }
    if args.limit.is_some_and(|limit| !(1..=MAX_SEARCH_LIMIT).contains(&limit)) {
        anyhow::bail!("limit must be between 1 and {MAX_SEARCH_LIMIT}");
    }
    Ok(args)
}

pub(crate) struct ToolSearchTool {
    description: String,
}

impl ToolSearchTool {
    pub(crate) fn new(catalog: &ToolDiscoveryCatalog) -> Self {
        Self {
            description: format!(
                "Search deferred MCP tool metadata with BM25. First search for the required tool, \
                 then invoke its returned stable ID in the next model call. Search only reads \
                 the authorized local catalog and never executes remote actions. \
                 Available MCP servers: {}.",
                catalog.deferred_server_names().join(", ")
            ),
        }
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for ToolSearchTool {
    fn name(&self) -> &str {
        TOOL_SEARCH_ID
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "query": {"type": "string", "description": "Search query for deferred tools."},
                "limit": {"type": "integer", "minimum": 1, "maximum": MAX_SEARCH_LIMIT,
                          "description": "Maximum tools returned; defaults to 8."}
            },
            "required": ["query"]
        })
    }

    fn validate_input(&self, input: Value) -> anyhow::Result<Value> {
        parse_args(input.clone())?;
        Ok(input)
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn execute(&self, input: Value) -> anyhow::Result<ToolResult> {
        let args = parse_args(input)?;
        let context = current_tool_use_context()
            .ok_or_else(|| anyhow::anyhow!("tool_search requires an authorized runtime catalog"))?;
        let discovery = context
            .tool_discovery()
            .ok_or_else(|| anyhow::anyhow!("tool_search requires an authorized runtime catalog"))?;
        Ok(ToolResult { success: true, output: discovery.search(args)?, error: None })
    }
}

#[cfg(test)]
#[path = "tool_search_tests.rs"]
mod tests;
