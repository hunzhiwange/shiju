//! Host-owned Codex authorization-code flow with an IPv4 loopback callback.

use super::{
    browser_callback::LoopbackCallback,
    codex_device::{CLIENT_ID, complete_account, exchange_code, oauth_client, validate_start},
    copilot::utc_now,
    credential_file::AccountCredential,
    flow::{AuthAccountScope, AuthFlowTiming, AuthFlows},
};
use reqwest::{Client, Request, Url};
use std::time::{Duration, Instant};
use vw_api_types::{
    TimestampMs,
    agent_apps_auth::{
        AuthAccount, AuthErrorCode, AuthFlow, AuthFlowId, AuthLoginMethod, AuthStartRequest,
        AuthStartResponse,
    },
};

const CALLBACK_PORT: u16 = 1455;
const CALLBACK_PATH: &str = "/auth/callback";
const LOGIN_LIFETIME: Duration = Duration::from_secs(900);

/// Only the start response leaves the host. The bound listener and verifier
/// belong to this attempt and are cleared on cancellation or expiry.
pub struct CodexBrowserStart {
    pub response: AuthStartResponse,
    flow_id: AuthFlowId,
    target_account_id: Option<String>,
    redirect_uri: String,
    client: Client,
    callback: LoopbackCallback,
    polling: tokio::sync::Mutex<()>,
}

impl CodexBrowserStart {
    /// `browser_on_execution_host` comes from trusted host routing. A browser
    /// on a different host cannot use this fixed localhost redirect.
    pub fn start(
        flows: &AuthFlows,
        scope: AuthAccountScope<'_>,
        request: AuthStartRequest,
        browser_on_execution_host: bool,
    ) -> Result<Self, AuthErrorCode> {
        Self::start_on(
            flows,
            scope,
            request,
            browser_on_execution_host,
            CALLBACK_PORT,
            LOGIN_LIFETIME,
        )
    }

    pub(super) fn start_on(
        flows: &AuthFlows,
        scope: AuthAccountScope<'_>,
        request: AuthStartRequest,
        browser_on_execution_host: bool,
        port: u16,
        lifetime: Duration,
    ) -> Result<Self, AuthErrorCode> {
        validate_start(scope, &request, AuthLoginMethod::Browser)?;
        if !browser_on_execution_host {
            return Err(AuthErrorCode::CallbackUnreachable);
        }

        let client = oauth_client()?;
        let deadline = Instant::now() + lifetime;
        let expires_at = TimestampMs(utc_now()?.0 + lifetime.as_millis() as i64);
        let (callback, challenge) =
            LoopbackCallback::bind(port, CALLBACK_PATH, "codex-oauth-callback", deadline)?;
        let redirect_uri = format!("http://localhost:{}{CALLBACK_PATH}", challenge.port);
        let verification_url = authorize_url(&redirect_uri, &challenge.state, &challenge.challenge);
        let target_account_id = request.target_account_id.clone();
        let flow = flows.start_with_cleanup(
            scope,
            request,
            AuthFlowTiming { deadline, expires_at, retry_after_seconds: 1 },
            callback.cleanup(),
        )?;
        let AuthFlow::Pending { flow_id, expires_at, retry_after_seconds } = flow else {
            unreachable!("registering a flow always returns its pending snapshot");
        };
        let response = AuthStartResponse::Pending {
            flow_id: flow_id.clone(),
            expires_at,
            retry_after_seconds,
            verification_url,
            user_code: None,
            can_paste: None,
        };
        Ok(Self {
            response,
            flow_id,
            target_account_id,
            redirect_uri,
            client,
            callback,
            polling: tokio::sync::Mutex::new(()),
        })
    }

    pub async fn poll(
        &self,
        flows: &AuthFlows,
        scope: AuthAccountScope<'_>,
        persist: impl FnOnce(
            AuthAccountScope<'_>,
            Option<&str>,
            &AuthAccount,
            &str,
            &AccountCredential,
        ) -> Result<(), AuthErrorCode>,
    ) -> Result<AuthFlow, AuthErrorCode> {
        self.poll_with(flows, scope, |_| {}, persist).await
    }

    pub(super) async fn poll_with(
        &self,
        flows: &AuthFlows,
        scope: AuthAccountScope<'_>,
        mut route: impl FnMut(&mut Request),
        persist: impl FnOnce(
            AuthAccountScope<'_>,
            Option<&str>,
            &AuthAccount,
            &str,
            &AccountCredential,
        ) -> Result<(), AuthErrorCode>,
    ) -> Result<AuthFlow, AuthErrorCode> {
        let snapshot = flows.snapshot(scope, &self.flow_id, Instant::now)?;
        if !matches!(snapshot, AuthFlow::Pending { .. }) {
            return Ok(snapshot);
        }
        let Ok(_poll) = self.polling.try_lock() else {
            return Ok(snapshot);
        };
        let result = match self.callback.take()? {
            None => return Ok(snapshot),
            Some(Ok(grant)) => exchange_code(
                &self.client,
                &grant.code,
                &grant.verifier,
                &self.redirect_uri,
                &mut route,
            )
            .await
            .and_then(|tokens| complete_account(scope, self.target_account_id.as_deref(), tokens)),
            Some(Err(error)) => Err(error),
        };
        let completed = match result {
            Ok((account, (identity, credential))) => flows.complete(
                scope,
                &self.flow_id,
                Instant::now,
                Ok(account),
                |bound, request, account| {
                    persist(
                        bound,
                        request.target_account_id.as_deref(),
                        account,
                        &identity,
                        &credential,
                    )
                },
            ),
            Err(error) => {
                flows.complete(scope, &self.flow_id, Instant::now, Err(error), |_, _, _| Ok(()))
            }
        };
        match completed {
            Err(AuthErrorCode::Conflict) => {
                let snapshot = flows.snapshot(scope, &self.flow_id, Instant::now)?;
                if matches!(snapshot, AuthFlow::Pending { .. }) {
                    return Err(AuthErrorCode::Conflict);
                }
                Ok(snapshot)
            }
            result => result,
        }
    }
}

fn authorize_url(redirect_uri: &str, state: &str, challenge: &str) -> String {
    let mut url =
        Url::parse("https://auth.openai.com/oauth/authorize").expect("constant OAuth URL");
    url.query_pairs_mut().extend_pairs([
        ("response_type", "code"),
        ("client_id", CLIENT_ID),
        ("redirect_uri", redirect_uri),
        ("scope", "openid profile email offline_access api.connectors.read api.connectors.invoke"),
        ("code_challenge", challenge),
        ("code_challenge_method", "S256"),
        ("state", state),
        ("id_token_add_organizations", "true"),
        ("codex_cli_simplified_flow", "true"),
        ("originator", "codex_cli_rs"),
    ]);
    url.into()
}
