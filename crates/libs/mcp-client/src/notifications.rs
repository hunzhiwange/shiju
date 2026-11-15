//! Connection-local catalog invalidation. Notifications never grant subscriptions or permissions.
use anyhow::{Result, bail};
use rmcp::{Peer, RoleClient, model::Tool};
use std::{
    future::Future,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};
use tokio_util::sync::CancellationToken;

struct Listed<T> {
    revision: u64,
    value: Arc<Vec<T>>,
}
struct Catalog<T> {
    revision: AtomicU64,
    listed: Mutex<Option<Listed<T>>>,
    refresh: tokio::sync::Mutex<()>,
}
impl<T> Default for Catalog<T> {
    fn default() -> Self {
        Self {
            revision: AtomicU64::new(0),
            listed: Mutex::new(None),
            refresh: tokio::sync::Mutex::new(()),
        }
    }
}
impl<T> Catalog<T> {
    fn invalidate(&self) {
        self.revision.fetch_add(1, Ordering::AcqRel);
    }
    fn snapshot(&self) -> Option<(u64, Arc<Vec<T>>)> {
        self.listed
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_ref()
            .map(|listed| (listed.revision, listed.value.clone()))
    }
    async fn list<F, Fut>(
        &self,
        cacheable: bool,
        closed: &CancellationToken,
        mut fetch: F,
    ) -> Result<Arc<Vec<T>>>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<Vec<T>>>,
    {
        tokio::time::timeout(std::time::Duration::from_secs(30), async {
            let _refresh = self.refresh.lock().await;
            if closed.is_cancelled() {
                bail!("MCP catalog connection is closed");
            }
            let revision = self.revision.load(Ordering::Acquire);
            if cacheable {
                let listed = self.listed.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                if let Some(listed) = listed.as_ref().filter(|listed| listed.revision == revision) {
                    return Ok(listed.value.clone());
                }
            }
            // Never publish a mix of pages fetched on either side of a notification.
            for _ in 0..2 {
                let revision = self.revision.load(Ordering::Acquire);
                let value = tokio::select! {
                    biased;
                    _ = closed.cancelled() => bail!("MCP catalog connection is closed"),
                    value = fetch() => value?,
                };
                if closed.is_cancelled() {
                    bail!("MCP catalog connection is closed");
                }
                if self.revision.load(Ordering::Acquire) != revision {
                    continue;
                }
                let value = Arc::new(value);
                *self.listed.lock().unwrap_or_else(std::sync::PoisonError::into_inner) =
                    Some(Listed { revision, value: value.clone() });
                return Ok(value);
            }
            bail!("MCP catalog changed during pagination; refresh remains pending")
        })
        .await
        .map_err(|_| anyhow::anyhow!("MCP catalog refresh timed out"))?
    }
}

pub struct McpCatalogs {
    closed: CancellationToken,
    tools: Catalog<Tool>,
    resources: Catalog<serde_json::Value>,
    templates: Catalog<serde_json::Value>,
    prompts: Catalog<serde_json::Value>,
}
impl McpCatalogs {
    pub fn close(&self) {
        self.closed.cancel();
    }
    pub fn new(closed: CancellationToken) -> Self {
        Self {
            closed,
            tools: Catalog::default(),
            resources: Catalog::default(),
            templates: Catalog::default(),
            prompts: Catalog::default(),
        }
    }
    pub fn tools_revision(&self) -> u64 {
        self.tools.revision.load(Ordering::Acquire)
    }
    pub fn tools_changed(&self) {
        if !self.closed.is_cancelled() {
            self.tools.invalidate();
        }
    }
    pub fn resources_changed(&self) {
        if !self.closed.is_cancelled() {
            self.resources.invalidate();
            self.templates.invalidate();
        }
    }
    pub fn prompts_changed(&self) {
        if !self.closed.is_cancelled() {
            self.prompts.invalidate();
        }
    }
    pub async fn tools(&self, peer: &Peer<RoleClient>) -> Result<Arc<Vec<Tool>>> {
        let cacheable = peer
            .peer_info()
            .and_then(|i| i.capabilities.tools.as_ref())
            .and_then(|c| c.list_changed)
            == Some(true);
        self.tools.list(cacheable, &self.closed, || crate::catalog::list_tools(peer)).await
    }
    pub async fn resources<F, Fut>(&self, cacheable: bool, fetch: F) -> Result<Arc<Vec<serde_json::Value>>>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<Vec<serde_json::Value>>>,
    {
        self.resources.list(cacheable, &self.closed, fetch).await
    }
    pub async fn templates<F, Fut>(&self, cacheable: bool, fetch: F) -> Result<Arc<Vec<serde_json::Value>>>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<Vec<serde_json::Value>>>,
    {
        self.templates.list(cacheable, &self.closed, fetch).await
    }
    pub async fn prompts<F, Fut>(&self, cacheable: bool, fetch: F) -> Result<Arc<Vec<serde_json::Value>>>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<Vec<serde_json::Value>>>,
    {
        self.prompts.list(cacheable, &self.closed, fetch).await
    }
    pub fn binding(self: &Arc<Self>) -> Option<ToolCatalogBinding> {
        let (revision, tools) = self.tools.snapshot()?;
        Some(ToolCatalogBinding { catalog: self.clone(), revision, tools })
    }
}

#[derive(Clone)]
pub struct ToolCatalogBinding {
    catalog: Arc<McpCatalogs>,
    revision: u64,
    tools: Arc<Vec<Tool>>,
}
impl ToolCatalogBinding {
    pub fn same_catalog(&self, other: &Self) -> bool {
        self.revision == other.revision
            && other.revision == other.catalog.tools.revision.load(Ordering::Acquire)
            && self.matches(&other.catalog, &other.tools)
    }
    pub fn matches(&self, current: &Arc<McpCatalogs>, tools: &[Tool]) -> bool {
        if !Arc::ptr_eq(&self.catalog, current)
            || self.catalog.closed.is_cancelled()
            || self.tools.len() != tools.len()
        {
            return false;
        }
        let mut before = self.tools.iter().collect::<Vec<_>>();
        let mut after = tools.iter().collect::<Vec<_>>();
        before.sort_by(|a, b| a.name.cmp(&b.name));
        after.sort_by(|a, b| a.name.cmp(&b.name));
        before == after
    }
}
