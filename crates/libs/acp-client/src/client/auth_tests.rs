//! 验证认证策略、凭据来源优先级及代理认证错误映射。
//!
//! 隔离的环境变量与协议连接 fixture 用于固定凭据来源及认证策略边界。

use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex, MutexGuard};

use agent_client_protocol as acp;
use async_trait::async_trait;
use parking_lot::Mutex as ParkingMutex;
use tokio_util::compat::TokioAsyncReadCompatExt;

use crate::types::{AcpAgentConfig, AuthPolicy};

use super::{AcpClient, AcpError};

static ENV_TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

#[derive(Clone, Default)]
struct TestClient;

#[async_trait(?Send)]
impl acp::Client for TestClient {
    async fn request_permission(
        &self,
        _args: acp::RequestPermissionRequest,
    ) -> acp::Result<acp::RequestPermissionResponse> {
        Err(acp::Error::method_not_found())
    }

    async fn session_notification(&self, _args: acp::SessionNotification) -> acp::Result<()> {
        Ok(())
    }
}

#[derive(Clone)]
struct TestAgent {
    authenticated: Arc<ParkingMutex<Vec<String>>>,
    fail_authenticate: bool,
}

impl TestAgent {
    fn new() -> Self {
        Self { authenticated: Arc::new(ParkingMutex::new(Vec::new())), fail_authenticate: false }
    }

    fn failing() -> Self {
        Self { authenticated: Arc::new(ParkingMutex::new(Vec::new())), fail_authenticate: true }
    }

    fn authenticated_methods(&self) -> Vec<String> {
        self.authenticated.lock().clone()
    }
}

#[async_trait(?Send)]
impl acp::Agent for TestAgent {
    async fn initialize(
        &self,
        args: acp::InitializeRequest,
    ) -> acp::Result<acp::InitializeResponse> {
        Ok(acp::InitializeResponse::new(args.protocol_version))
    }

    async fn authenticate(
        &self,
        args: acp::AuthenticateRequest,
    ) -> acp::Result<acp::AuthenticateResponse> {
        if self.fail_authenticate {
            return Err(acp::Error::invalid_params());
        }

        self.authenticated.lock().push(args.method_id.0.to_string());
        Ok(acp::AuthenticateResponse::default())
    }

    async fn new_session(
        &self,
        _args: acp::NewSessionRequest,
    ) -> acp::Result<acp::NewSessionResponse> {
        Err(acp::Error::method_not_found())
    }

    async fn prompt(&self, _args: acp::PromptRequest) -> acp::Result<acp::PromptResponse> {
        Err(acp::Error::method_not_found())
    }

    async fn cancel(&self, _args: acp::CancelNotification) -> acp::Result<()> {
        Ok(())
    }
}

struct EnvGuard {
    key: &'static str,
    original: Option<std::ffi::OsString>,
    _lock: MutexGuard<'static, ()>,
}

impl EnvGuard {
    fn set(key: &'static str, value: &str) -> Self {
        // Tests serialize environment mutation because process env is global.
        let lock = ENV_TEST_LOCK.lock().expect("env test lock should acquire");
        let original = std::env::var_os(key);
        unsafe { std::env::set_var(key, value) };
        Self { key, original, _lock: lock }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match &self.original {
            Some(value) => unsafe { std::env::set_var(self.key, value) },
            None => unsafe { std::env::remove_var(self.key) },
        }
    }
}

fn client() -> AcpClient {
    AcpClient::new(
        "test-agent",
        AcpAgentConfig { command: "agent".to_string(), args: Vec::new(), env: HashMap::new() },
    )
}

fn auth_method(id: &'static str) -> acp::AuthMethod {
    acp::AuthMethod::Agent(acp::AuthMethodAgent::new(id, id))
}

fn connection_pair(agent: TestAgent) -> (acp::ClientSideConnection, TestAgent) {
    let (client_out, agent_in) = tokio::io::duplex(4096);
    let (agent_out, client_in) = tokio::io::duplex(4096);

    let (client_conn, client_io_task) = acp::ClientSideConnection::new(
        TestClient,
        client_out.compat(),
        client_in.compat(),
        |fut| {
            tokio::task::spawn_local(fut);
        },
    );
    let (agent_conn, agent_io_task) = acp::AgentSideConnection::new(
        agent.clone(),
        agent_out.compat(),
        agent_in.compat(),
        |fut| {
            tokio::task::spawn_local(fut);
        },
    );

    tokio::task::spawn_local(client_io_task);
    tokio::task::spawn_local(agent_io_task);
    drop(agent_conn);

    (client_conn, agent)
}

async fn with_connection<R>(
    agent: TestAgent,
    f: impl AsyncFnOnce(acp::ClientSideConnection, TestAgent) -> R,
) -> R {
    let local_set = tokio::task::LocalSet::new();
    local_set
        .run_until(async {
            let (conn, agent) = connection_pair(agent);
            f(conn, agent).await
        })
        .await
}

// 下列行为用例覆盖凭据优先级、空值过滤和认证失败映射。
#[tokio::test]
async fn authenticate_if_required_ignores_empty_method_list() {
    with_connection(TestAgent::new(), async |conn, agent| {
        client()
            .authenticate_if_required(&conn, &[])
            .await
            .expect("empty methods should not require authentication");

        assert!(agent.authenticated_methods().is_empty());
    })
    .await;
}

#[tokio::test]
async fn authenticate_if_required_skips_missing_credentials_by_default() {
    with_connection(TestAgent::new(), async |conn, agent| {
        client()
            .authenticate_if_required(&conn, &[auth_method("testcov-0013-missing")])
            .await
            .expect("missing credentials should be skipped by default");

        assert!(agent.authenticated_methods().is_empty());
    })
    .await;
}

#[tokio::test]
async fn authenticate_if_required_logs_verbose_skip_without_credentials() {
    with_connection(TestAgent::new(), async |conn, agent| {
        client()
            .with_verbose(true)
            .authenticate_if_required(&conn, &[auth_method("testcov-0013-verbose-missing")])
            .await
            .expect("verbose missing credentials should still skip");

        assert!(agent.authenticated_methods().is_empty());
    })
    .await;
}

#[tokio::test]
async fn authenticate_if_required_fails_missing_credentials_when_policy_requires_auth() {
    with_connection(TestAgent::new(), async |conn, agent| {
        let err = client()
            .with_auth_policy(AuthPolicy::Fail)
            .authenticate_if_required(
                &conn,
                &[auth_method("testcov-0013-primary"), auth_method("testcov-0013-secondary")],
            )
            .await
            .expect_err("missing credentials should fail under fail policy");

        assert!(matches!(err, AcpError::Initialize(_)));
        assert!(err.to_string().contains("testcov-0013-primary"));
        assert!(err.to_string().contains("testcov-0013-secondary"));
        assert!(agent.authenticated_methods().is_empty());
    })
    .await;
}

#[tokio::test]
async fn authenticate_if_required_uses_first_non_empty_config_credential() {
    let credentials = HashMap::from([
        ("testcov-0013-empty".to_string(), "  ".to_string()),
        ("testcov-0013-config".to_string(), "secret".to_string()),
    ]);

    with_connection(TestAgent::new(), async |conn, agent| {
        client()
            .with_auth_credentials(credentials)
            .with_verbose(true)
            .authenticate_if_required(
                &conn,
                &[auth_method("testcov-0013-empty"), auth_method("testcov-0013-config")],
            )
            .await
            .expect("config credential should authenticate");

        assert_eq!(agent.authenticated_methods(), vec!["testcov-0013-config"]);
    })
    .await;
}

#[tokio::test]
async fn authenticate_if_required_uses_normalized_config_key_with_original_method_id() {
    let credentials =
        HashMap::from([("TESTCOV_0013_NORMALIZED".to_string(), "secret".to_string())]);

    with_connection(TestAgent::new(), async |conn, agent| {
        client()
            .with_auth_credentials(credentials)
            .authenticate_if_required(&conn, &[auth_method("testcov-0013-normalized")])
            .await
            .expect("normalized credential should authenticate");

        assert_eq!(agent.authenticated_methods(), vec!["testcov-0013-normalized"]);
    })
    .await;
}

#[tokio::test]
async fn authenticate_if_required_prefers_environment_credentials() {
    let _env = EnvGuard::set("VWACP_AUTH_TESTCOV_0013_ENV_ONLY", "secret");

    with_connection(TestAgent::new(), async |conn, agent| {
        client()
            .with_auth_credentials(HashMap::from([(
                "testcov-0013-config-only".to_string(),
                "secret".to_string(),
            )]))
            .authenticate_if_required(
                &conn,
                &[auth_method("testcov-0013-env-only"), auth_method("testcov-0013-config-only")],
            )
            .await
            .expect("env credential should authenticate");

        assert_eq!(agent.authenticated_methods(), vec!["testcov-0013-env-only"]);
    })
    .await;
}

#[tokio::test]
async fn authenticate_if_required_maps_agent_authentication_errors() {
    with_connection(TestAgent::failing(), async |conn, agent| {
        let err = client()
            .with_auth_credentials(HashMap::from([(
                "testcov-0013-agent-error".to_string(),
                "secret".to_string(),
            )]))
            .authenticate_if_required(&conn, &[auth_method("testcov-0013-agent-error")])
            .await
            .expect_err("agent authentication error should be mapped");

        assert!(matches!(err, AcpError::Initialize(_)));
        assert!(agent.authenticated_methods().is_empty());
    })
    .await;
}
