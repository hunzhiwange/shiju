//! 会话持有的工具目录及连接租约。宿主类型通过明确契约保留身份。

use std::{fmt, sync::Arc};
use tool_types::Tool;

/// 已验证工具目录的运行工具视图；目录授权与版本仍由实际目录实现负责。
pub trait SessionCatalog: Default {
    fn from_runtime_tools(tools: Vec<Arc<dyn Tool>>) -> Self;
    fn tools(&self) -> &[Arc<dyn Tool>];
}

/// 最后一个会话租约释放时关闭连接，保持原会话生命周期。
pub trait SessionConnections {
    fn shutdown(&self);
}

/// 只属于单个原生会话的运行时工具集合。
pub struct SessionToolset<C: SessionCatalog, M: SessionConnections> {
    catalog: Arc<C>,
    mcp_lease: Option<Arc<McpSessionLease<M>>>,
}

struct McpSessionLease<M: SessionConnections>(Arc<M>);
impl<M: SessionConnections> Drop for McpSessionLease<M> {
    fn drop(&mut self) {
        self.0.shutdown();
    }
}

impl<C: SessionCatalog, M: SessionConnections> fmt::Debug for SessionToolset<C, M> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("SessionToolset").field("len", &self.catalog.tools().len()).finish()
    }
}

impl<C: SessionCatalog, M: SessionConnections> SessionToolset<C, M> {
    /// 从已完成启动和发现的工具构造会话级集合。
    pub fn new(tools: Vec<Arc<dyn Tool>>) -> Self {
        Self {
            catalog: Arc::new(C::from_runtime_tools(tools)),
            mcp_lease: None,
        }
    }

    pub fn with_mcp_connections(
        mut self,
        connections: Arc<M>,
    ) -> Self {
        self.mcp_lease = Some(Arc::new(McpSessionLease(connections)));
        self
    }

    pub fn mcp_connections(&self) -> Option<Arc<M>> {
        self.mcp_lease.as_ref().map(|lease| lease.0.clone())
    }

    /// 返回本会话的运行时工具。
    pub fn tools(&self) -> &[Arc<dyn Tool>] {
        self.catalog.tools()
    }

    /// Retain a validated catalog version, including its executor ownership.
    pub fn with_catalog(
        mut self,
        catalog: Arc<C>,
    ) -> Self {
        self.catalog = catalog;
        self
    }

    pub fn catalog(&self) -> &Arc<C> {
        &self.catalog
    }

    /// 返回会话级工具数量。
    pub fn len(&self) -> usize {
        self.catalog.tools().len()
    }

    /// 返回集合是否为空。
    pub fn is_empty(&self) -> bool {
        self.catalog.tools().is_empty()
    }
}

impl<C: SessionCatalog, M: SessionConnections> Clone for SessionToolset<C, M> {
    fn clone(&self) -> Self {
        Self { catalog: self.catalog.clone(), mcp_lease: self.mcp_lease.clone() }
    }
}

impl<C: SessionCatalog, M: SessionConnections> Default for SessionToolset<C, M> {
    fn default() -> Self {
        Self { catalog: Arc::new(C::default()), mcp_lease: None }
    }
}
