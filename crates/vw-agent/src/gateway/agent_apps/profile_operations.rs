//! Outer composition lock rank: tenant + executor, then domain catalog locks,
//! then native app/directory guards, then each domain's storage transaction.
//! Never pre-acquire native guards: the domain entry points acquire their own.

use std::{
    cell::Cell,
    collections::HashSet,
    marker::PhantomData,
    rc::Rc,
    sync::{Condvar, Mutex, OnceLock},
};

use anyhow::{Result, bail, ensure};
use vw_api_types::id::TenantScope;

#[derive(Clone, PartialEq, Eq, Hash)]
enum Resource {
    // Shared entities conflict across every host/group in their tenant.
    Tenant(TenantScope),
    // Physical native slots may alias across tenants and application groups.
    // Use trusted executor identity, never a requested host alias or project ID.
    Executor(String),
}

type Locks = (Mutex<HashSet<Resource>>, Condvar);
fn locks() -> &'static Locks {
    static LOCKS: OnceLock<Locks> = OnceLock::new();
    LOCKS.get_or_init(|| (Mutex::new(HashSet::new()), Condvar::new()))
}

thread_local! {
    static IN_OPERATION: Cell<bool> = const { Cell::new(false) };
}

// Not Send/Sync: release and recursion detection belong to the acquiring worker.
pub(super) struct ProfileWriteGuard {
    resources: [Resource; 2],
    _worker: PhantomData<Rc<()>>,
}

/// Blocking, non-reentrant executor boundary. Acquire all outer resources at
/// once, so different tenant/host combinations cannot invert the lock order.
/// RAII releases on success, error and unwinding; no mutex is held during work.
pub(super) fn lock(tenant: &TenantScope, executor: &str) -> Result<ProfileWriteGuard> {
    ensure!(!executor.trim().is_empty(), "empty profile executor identity");
    if IN_OPERATION.with(Cell::get) {
        bail!("nested project profile operation is not supported");
    }
    let resources = [Resource::Tenant(tenant.clone()), Resource::Executor(executor.to_owned())];
    let (held, released) = locks();
    let mut held = held.lock().map_err(|_| anyhow::anyhow!("profile lock unavailable"))?;
    while resources.iter().any(|key| held.contains(key)) {
        held = released.wait(held).map_err(|_| anyhow::anyhow!("profile lock unavailable"))?;
    }
    held.extend(resources.iter().cloned());
    IN_OPERATION.with(|active| active.set(true));
    Ok(ProfileWriteGuard { resources, _worker: PhantomData })
}

impl Drop for ProfileWriteGuard {
    fn drop(&mut self) {
        let (held, released) = locks();
        let mut held = held.lock().unwrap_or_else(|error| error.into_inner());
        for resource in &self.resources {
            held.remove(resource);
        }
        IN_OPERATION.with(|active| active.set(false));
        released.notify_all();
    }
}
