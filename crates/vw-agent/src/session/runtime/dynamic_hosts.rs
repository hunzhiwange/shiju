//! Authenticated session host bindings. Secrets and live callbacks are never persisted.

use super::{
    SessionScope,
    dynamic_calls::{ClientToolHost, ClientToolRequests, DynamicCallOwner},
};
use crate::tools::catalog::{ToolCatalog, ToolCatalogSnapshot};
use crate::tools::client_tool::ClientTool;
use crate::tools::{Tool, ToolSpec, ToolUseContext};
use anyhow::{Result, bail};
use std::sync::{Arc, LazyLock, Mutex};
use vw_api_types::tools::{
    DynamicToolHostBinding, DynamicToolRegistration, ReplaceDynamicToolsRequest,
};

static HOSTS: LazyLock<Mutex<Vec<Arc<SessionHost>>>> = LazyLock::new(|| Mutex::new(Vec::new()));
const MAX_SESSION_HOSTS: usize = 256;

pub(crate) struct SessionHost {
    pub host: Arc<ClientToolHost>,
    secret: String,
    requests: Mutex<Option<ClientToolRequests>>,
    pub catalog: ToolCatalog,
}

impl SessionHost {
    pub fn register(scope: SessionScope, session: &str) -> Result<DynamicToolHostBinding> {
        let mut hosts = HOSTS.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if hosts
            .iter()
            .any(|entry| entry.host.owner().scope == scope && entry.host.owner().session == session)
        {
            bail!("session already has a dynamic tool host");
        }
        if hosts.len() >= MAX_SESSION_HOSTS {
            bail!("dynamic tool host limit reached");
        }
        let host_id = uuid::Uuid::new_v4().to_string();
        let secret = format!("{}{}", uuid::Uuid::new_v4().simple(), uuid::Uuid::new_v4().simple());
        let (host, requests) =
            ClientToolHost::register(DynamicCallOwner::new(scope, session, &host_id)?);
        hosts.push(Arc::new(Self {
            host,
            secret: secret.clone(),
            requests: Mutex::new(Some(requests)),
            catalog: ToolCatalog::default(),
        }));
        Ok(DynamicToolHostBinding { host_id, binding_token: secret })
    }

    pub fn authenticate(scope: &SessionScope, session: &str, secret: &str) -> Result<Arc<Self>> {
        let hosts = HOSTS.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        hosts
            .iter()
            .find(|entry| {
                entry.host.owner().scope == *scope
                    && entry.host.owner().session == session
                    && entry.secret == secret
            })
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("invalid or expired dynamic tool host binding"))
    }

    pub fn replace(&self, request: ReplaceDynamicToolsRequest) -> Result<u64> {
        // Persistence is a trusted server policy, never a client-granted capability.
        if request.persist {
            bail!("persistent dynamic tool updates are disabled by server policy");
        }
        let mut tools: Vec<Arc<dyn Tool>> = Vec::new();
        for registration in request.tools {
            registration.validate().map_err(anyhow::Error::msg)?;
            let specs = match registration {
                DynamicToolRegistration::Function(spec) => vec![spec],
                DynamicToolRegistration::Namespace { tools, .. } => tools,
            };
            for dto in specs {
                let mut spec = ToolSpec::new(dto.id.0, dto.description, dto.input_schema);
                spec.identity = dto.identity;
                spec.source = dto.source;
                spec.exposure = dto.exposure;
                spec.display_name = dto.display_name;
                spec.aliases = dto.aliases;
                spec.read_only = dto.read_only;
                spec.destructive = dto.destructive;
                spec.concurrency_safe = dto.concurrency_safe;
                spec.requires_user_interaction = dto.requires_user_interaction;
                spec.strict = dto.strict;
                tools.push(Arc::new(ClientTool::new(spec, self.host.clone())?));
            }
        }
        let ids = tools.iter().map(|tool| tool.spec().id).collect::<Vec<_>>();
        Ok(self.catalog.replace(tools, ids)?.version())
    }

    pub fn calls(self: &Arc<Self>) -> Result<SessionHostCalls> {
        let requests = self
            .requests
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take()
            .ok_or_else(|| anyhow::anyhow!("dynamic tool call stream already attached"))?;
        Ok(SessionHostCalls { entry: self.clone(), requests })
    }
}

pub(crate) struct SessionHostCalls {
    entry: Arc<SessionHost>,
    requests: ClientToolRequests,
}
impl SessionHostCalls {
    pub async fn recv(&mut self) -> Option<vw_api_types::tools::DynamicToolCallRequest> {
        self.requests.recv().await
    }
}
impl Drop for SessionHostCalls {
    fn drop(&mut self) {
        self.entry.host.disconnect();
        HOSTS
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .retain(|host| !Arc::ptr_eq(host, &self.entry));
    }
}

/// Overlay only at model request boundaries. Existing executors and grants remain intact.
/// The shared executor still applies configured tool policy, approvals and hooks.
pub(crate) fn request_catalog(
    context: &ToolUseContext,
    base: Arc<ToolCatalogSnapshot>,
) -> Result<Arc<ToolCatalogSnapshot>> {
    let Some(root) = context.workspace_root() else {
        return Ok(base);
    };
    let hosts = HOSTS.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let Some(entry) = hosts.iter().find(|entry| {
        entry.host.owner().scope.tenant() == &context.tenant_scope()
            && entry.host.owner().session == context.session()
    }) else {
        return Ok(base);
    };
    if SessionScope::new(context.tenant_scope(), root)? != entry.host.owner().scope {
        return Ok(base);
    }
    let dynamic = entry.catalog.snapshot();
    if dynamic.specs().is_empty() {
        return Ok(base);
    }
    let tools = base.tools().iter().chain(dynamic.tools()).cloned().collect::<Vec<_>>();
    let ids =
        base.specs().into_iter().chain(dynamic.specs()).map(|spec| spec.id).collect::<Vec<_>>();
    // Catalog validation rejects collisions atomically; a host cannot shadow a builtin/MCP tool.
    ToolCatalog::default().replace(tools, ids)
}
