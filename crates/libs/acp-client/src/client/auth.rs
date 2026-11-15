//! ACP 客户端认证协商。
//!
//! 本模块只负责从代理声明的认证方法中选择可用凭据，并在初始化后执行
//! `authenticate` 请求。进程启动、会话控制和 prompt 循环仍由 actor 模块处理。

use super::*;

impl AcpClient {
    pub(super) async fn authenticate_if_required(
        &self,
        conn: &acp::ClientSideConnection,
        methods: &[acp::AuthMethod],
    ) -> Result<(), AcpError> {
        if methods.is_empty() {
            return Ok(());
        }

        let Some(selected) = self.select_auth_method(methods) else {
            if self.auth_policy == AuthPolicy::Fail {
                let method_ids = methods
                    .iter()
                    .map(|method| method.id().0.as_ref())
                    .collect::<Vec<_>>()
                    .join(", ");
                return Err(AcpError::Initialize(format!(
                    "agent advertised auth methods [{method_ids}] but no matching credentials found"
                )));
            }

            if self.verbose {
                let method_ids = methods
                    .iter()
                    .map(|method| method.id().0.as_ref())
                    .collect::<Vec<_>>()
                    .join(", ");
                tracing::info!(
                    target: "vw_acp",
                    acp_agent = %self.agent_name,
                    auth_methods = %method_ids,
                    "agent advertised auth methods but no matching credentials were found; skipping client authentication"
                );
            }
            return Ok(());
        };

        conn.authenticate(acp::AuthenticateRequest::new(selected.method_id.clone()))
            .await
            .map_err(|err| AcpError::Initialize(err.to_string()))?;

        if self.verbose {
            tracing::info!(
                target: "vw_acp",
                acp_agent = %self.agent_name,
                method_id = %selected.method_id,
                source = selected.source,
                "authenticated ACP client"
            );
        }

        Ok(())
    }

    fn select_auth_method(&self, methods: &[acp::AuthMethod]) -> Option<AuthSelection> {
        for method in methods {
            let method_id = method.id().0.as_ref();

            if read_env_credential(method_id).is_some() {
                return Some(AuthSelection { method_id: method_id.to_string(), source: "env" });
            }

            let normalized = to_env_token(method_id);
            let config_credential = self
                .auth_credentials
                .get(method_id)
                .or_else(|| normalized.as_ref().and_then(|key| self.auth_credentials.get(key)));

            if config_credential.is_some_and(|value| !value.trim().is_empty()) {
                return Some(AuthSelection { method_id: method_id.to_string(), source: "config" });
            }
        }

        None
    }
}
