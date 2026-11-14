//! Process-wide exclusion for authorized tenant/host sync operations.

use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex, OnceLock, Weak};

use anyhow::{bail, Result};
use tokio::sync::{Mutex as AsyncMutex, OwnedMutexGuard};
use tokio_util::sync::CancellationToken;
use vw_api_types::device_sync_task::SyncTaskOrigin;

// A tuple avoids collisions between tenant and host strings. Transport, origin,
// principal and connection are deliberately absent: all share this business lock.
type Scope = (String, String);
static LOCKS: OnceLock<Mutex<HashMap<Scope, Weak<AsyncMutex<()>>>>> = OnceLock::new();

/// Fresh connection state returned by the authorized storage/credential owner.
/// `material` must contain only the newly read configuration and its bound lease;
/// it intentionally has no Debug or serialization implementation.
pub struct CurrentSyncConnection<C> {
    pub enabled: bool,
    pub auto_sync: bool,
    pub material: C,
}

/// Also used by connection mutation/clear paths, so they cannot invalidate a
/// lease during an operation. Inputs are canonical IDs from trusted authorization,
/// never request-provided tenant/host aliases. This function grants no permission.
pub(crate) async fn acquire_sync_lock(
    tenant: &str,
    host: &str,
    cancellation: &CancellationToken,
) -> Result<OwnedMutexGuard<()>> {
    if tenant.trim().is_empty() || host.trim().is_empty() {
        bail!("device sync requires an authorized tenant and execution host");
    }
    if cancellation.is_cancelled() {
        bail!("device sync cancelled while waiting for operation lock");
    }
    let lock = {
        let mut locks = LOCKS
            .get_or_init(|| Mutex::new(HashMap::new()))
            .lock()
            .map_err(|_| anyhow::anyhow!("device sync lock registry unavailable"))?;
        // Keep live owners and waiters; discard inactive scopes without leaking
        // a permanent registry entry for every tenant seen by this process.
        locks.retain(|_, lock| lock.strong_count() > 0);
        let key = (tenant.to_owned(), host.to_owned());
        match locks.get(&key).and_then(Weak::upgrade) {
            Some(lock) => lock,
            None => {
                let lock = Arc::new(AsyncMutex::new(()));
                locks.insert(key, Arc::downgrade(&lock));
                lock
            }
        }
    };
    tokio::select! {
        biased;
        () = cancellation.cancelled() => {
            bail!("device sync cancelled while waiting for operation lock");
        }
        guard = lock.lock_owned() => Ok(guard),
    }
}

/// Serialize either transport/direction and manual/automatic origins.
///
/// `reload` runs only after acquisition and must read the selected connection
/// and obtain a lease bound to its current identity from the real owners. None
/// means deleted. Never capture a previously read credential in `operation`.
/// `operation` must await the entire export/transfer or apply/rollback/projection
/// sequence inline; detached work would escape the lock. After entry it owns
/// cancellation/commit semantics: we never drop it in response to the token.
pub async fn run_with_sync_lock<C, T, Reload, ReloadFuture, Operation, OperationFuture>(
    tenant: &str,
    host: &str,
    origin: SyncTaskOrigin,
    cancellation: &CancellationToken,
    reload: Reload,
    operation: Operation,
) -> Result<T>
where
    Reload: FnOnce() -> ReloadFuture,
    ReloadFuture: Future<Output = Result<Option<CurrentSyncConnection<C>>>>,
    Operation: FnOnce(C) -> OperationFuture,
    OperationFuture: Future<Output = Result<T>>,
{
    let _guard = acquire_sync_lock(tenant, host, cancellation).await?;
    if cancellation.is_cancelled() {
        bail!("device sync cancelled before configuration reload");
    }
    let Some(connection) = reload().await? else {
        bail!("device sync connection was deleted");
    };
    if !connection.enabled {
        bail!("device sync connection is disabled");
    }
    if origin == SyncTaskOrigin::Auto && !connection.auto_sync {
        bail!("automatic device sync is disabled");
    }
    if cancellation.is_cancelled() {
        bail!("device sync cancelled before operation start");
    }
    operation(connection.material).await
}
