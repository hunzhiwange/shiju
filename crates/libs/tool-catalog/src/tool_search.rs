//! Client-side discovery. Only runtime-owned state can expose deferred tools.

use super::discovery::{MAX_SEARCH_LIMIT, ToolDiscoveryCatalog, ToolExposureState};
use super::ToolSpec;
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Mutex;

pub const TOOL_SEARCH_ID: &str = "tool_search";

/// One authorized user turn. Sharing this reference never grants extra tools.
/// Callers capture exposed_specs before executing a batch, then refresh next round.
pub struct ToolDiscoverySession {
    catalog: ToolDiscoveryCatalog,
    exposure: Mutex<ToolExposureState>,
    search_enabled: bool,
}

impl ToolDiscoverySession {
    pub fn new(authorized_specs: Vec<ToolSpec>) -> Self {
        let search_allowed = authorized_specs
            .iter()
            .any(|spec| spec.id == TOOL_SEARCH_ID && spec.exposure.is_direct());
        let catalog = ToolDiscoveryCatalog::new(authorized_specs);
        let search_enabled = search_allowed && catalog.has_deferred_tools();
        Self {
            catalog,
            exposure: Mutex::new(ToolExposureState::default()),
            search_enabled,
        }
    }

    /// Preserve loaded IDs only while the replacement still authorizes them.
    pub fn rebase(&self, authorized_specs: Vec<ToolSpec>) -> Self {
        let next = Self::new(authorized_specs);
        let mut exposure = self.exposure.lock().unwrap_or_else(|error| error.into_inner()).clone();
        exposure.retain_authorized(&next.catalog);
        *next.exposure.lock().unwrap_or_else(|error| error.into_inner()) = exposure;
        next
    }

    pub fn authorizes(&self, id: &str) -> bool {
        self.catalog.contains(id) && (id != TOOL_SEARCH_ID || self.search_enabled)
    }

    pub fn search_tool(&self) -> ToolSearchTool {
        ToolSearchTool::new(&self.catalog)
    }

    pub fn exposed_specs(&self) -> Vec<ToolSpec> {
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

    #[cfg(not(target_arch = "wasm32"))]
    pub fn authorized_specs(&self) -> Vec<ToolSpec> { self.catalog.authorized_specs() }

    pub fn search(&self, input: Value) -> anyhow::Result<String> {
        let args = parse_args(input)?;
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

pub struct ToolSearchTool {
    description: String,
}

impl ToolSearchTool {
    pub fn new(catalog: &ToolDiscoveryCatalog) -> Self {
        Self {
            description: format!(
                "Search deferred tool names, descriptions, parameter schemas and sources with BM25. First search for the required tool, \
                 then invoke its returned stable ID in the next model call. Search only reads \
                 the authorized local catalog and never executes remote actions. \
                 Available tool namespaces: {}.",
                catalog.deferred_namespace_names().join(", ")
            ),
        }
    }
}

impl ToolSearchTool {
    pub fn name(&self) -> &str {
        TOOL_SEARCH_ID
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn parameters_schema(&self) -> Value {
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

    pub fn validate_input(&self, input: Value) -> anyhow::Result<Value> {
        parse_args(input.clone())?;
        Ok(input)
    }

    pub fn is_read_only(&self) -> bool {
        true
    }

    pub fn is_concurrency_safe(&self) -> bool {
        true
    }

    pub fn spec(&self) -> ToolSpec {
        ToolSpec::new(self.name(), self.description(), self.parameters_schema())
            .with_display_name(self.name()).with_aliases(Vec::<String>::new())
            .with_read_only(true).with_destructive(false).with_concurrency_safe(true)
            .with_requires_user_interaction(false).with_strict(true)
    }
}
