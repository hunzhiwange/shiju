//! AGY OAuth on the authorized macOS execution host.

use super::{
    agy_account::lock_keychain,
    browser_callback::{CallbackChallenge, CallbackCode, LoopbackCallback},
    copilot::{read_json_limited, utc_now, valid_token},
    flow::{AuthAccountScope, AuthFlowTiming, AuthFlows},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use reqwest::{Client, Request, StatusCode, Url};
use security_framework::os::macos::keychain::{SecKeychain, SecPreferencesDomain};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::{Duration, Instant};
use vw_api_types::{
    TimestampMs,
    agent_apps_auth::{
        AuthAccount, AuthCredentialState, AuthErrorCode, AuthFlow, AuthFlowId, AuthService,
        AuthStartRequest, AuthStartResponse,
    },
};

const CLIENT_ID: &str = "1071006060591-tmhssin2h21lcre235vtolojh4g403ep.apps.googleusercontent.com";
// Google installed-app credential embedded in AGY, shared by all installations.
const CLIENT_SECRET_PARTS: [&str; 2] = ["GOCSPX-", "K58FWR486LdLJ1mLB8sXC4z6qDAf"];
const AUTH_ENDPOINT: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_ENDPOINT: &str = "https://oauth2.googleapis.com/token";
const CALLBACK_PATH: &str = "/oauth-callback";
const SCOPES: &str = concat!(
    "openid https://www.googleapis.com/auth/cloud-platform ",
    "https://www.googleapis.com/auth/userinfo.email https://www.googleapis.com/auth/userinfo.profile ",
    "https://www.googleapis.com/auth/cclog https://www.googleapis.com/auth/experimentsandconfigs ",
    "https://www.googleapis.com/auth/aicode",
);
const LOGIN_LIFETIME: Duration = Duration::from_secs(600);
const MAX_TOKEN_RESPONSE_BYTES: usize = 64 * 1024;
pub(super) const KEYCHAIN_ITEM_NOT_FOUND: i32 = -25300;

pub(super) const MANAGED_SERVICE: &str = "shiju.agy.oauth";

/// A host-side login attempt. The browser only receives `response`.
pub struct AgyLogin {
    pub response: AuthStartResponse,
    flow_id: AuthFlowId,
    redirect_uri: String,
    callback: LoopbackCallback,
    client: Client,
    keychain: SecKeychain,
    polling: tokio::sync::Mutex<()>,
}

impl AgyLogin {
    /// The host router determines whether the user's browser shares this loopback.
    pub fn start(
        flows: &AuthFlows,
        scope: AuthAccountScope<'_>,
        request: AuthStartRequest,
        browser_on_execution_host: bool,
    ) -> Result<Self, AuthErrorCode> {
        Self::start_with_lifetime(flows, scope, request, browser_on_execution_host, LOGIN_LIFETIME)
    }

    pub(super) fn start_with_lifetime(
        flows: &AuthFlows,
        scope: AuthAccountScope<'_>,
        request: AuthStartRequest,
        browser_on_execution_host: bool,
        lifetime: Duration,
    ) -> Result<Self, AuthErrorCode> {
        scope.validate()?;
        if scope.service != AuthService::AgyOauth {
            return Err(AuthErrorCode::Unsupported);
        }
        request.validate_for(scope.service)?;
        if !browser_on_execution_host {
            return Err(AuthErrorCode::CallbackUnreachable);
        }

        let client = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|_| AuthErrorCode::ExecutionHostUnavailable)?;
        let keychain = SecKeychain::default_for_domain(SecPreferencesDomain::User)
            .map_err(|_| AuthErrorCode::KeychainUnavailable)?;
        let deadline = Instant::now() + lifetime;
        let expires_at = TimestampMs(utc_now()?.0 + lifetime.as_millis() as i64);
        let (callback, challenge) =
            LoopbackCallback::bind(0, CALLBACK_PATH, "agy-oauth-callback", deadline)?;
        let redirect_uri = format!("http://127.0.0.1:{}{CALLBACK_PATH}", challenge.port);
        let verification_url = authorization_url(&redirect_uri, &challenge);
        let flow = flows.replace_with_cleanup(
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
            redirect_uri,
            callback,
            client,
            keychain,
            polling: tokio::sync::Mutex::new(()),
        })
    }

    pub async fn poll(
        &self,
        flows: &AuthFlows,
        scope: AuthAccountScope<'_>,
    ) -> Result<AuthFlow, AuthErrorCode> {
        self.poll_with(flows, scope, |_| {}).await
    }

    pub(super) async fn poll_with(
        &self,
        flows: &AuthFlows,
        scope: AuthAccountScope<'_>,
        route: impl FnOnce(&mut Request),
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
            Some(Ok(grant)) => {
                self.exchange(grant, route).await.and_then(|tokens| tokens.account(scope))
            }
            Some(Err(error)) => Err(error),
        };
        let completed = match result {
            Ok(record) => flows.complete(
                scope,
                &self.flow_id,
                Instant::now,
                Ok(record.account.clone()),
                |_, _, _| record.save(&self.keychain),
            ),
            Err(error) => {
                flows.complete(scope, &self.flow_id, Instant::now, Err(error), |_, _, _| Ok(()))
            }
        };
        if matches!(completed, Err(AuthErrorCode::Conflict)) {
            let snapshot = flows.snapshot(scope, &self.flow_id, Instant::now)?;
            if !matches!(snapshot, AuthFlow::Pending { .. }) {
                return Ok(snapshot);
            }
        }
        completed
    }

    async fn exchange(
        &self,
        grant: CallbackCode,
        route: impl FnOnce(&mut Request),
    ) -> Result<TokenResponse, AuthErrorCode> {
        let client_secret = CLIENT_SECRET_PARTS.concat();
        let mut request = self
            .client
            .post(TOKEN_ENDPOINT)
            .form(&[
                ("grant_type", "authorization_code"),
                ("code", grant.code.as_str()),
                ("redirect_uri", self.redirect_uri.as_str()),
                ("client_id", CLIENT_ID),
                ("client_secret", client_secret.as_str()),
                ("code_verifier", grant.verifier.as_str()),
            ])
            .build()
            .map_err(|_| AuthErrorCode::UpstreamFailure)?;
        route(&mut request);
        let response =
            self.client.execute(request).await.map_err(|_| AuthErrorCode::UpstreamFailure)?;
        if !response.status().is_success() {
            if response.status() == StatusCode::BAD_REQUEST {
                let error: TokenError =
                    read_json_limited(response, MAX_TOKEN_RESPONSE_BYTES).await?;
                if error.error == "invalid_grant" {
                    return Err(AuthErrorCode::ReauthRequired);
                }
            }
            return Err(AuthErrorCode::UpstreamFailure);
        }
        read_json_limited(response, MAX_TOKEN_RESPONSE_BYTES).await
    }
}

fn authorization_url(redirect_uri: &str, challenge: &CallbackChallenge) -> String {
    let mut url = Url::parse(AUTH_ENDPOINT).expect("constant OAuth URL");
    url.query_pairs_mut().extend_pairs([
        ("client_id", CLIENT_ID),
        ("redirect_uri", redirect_uri),
        ("response_type", "code"),
        ("scope", SCOPES),
        ("state", challenge.state.as_str()),
        ("code_challenge", challenge.challenge.as_str()),
        ("code_challenge_method", "S256"),
        ("access_type", "offline"),
        ("prompt", "select_account consent"),
    ]);
    url.into()
}

#[derive(Deserialize)]
struct TokenError {
    error: String,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    token_type: Option<String>,
    expires_in: Option<i64>,
    refresh_token: Option<String>,
    id_token: Option<String>,
}

#[derive(Deserialize)]
pub(super) struct Identity {
    pub(super) sub: String,
    email: Option<String>,
    name: Option<String>,
}

#[derive(Serialize)]
struct ManagedAccount {
    account: AuthAccount,
    credential: AgyCredential,
}

#[derive(Serialize)]
struct AgyCredential {
    token: AgyToken,
    auth_method: &'static str,
    id_token: String,
}

#[derive(Serialize)]
struct AgyToken {
    access_token: String,
    refresh_token: String,
    token_type: String,
    expiry: String,
    expires_in: i64,
}

impl TokenResponse {
    fn account(self, scope: AuthAccountScope<'_>) -> Result<ManagedAccount, AuthErrorCode> {
        let refresh_token = self
            .refresh_token
            .filter(|token| valid_token(token))
            .ok_or(AuthErrorCode::ReauthRequired)?;
        let id_token = self
            .id_token
            .filter(|token| valid_token(token))
            .ok_or(AuthErrorCode::UpstreamFailure)?;
        let token_type = self.token_type.unwrap_or_else(|| "Bearer".into());
        if !valid_token(&self.access_token) || !token_type.eq_ignore_ascii_case("Bearer") {
            return Err(AuthErrorCode::UpstreamFailure);
        }

        // Google returned this ID token directly over the fixed HTTPS exchange.
        let identity = token_identity(&id_token).ok_or(AuthErrorCode::UpstreamFailure)?;

        let now = utc_now()?;
        let expires_in = self.expires_in.unwrap_or(3599);
        let expiry = expires_in
            .checked_mul(1000)
            .filter(|duration| *duration > 0)
            .and_then(|duration| now.0.checked_add(duration))
            .and_then(chrono::DateTime::from_timestamp_millis)
            .ok_or(AuthErrorCode::UpstreamFailure)?;
        let label = identity
            .email
            .filter(|label| valid_label(label))
            .or_else(|| identity.name.filter(|label| valid_label(label)))
            .unwrap_or_else(|| "Antigravity".into());
        let account = AuthAccount {
            account_id: account_id_for(scope, &identity.sub)?,
            service: scope.service,
            label,
            credential_state: AuthCredentialState::Ready,
            is_default: false,
            authenticated_at: Some(now),
            login_expires_at: None,
            github_domain: None,
            region: None,
        };
        let credential = AgyCredential {
            token: AgyToken {
                access_token: self.access_token,
                refresh_token,
                token_type,
                expiry: expiry.to_rfc3339_opts(chrono::SecondsFormat::Micros, false),
                expires_in,
            },
            auth_method: "consumer",
            id_token,
        };
        Ok(ManagedAccount { account, credential })
    }
}

pub(super) fn token_identity(id_token: &str) -> Option<Identity> {
    let mut jwt = id_token.split('.');
    jwt.next().filter(|part| !part.is_empty())?;
    let claims = jwt.next()?;
    jwt.next().filter(|part| !part.is_empty())?;
    if jwt.next().is_some() {
        return None;
    }
    let decoded = URL_SAFE_NO_PAD.decode(claims.trim_end_matches('=')).ok()?;
    let identity: Identity = serde_json::from_slice(&decoded).ok()?;
    (valid_token(&identity.sub) && identity.sub.len() <= 255).then_some(identity)
}

pub(super) fn account_id_for(
    scope: AuthAccountScope<'_>,
    subject: &str,
) -> Result<String, AuthErrorCode> {
    let identity =
        serde_json::to_vec(&(scope.tenant, scope.principal, scope.host, scope.service, subject))
            .map_err(|_| AuthErrorCode::ExecutionHostUnavailable)?;
    Ok(URL_SAFE_NO_PAD.encode(Sha256::digest(identity)))
}

fn valid_label(label: &str) -> bool {
    !label.trim().is_empty() && label.len() <= 256 && !label.chars().any(char::is_control)
}

impl ManagedAccount {
    fn save(&self, keychain: &SecKeychain) -> Result<(), AuthErrorCode> {
        let home = std::env::var_os("HOME")
            .and_then(|path| std::fs::canonicalize(path).ok())
            .ok_or(AuthErrorCode::KeychainUnavailable)?;
        let _lock = lock_keychain(&home)?;
        // One keychain record publishes the projection and its material together.
        let payload = serde_json::to_vec(self).map_err(|_| AuthErrorCode::StorageUnavailable)?;
        let id = &self.account.account_id;
        let write = match keychain.find_generic_password(MANAGED_SERVICE, id) {
            Ok((_, mut item)) => item.set_password(&payload),
            Err(error) if error.code() == KEYCHAIN_ITEM_NOT_FOUND => {
                keychain.add_generic_password(MANAGED_SERVICE, id, &payload)
            }
            Err(_) => return Err(AuthErrorCode::KeychainUnavailable),
        };
        write.map_err(|_| AuthErrorCode::KeychainUnavailable)?;
        let (saved, _) = keychain
            .find_generic_password(MANAGED_SERVICE, id)
            .map_err(|_| AuthErrorCode::KeychainUnavailable)?;
        if *saved != payload {
            return Err(AuthErrorCode::KeychainUnavailable);
        }
        Ok(())
    }
}
