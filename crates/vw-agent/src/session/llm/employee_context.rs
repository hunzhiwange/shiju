//! Carries the verified employee launch through the synchronous processor and async ACP worker.
use llm_runtime::acp::host::{EmployeeAcpKind, EmployeeAcpLaunch};
use session_engine::agent_binding::{EmployeeKind, EmployeeTarget};
use std::sync::Arc;
use vw_api_transport::TenantScope;

pub(crate) struct Launch {
    tenant: TenantScope,
    session: String,
    data: EmployeeAcpLaunch,
}

impl Launch {
    pub(crate) fn new(
        tenant: TenantScope,
        session: String,
        target: EmployeeTarget,
        credential: String,
        stop: tokio_util::sync::CancellationToken,
    ) -> Arc<Self> {
        use sha2::{Digest, Sha256};

        let scope = serde_json::to_vec(&(
            tenant.platform_id().as_ref(),
            tenant.company_id().as_ref(),
            tenant.user_id().as_ref(),
        ))
        .expect("serializable employee account scope");
        let account_scope = hex::encode(Sha256::digest(scope));
        let kind = match target.kind {
            EmployeeKind::Private => EmployeeAcpKind::Private,
            EmployeeKind::Shared => EmployeeAcpKind::Shared,
        };
        Arc::new(Self {
            tenant,
            session,
            data: EmployeeAcpLaunch {
                employee_id: target.employee_id().to_owned(),
                kind,
                account_scope,
                state_dir: crate::global::paths().data.join("employee-acp"),
                credential,
                stop: Some(stop),
            },
        })
    }

    fn matches(&self, tenant: &TenantScope, session: &str) -> bool {
        self.tenant == *tenant && self.session == session
    }
}

std::thread_local! {
    static PROCESSOR_LAUNCH: std::cell::RefCell<Option<Arc<Launch>>> = const { std::cell::RefCell::new(None) };
}
tokio::task_local! {
    static WORKER_LAUNCH: Option<Arc<Launch>>;
}

pub(crate) fn with_processor_launch<T>(launch: Option<Arc<Launch>>, run: impl FnOnce() -> T) -> T {
    // The processor owns a dedicated thread. Pass an explicit copy to its LLM worker.
    struct Restore(Option<Arc<Launch>>);
    impl Drop for Restore {
        fn drop(&mut self) {
            PROCESSOR_LAUNCH.with(|slot| slot.replace(self.0.take()));
        }
    }
    let _restore = Restore(PROCESSOR_LAUNCH.with(|slot| slot.replace(launch)));
    run()
}

pub(crate) fn processor_launch(tenant: &TenantScope, session: &str) -> Option<Arc<Launch>> {
    PROCESSOR_LAUNCH.with(|launch| {
        launch.borrow().as_ref().filter(|launch| launch.matches(tenant, session)).cloned()
    })
}

pub(crate) async fn with_worker_launch<T>(
    launch: Option<Arc<Launch>>,
    run: impl Future<Output = T>,
) -> T {
    WORKER_LAUNCH.scope(launch, run).await
}

pub(super) fn worker_launch(tenant: &TenantScope, session: &str) -> Option<EmployeeAcpLaunch> {
    WORKER_LAUNCH
        .try_with(|launch| {
            launch
                .as_ref()
                .filter(|launch| launch.matches(tenant, session))
                .map(|launch| launch.data.clone())
        })
        .ok()
        .flatten()
}

#[cfg(test)]
pub(crate) static TEST_EXECUTABLE: parking_lot::Mutex<Option<(TenantScope, std::path::PathBuf)>> =
    parking_lot::Mutex::new(None);

#[cfg(test)]
pub(super) fn test_executable(tenant: &TenantScope) -> Option<std::path::PathBuf> {
    TEST_EXECUTABLE
        .lock()
        .as_ref()
        .filter(|(scope, _)| scope == tenant)
        .map(|(_, path)| path.clone())
}
