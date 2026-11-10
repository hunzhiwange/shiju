//! Request-boundary publication of complete MCP specifications and executors.
use super::{
    mcp_connections::{McpSessionConnections, configuration_key},
    mcp_notifications::ToolCatalogBinding,
};
use crate::tools::{Tool, catalog::ToolCatalog, mcp_common::server_name};
use agent_client_protocol::McpServer;
use anyhow::{Result, bail};
use std::sync::Arc;

#[derive(Clone)]
pub(super) struct Source {
    server: McpServer,
    management: vw_acp::McpServerManagement,
    binding: Option<ToolCatalogBinding>,
}
impl McpSessionConnections {
    pub(crate) fn register_tool_source(
        &self,
        server: McpServer,
        management: vw_acp::McpServerManagement,
    ) {
        let binding = self.tool_binding(&server);
        if binding.is_none() {
            return;
        }
        let mut state = self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        match state.tool_sources.entry(server_name(&server).to_owned()) {
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                entry.get_mut().server = server;
                entry.get_mut().management = management;
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(Source { server, management, binding });
            }
        }
    }

    pub(crate) async fn discover_catalog_tools(
        self: &Arc<Self>,
        server: &McpServer,
        management: vw_acp::McpServerManagement,
        existing_ids: &[&str],
    ) -> Result<Vec<Arc<dyn Tool>>> {
        let discovered = crate::tools::discover_dynamic_mcp_tools(
            server.clone(),
            management.clone(),
            existing_ids,
            self.clone(),
        )
        .await?;
        self.register_tool_source(server.clone(), management);
        Ok(discovered.tools.into_iter().map(|tool| Arc::new(tool) as Arc<dyn Tool>).collect())
    }

    /// Preserve existing ID grants; new IDs require an explicit configured tool allowlist.
    /// Callers retain the previous snapshot through every call in the model response.
    pub(crate) async fn refresh_tool_catalog(
        self: &Arc<Self>,
        catalog: &ToolCatalog,
    ) -> Result<()> {
        let _refresh = self.tool_refresh.lock().await;
        self.ensure_ready()?;
        let sources = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .tool_sources
            .clone();
        if sources.is_empty() {
            return Ok(());
        }
        let previous = catalog.snapshot();
        let mut tools: Vec<_> = previous
            .tools()
            .iter()
            .filter(|tool| {
                let spec = tool.spec();
                spec.source != vw_api_types::tools::ToolSource::Mcp
                    || !spec
                        .identity
                        .namespace
                        .as_ref()
                        .is_some_and(|name| sources.contains_key(name))
            })
            .cloned()
            .collect();
        let mut bindings = Vec::new();
        #[cfg(not(target_arch = "wasm32"))]
        let mut revoked = Vec::new();
        let mut changed = false;
        for (name, source) in &sources {
            let enabled = {
                let state = self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                state.allowed.as_ref().is_none_or(|allowed| {
                    allowed.get(name) == configuration_key(&source.server).ok().as_ref()
                })
            };
            if !enabled {
                changed = true;
                continue;
            }
            #[cfg(not(target_arch = "wasm32"))]
            if !self.managed_selection_is_current(&source.server)? {
                revoked.push(&source.server);
                changed = true;
                continue;
            }
            let ids: Vec<_> = tools.iter().map(|tool| tool.spec().id).collect();
            let refs: Vec<_> = ids.iter().map(String::as_str).collect();
            let discovered = crate::tools::discover_dynamic_mcp_tools(
                source.server.clone(),
                source.management.clone(),
                &refs,
                self.clone(),
            )
            .await?;
            let binding = self.tool_binding(&source.server);
            changed |= !source
                .binding
                .as_ref()
                .zip(binding.as_ref())
                .is_some_and(|(before, after)| before.same_catalog(after));
            tools.extend(
                discovered
                    .tools
                    .into_iter()
                    .filter(|tool| {
                        let spec = tool.spec();
                        previous.authorizes(&spec.id)
                            || (!source.management.allowed_tools.is_empty()
                                && crate::tools::mcp_common::mcp_tool_allowed(
                                    &source.management,
                                    &spec.identity.name,
                                    &spec.id,
                                ))
                    })
                    .map(|tool| Arc::new(tool) as Arc<dyn Tool>),
            );
            bindings.push((name.clone(), binding));
        }
        if !changed {
            let before: std::collections::BTreeMap<_, _> =
                previous.specs().into_iter().map(|spec| (spec.id.clone(), spec.to_dto())).collect();
            let after: std::collections::BTreeMap<_, _> = tools
                .iter()
                .map(|tool| tool.spec())
                .map(|spec| (spec.id.clone(), spec.to_dto()))
                .collect();
            if before == after {
                return Ok(());
            }
        }
        let ids = tools.iter().map(|tool| tool.spec().id).collect::<Vec<_>>();
        // Hold session state through publication so shutdown/removal cannot race it.
        let mut state = self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if state.closed {
            bail!("MCP session is closed");
        }
        for (name, binding) in &bindings {
            let source = sources.get(name).expect("registered MCP source");
            if state.allowed.as_ref().is_some_and(|allowed| {
                allowed.get(name) != configuration_key(&source.server).ok().as_ref()
            }) || !binding.as_ref().is_some_and(|binding| binding.same_catalog(binding))
            {
                bail!("MCP connection changed during refresh; retry at the next request");
            }
        }
        catalog.replace_if_current(&previous, tools, ids)?;
        #[cfg(not(target_arch = "wasm32"))]
        for server in revoked {
            state.retire_managed_server(server);
        }
        state.tool_sources.retain(|name, _| bindings.iter().any(|(enabled, _)| enabled == name));
        for (name, binding) in bindings {
            if let Some(source) = state.tool_sources.get_mut(&name) {
                source.binding = binding;
            }
        }
        Ok(())
    }
}
