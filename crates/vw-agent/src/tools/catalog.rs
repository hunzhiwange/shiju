//! Versioned executable catalogs. Metadata registration never grants authorization.

use super::{
    ExecutedToolCall, Tool, ToolCallError, ToolCallResult, ToolRenderHint, ToolResult, ToolSpec,
    ToolUseContext,
};
use serde_json::Value;
use std::collections::BTreeSet;
use std::sync::{Arc, RwLock};

/// A specification and its executor are retained together for the entire request.
#[derive(Default)]
pub struct ToolCatalogSnapshot {
    version: u64,
    tools: Vec<Arc<dyn Tool>>,
    specs: Vec<ToolSpec>,
    authorized: BTreeSet<String>,
}

impl std::fmt::Debug for ToolCatalogSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolCatalogSnapshot")
            .field("version", &self.version)
            .field("tools", &self.tools.len())
            .field("authorized", &self.authorized.len())
            .finish()
    }
}

impl ToolCatalogSnapshot {
    /// Initial runtime-owned tools retain the existing session authorization boundary.
    pub(crate) fn from_runtime_tools(tools: Vec<Arc<dyn Tool>>) -> Self {
        let specs: Vec<_> = tools.iter().map(|tool| tool.spec()).collect();
        let authorized = specs.iter().map(|spec| spec.id.clone()).collect();
        Self { version: 0, tools, specs, authorized }
    }

    pub fn version(&self) -> u64 {
        self.version
    }

    /// Only specifications explicitly authorized for this version are exposed.
    pub fn specs(&self) -> Vec<ToolSpec> {
        self.specs.iter().filter(|spec| self.authorizes(&spec.id)).cloned().collect()
    }

    pub fn authorizes(&self, id: &str) -> bool {
        self.authorized.contains(id)
    }

    pub(crate) fn tools(&self) -> &[Arc<dyn Tool>] {
        &self.tools
    }

    /// Resolve a stable ID at the request boundary, never at a later catalog version.
    pub fn bind(self: &Arc<Self>, id: &str) -> anyhow::Result<CatalogToolCall> {
        if !self.authorizes(id) {
            anyhow::bail!("tool is not authorized in this catalog version: {id}");
        }
        Ok(CatalogToolCall { snapshot: self.clone(), id: id.into() })
    }
}

/// The live pointer changes only after every registration and grant has been validated.
#[derive(Default)]
pub struct ToolCatalog {
    current: RwLock<Arc<ToolCatalogSnapshot>>,
}

impl ToolCatalog {
    /// Resume the persisted version without restoring any saved authorization or executor.
    pub(crate) fn restored(
        version: u64,
        tools: Vec<Arc<dyn Tool>>,
        authorized: impl IntoIterator<Item = String>,
    ) -> anyhow::Result<Self> {
        let previous = version
            .checked_sub(1)
            .ok_or_else(|| anyhow::anyhow!("invalid restored catalog version"))?;
        let catalog = Self {
            current: RwLock::new(Arc::new(ToolCatalogSnapshot {
                version: previous,
                ..ToolCatalogSnapshot::default()
            })),
        };
        catalog.replace(tools, authorized)?;
        Ok(catalog)
    }

    pub fn snapshot(&self) -> Arc<ToolCatalogSnapshot> {
        self.current.read().unwrap_or_else(std::sync::PoisonError::into_inner).clone()
    }

    pub fn replace(
        &self,
        tools: Vec<Arc<dyn Tool>>,
        authorized: impl IntoIterator<Item = String>,
    ) -> anyhow::Result<Arc<ToolCatalogSnapshot>> {
        self.replace_inner(None, tools, authorized, |_| Ok(()))
    }

    pub(crate) fn replace_if_current(
        &self,
        expected: &Arc<ToolCatalogSnapshot>,
        tools: Vec<Arc<dyn Tool>>,
        authorized: impl IntoIterator<Item = String>,
    ) -> anyhow::Result<Arc<ToolCatalogSnapshot>> {
        self.replace_inner(Some(expected), tools, authorized, |_| Ok(()))
    }

    /// Hold the publication lock across persistence; failure leaves the old snapshot intact.
    pub(crate) fn replace_persisted(
        &self,
        tools: Vec<Arc<dyn Tool>>,
        authorized: impl IntoIterator<Item = String>,
        persist: impl FnOnce(&ToolCatalogSnapshot) -> anyhow::Result<()>,
    ) -> anyhow::Result<Arc<ToolCatalogSnapshot>> {
        self.replace_inner(None, tools, authorized, persist)
    }

    fn replace_inner(
        &self,
        expected: Option<&Arc<ToolCatalogSnapshot>>,
        tools: Vec<Arc<dyn Tool>>,
        authorized: impl IntoIterator<Item = String>,
        persist: impl FnOnce(&ToolCatalogSnapshot) -> anyhow::Result<()>,
    ) -> anyhow::Result<Arc<ToolCatalogSnapshot>> {
        let specs: Vec<_> = tools.iter().map(|tool| tool.spec()).collect();
        let mut ids = BTreeSet::new();
        let mut identities = BTreeSet::new();
        let mut names = BTreeSet::new();
        for spec in &specs {
            spec.identity.validate().map_err(anyhow::Error::msg)?;
            if spec.id.trim().is_empty() || !spec.input_schema.is_object() {
                anyhow::bail!("invalid tool ID or schema");
            }
            if !ids.insert(spec.id.clone())
                || !identities.insert((spec.identity.namespace.clone(), spec.identity.name.clone()))
            {
                anyhow::bail!("duplicate tool ID or identity");
            }
            if spec.source != vw_api_types::tools::ToolSource::Builtin
                && std::iter::once(spec.id.as_str())
                    .chain(std::iter::once(spec.identity.name.as_str()))
                    .chain(spec.identity.namespace.as_deref())
                    .chain(spec.aliases.iter().map(String::as_str))
                    .any(|name| matches!(name, "tool_search" | "exec" | "wait"))
            {
                anyhow::bail!("reserved dynamic tool name");
            }
            if spec.source == vw_api_types::tools::ToolSource::Client {
                vw_api_types::tools::DynamicToolRegistration::Function(spec.to_dto())
                    .validate()
                    .map_err(anyhow::Error::msg)?;
            }
            // Aliases participate in execution lookup and cannot shadow another ID.
            for name in std::iter::once(&spec.id).chain(spec.aliases.iter()) {
                if name.trim().is_empty() || !names.insert(name.clone()) {
                    anyhow::bail!("duplicate or empty tool execution name");
                }
            }
        }
        let authorized: BTreeSet<_> = authorized.into_iter().collect();
        if !authorized.is_subset(&ids) {
            anyhow::bail!("authorization references an unregistered tool");
        }
        let tools = tools
            .into_iter()
            .zip(&specs)
            .filter(|(_, spec)| authorized.contains(&spec.id))
            .map(|(inner, spec)| {
                Arc::new(FrozenTool { inner, spec: spec.clone() }) as Arc<dyn Tool>
            })
            .collect();
        let specs = specs.into_iter().filter(|spec| authorized.contains(&spec.id)).collect();
        let mut current = self.current.write().unwrap_or_else(std::sync::PoisonError::into_inner);
        if expected.is_some_and(|expected| !Arc::ptr_eq(expected, &current)) {
            anyhow::bail!("tool catalog changed during MCP refresh; retry at the next request");
        }
        let version = current
            .version
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("tool catalog version exhausted"))?;
        let snapshot = Arc::new(ToolCatalogSnapshot { version, tools, specs, authorized });
        persist(&snapshot)?;
        *current = snapshot.clone();
        Ok(snapshot)
    }
}

/// A pending call cannot be paired with a different version, even from another session.
pub struct CatalogToolCall {
    snapshot: Arc<ToolCatalogSnapshot>,
    id: String,
}

impl CatalogToolCall {
    pub async fn execute(
        &self,
        snapshot: &Arc<ToolCatalogSnapshot>,
        input: serde_json::Value,
        context: Arc<ToolUseContext>,
    ) -> Result<ExecutedToolCall, ToolCallError> {
        if !Arc::ptr_eq(&self.snapshot, snapshot)
            || context.tool_catalog().is_some_and(|bound| !Arc::ptr_eq(bound, snapshot))
        {
            return Err(ToolCallError::denied("tool catalog version mismatch"));
        }
        let tools = super::toolset::boxed_registry_from_arcs(snapshot.tools.clone());
        let context = Arc::new(context.as_ref().clone().with_tool_catalog(snapshot.clone()));
        super::execute_tool_from_registry(&tools, &self.id, input, context).await
    }
}

struct FrozenTool {
    inner: Arc<dyn Tool>,
    spec: ToolSpec,
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Tool for FrozenTool {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn description(&self) -> &str {
        self.inner.description()
    }

    fn parameters_schema(&self) -> serde_json::Value {
        self.inner.parameters_schema()
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        self.inner.execute(args).await
    }

    fn validate_input(&self, input: Value) -> anyhow::Result<Value> {
        self.inner.validate_input(input)
    }

    async fn check_permissions(&self, input: &Value) -> anyhow::Result<()> {
        self.inner.check_permissions(input).await
    }

    async fn call(&self, input: Value) -> anyhow::Result<ToolCallResult> {
        self.inner.call(input).await
    }

    fn map_result_for_model(&self, result: &ToolCallResult) -> Value {
        self.inner.map_result_for_model(result)
    }

    fn render_hint(&self, result: &ToolCallResult) -> Option<ToolRenderHint> {
        self.inner.render_hint(result)
    }

    fn is_concurrency_safe(&self) -> bool {
        self.inner.is_concurrency_safe()
    }

    fn is_read_only(&self) -> bool {
        self.inner.is_read_only()
    }

    fn to_audit_input(&self, input: &Value) -> Value {
        self.inner.to_audit_input(input)
    }

    fn spec(&self) -> ToolSpec {
        self.spec.clone()
    }
}
