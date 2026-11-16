//! Concrete tool executor factories for the shared host registry.
use std::sync::Arc;
use super::SessionScope;
use crate::tools::{Tool, ToolSpec};
pub use session_engine::runtime::dynamic_hosts::{HostRegistrationConflict, SessionHostCalls};
pub(crate) struct HostTools;
impl session_engine::runtime::dynamic_hosts::HostTools for HostTools {
    type Context = crate::tools::context::AgentContextHost;
    fn client_tool(spec: ToolSpec, host: Arc<super::dynamic_calls::ClientToolHost>) -> anyhow::Result<Arc<dyn Tool>> { Ok(Arc::new(crate::tools::client_tool::ClientTool::new(spec, host)?)) }
    fn search_tool(catalog: &crate::tools::discovery::ToolDiscoveryCatalog) -> Arc<dyn Tool> { Arc::new(crate::tools::tool_search::ToolSearchTool::new(catalog)) }
}
pub(crate) struct SessionHost(Arc<session_engine::runtime::dynamic_hosts::SessionHost>);
impl std::ops::Deref for SessionHost {
    type Target = session_engine::runtime::dynamic_hosts::SessionHost;
    fn deref(&self) -> &Self::Target { &self.0 }
}
impl SessionHost {
    pub fn register(scope: SessionScope, info: session_info::Info, policy: session_store::dynamic_tool_store::PersistentToolPolicy) -> anyhow::Result<vw_api_tools::DynamicToolHostBinding> { session_engine::runtime::dynamic_hosts::SessionHost::register(scope, info, policy) }
    pub fn authenticate(scope: &SessionScope, session: &str, secret: &str) -> anyhow::Result<Arc<Self>> { session_engine::runtime::dynamic_hosts::SessionHost::authenticate(scope, session, secret).map(|host| Arc::new(Self(host))) }
    pub fn replace(&self, request: vw_api_tools::ReplaceDynamicToolsRequest) -> anyhow::Result<u64> { self.0.replace::<HostTools>(request) }
    pub fn calls(self: &Arc<Self>) -> anyhow::Result<SessionHostCalls> { self.0.calls() }
}
pub(crate) fn request_catalog(context: &crate::tools::ToolUseContext, base: Arc<crate::tools::catalog::ToolCatalogSnapshot>) -> anyhow::Result<Arc<crate::tools::catalog::ToolCatalogSnapshot>> { session_engine::runtime::dynamic_hosts::request_catalog::<HostTools>(context, base) }
