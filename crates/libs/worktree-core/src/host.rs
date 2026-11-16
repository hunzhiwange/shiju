//! Narrow host actions. No session engine or process implementation is owned here.
use crate::Error;
use std::{collections::{HashMap, HashSet}, future::Future, sync::{Arc, OnceLock}};
use vw_api_transport::{ProjectScope, TenantScope};

pub struct ProtectionSnapshot {
    pub sessions: Vec<session_info::Info>,
    pub foreign_directories: Vec<String>,
    pub pinned: HashMap<String, HashSet<String>>,
    /// Sessions whose existing runtime status is Busy or Retry.
    pub busy: HashSet<String>,
}

/// The guard's destructor must terminate descendants, including on cancellation.
pub trait ProcessGuard: Send {}
pub type ScopedFuture<'a> = std::pin::Pin<Box<dyn Future<Output = Result<(), Error>> + Send + 'a>>;

pub trait Host: Send + Sync {
    fn bootstrap(&self, directory: std::path::PathBuf) -> ScopedFuture<'static>;
    fn with_tenant_scope<'a>(&self, tenant: TenantScope, future: ScopedFuture<'a>) -> ScopedFuture<'a>;
    fn current_tenant_scope(&self) -> TenantScope;
    fn read_snapshot(&self, scope: &ProjectScope) -> Result<ProtectionSnapshot, Error>;
    fn spawn(&self, command: &mut tokio::process::Command)
        -> anyhow::Result<(tokio::process::Child, Box<dyn ProcessGuard>)>;
}
static HOST: OnceLock<Arc<dyn Host>> = OnceLock::new();
pub fn install(host: Arc<dyn Host>) -> Result<(), Error> {
    HOST.set(host).map_err(|_| Error::Invalid("Worktree host already installed".into()))
}
fn host() -> Result<&'static dyn Host, Error> {
    HOST.get().map(Arc::as_ref).ok_or_else(|| Error::Invalid("Worktree host is not installed".into()))
}
tokio::task_local! { static TENANT: TenantScope; }
pub fn current_tenant_scope() -> Result<TenantScope, Error> {
    match TENANT.try_with(Clone::clone) {
        Ok(tenant) => Ok(tenant),
        Err(_) => Ok(host()?.current_tenant_scope()),
    }
}
pub async fn with_tenant_scope(tenant: TenantScope, future: impl Future<Output = Result<(), Error>> + Send) -> Result<(), Error> {
    let scoped = TENANT.scope(tenant.clone(), future);
    host()?.with_tenant_scope(tenant, Box::pin(scoped)).await
}
pub fn read_snapshot(scope: &ProjectScope) -> Result<ProtectionSnapshot, Error> {
    host()?.read_snapshot(scope)
}
pub fn spawn(command: &mut tokio::process::Command)
    -> anyhow::Result<(tokio::process::Child, Box<dyn ProcessGuard>)> {
    host().map_err(|error| anyhow::anyhow!(error.to_string()))?.spawn(command)
}

pub async fn bootstrap(directory: std::path::PathBuf) -> Result<(), Error> {
    host()?.bootstrap(directory).await
}
