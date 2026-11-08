//! DeepSeek Harness Web and Desktop share one account grant on the execution host.

use super::{
    copilot::{read_json_limited, valid_token},
    dsh_login::{
        DshDevice, DshGrant, DshLogin, GRANT_RECORD, PLATFORM_ORIGIN, account, platform_client,
        platform_identity,
    },
    flow::{AuthAccountScope, AuthFlows},
};
use crate::providers::{
    dsh::{DshCredentials, credential_document},
    live::{
        commit::{ExecutionHostId, ProviderTransactions},
        write_guard::WriteError,
    },
};
use reqwest::{Request, StatusCode, header::HeaderValue};
use serde_yaml::{Mapping, Value as Yaml};
use sha2::{Digest, Sha256};
use std::path::Path;
use uuid::Uuid;
use vw_api_types::{
    agent_apps::AppId,
    agent_apps_auth::{
        AuthAccount, AuthCredentialState, AuthErrorCode, AuthFlow, AuthService, AuthStartRequest,
        AuthStartResponse,
    },
};

const DEVICE_RECORD: &str = "deepseek-account-platform/device";
const STATUS_PATH: &str = "/auth-api/v0/users/current";
const MAX_STATUS_BYTES: usize = 64 * 1024;

/// Access to the native slot. The executor must resolve its home and prove the
/// requesting scope owns that slot before constructing this handle.
#[derive(Clone, Copy)]
pub struct DshAccount<'a> {
    home: &'a Path,
    transactions: &'a ProviderTransactions,
    owner: AuthAccountScope<'a>,
}

impl<'a> DshAccount<'a> {
    pub fn new(
        home: &'a Path,
        transactions: &'a ProviderTransactions,
        owner: AuthAccountScope<'a>,
    ) -> Result<Self, AuthErrorCode> {
        owner.validate()?;
        if owner.service != AuthService::DshAccount {
            return Err(AuthErrorCode::Unsupported);
        }
        if &ExecutionHostId::new(owner.host).map_err(storage_error)?
            != transactions.execution_host()
        {
            return Err(AuthErrorCode::Forbidden);
        }
        Ok(Self { home, transactions, owner })
    }

    pub async fn start_login(
        &self,
        flows: &AuthFlows,
        request: AuthStartRequest,
        browser_on_execution_host: bool,
        locale: &str,
        timezone_offset_seconds: i32,
    ) -> Result<DshAccountLogin<'a>, AuthErrorCode> {
        self.start_login_with(
            flows,
            request,
            browser_on_execution_host,
            locale,
            timezone_offset_seconds,
            |_| {},
        )
        .await
    }

    pub(super) async fn start_login_with(
        &self,
        flows: &AuthFlows,
        request: AuthStartRequest,
        browser_on_execution_host: bool,
        locale: &str,
        timezone_offset_seconds: i32,
        route: impl Fn(&mut Request) + Clone + Send + Sync + 'static,
    ) -> Result<DshAccountLogin<'a>, AuthErrorCode> {
        request.validate_for(self.owner.service)?;
        if !browser_on_execution_host {
            return Err(AuthErrorCode::CallbackUnreachable);
        }
        let document = self.read_document()?;
        let grant_revision = grant_revision(&document)?;
        let previous_device = device_id(&document)?;
        let id = previous_device.unwrap_or_else(Uuid::new_v4);
        let device = DshDevice { id, locale: locale.into(), timezone_offset_seconds };
        let login = DshLogin::start_with(
            flows,
            self.owner,
            request,
            browser_on_execution_host,
            device,
            route,
        )
        .await?;
        Ok(DshAccountLogin {
            account: *self,
            login,
            grant_revision,
            previous_device,
            device_id: id,
        })
    }

    /// Reads the current native grant and checks it against the fixed platform.
    /// Only the scoped account DTO and a stable credential state are returned.
    pub async fn status(
        &self,
        locale: &str,
        timezone_offset_seconds: i32,
    ) -> Result<Option<AuthAccount>, AuthErrorCode> {
        self.status_with(locale, timezone_offset_seconds, |_| {}).await
    }

    pub(super) async fn status_with(
        &self,
        locale: &str,
        timezone_offset_seconds: i32,
        route: impl FnOnce(&mut Request),
    ) -> Result<Option<AuthAccount>, AuthErrorCode> {
        let (mut token, revision) = {
            let document = self.read_document()?;
            let Some(grant) = stored_grant(&document)? else {
                return Ok(None);
            };
            if grant.issuer.trim_end_matches('/') != PLATFORM_ORIGIN {
                return Err(AuthErrorCode::ReauthRequired);
            }
            let token =
                HeaderValue::from_str(grant.token).map_err(|_| AuthErrorCode::InvalidInput)?;
            (token, grant_revision(&document)?)
        };
        token.set_sensitive(true);
        let client = platform_client()?;
        let mut request = platform_identity(
            client.get(format!("{PLATFORM_ORIGIN}{STATUS_PATH}")),
            locale,
            timezone_offset_seconds,
        )
        .header("x-dsh-auth-token", token)
        .build()
        .map_err(|_| AuthErrorCode::UpstreamFailure)?;
        route(&mut request);
        let response = client.execute(request).await.map_err(|_| AuthErrorCode::UpstreamFailure)?;
        let state = if response.status() == StatusCode::UNAUTHORIZED {
            AuthCredentialState::ReauthRequired
        } else if response.status().is_success() {
            let body: serde_json::Value = read_json_limited(response, MAX_STATUS_BYTES).await?;
            match body["code"].as_i64() {
                Some(0) if body["data"]["biz_data"].is_object() => AuthCredentialState::Ready,
                Some(40003) => AuthCredentialState::ReauthRequired,
                _ => return Err(AuthErrorCode::UpstreamFailure),
            }
        } else {
            return Err(AuthErrorCode::UpstreamFailure);
        };
        if grant_revision(&self.read_document()?)? != revision {
            return Err(AuthErrorCode::Conflict);
        }
        account(self.owner, state, None).map(Some)
    }

    /// Removes the local login from both profiles. A valid foreign-issuer grant
    /// can be cleared locally without sending its token anywhere.
    pub fn sign_out(&self) -> Result<(), AuthErrorCode> {
        self.clear_grant(false)
    }

    /// Delete the selected shared grant without contacting the issuer.
    pub fn remove_account(
        &self,
        scope: AuthAccountScope<'_>,
        account_id: &str,
    ) -> Result<(), AuthErrorCode> {
        if self.owner != scope {
            return Err(AuthErrorCode::Forbidden);
        }
        crate::providers::validate_identity(account_id).map_err(|_| AuthErrorCode::InvalidInput)?;
        if account(self.owner, AuthCredentialState::Ready, None)?.account_id != account_id {
            return Err(AuthErrorCode::NotFound);
        }
        self.clear_grant(true)
    }

    fn clear_grant(&self, require_present: bool) -> Result<(), AuthErrorCode> {
        let Some(files) = self.credentials(false)? else {
            return if require_present { Err(AuthErrorCode::NotFound) } else { Ok(()) };
        };
        let before = files.read().map_err(storage_error)?;
        let mut document = credential_document(before.as_deref()).map_err(storage_error)?;
        if stored_grant(&document)?.is_some() {
            records_mut(&mut document).remove(Yaml::from(GRANT_RECORD));
            write_document(&files, before.as_deref(), &document)?;
        } else if require_present {
            return Err(AuthErrorCode::NotFound);
        }
        Ok(())
    }

    fn credentials(&self, create: bool) -> Result<Option<DshCredentials<'a>>, AuthErrorCode> {
        let files = DshCredentials::open(self.home, create).map_err(storage_error)?;
        if self.transactions.pending(AppId::Dsh).map_err(storage_error)?.is_some() {
            return Err(AuthErrorCode::Conflict);
        }
        Ok(files)
    }

    fn read_document(&self) -> Result<Yaml, AuthErrorCode> {
        let bytes = match self.credentials(false)? {
            Some(files) => files.read().map_err(storage_error)?,
            None => None,
        };
        credential_document(bytes.as_deref()).map_err(storage_error)
    }
}

/// A browser attempt bound to the native slot and the grant it observed.
pub struct DshAccountLogin<'a> {
    account: DshAccount<'a>,
    login: DshLogin,
    grant_revision: Option<[u8; 32]>,
    previous_device: Option<Uuid>,
    device_id: Uuid,
}

impl DshAccountLogin<'_> {
    pub fn response(&self) -> &AuthStartResponse {
        &self.login.response
    }

    pub async fn poll(&self, flows: &AuthFlows) -> Result<AuthFlow, AuthErrorCode> {
        self.poll_with(flows, |_| {}).await
    }

    pub(super) async fn poll_with(
        &self,
        flows: &AuthFlows,
        route: impl FnOnce(&mut Request),
    ) -> Result<AuthFlow, AuthErrorCode> {
        self.login
            .poll_with(flows, self.account.owner, route, |_, _, grant| self.persist(grant))
            .await
    }

    pub fn cancel(&self, flows: &AuthFlows) -> Result<(), AuthErrorCode> {
        self.login.cancel(flows, self.account.owner)
    }

    fn persist(&self, grant: &DshGrant) -> Result<(), AuthErrorCode> {
        let files = self.account.credentials(true)?.ok_or(AuthErrorCode::StorageUnavailable)?;
        let before = files.read().map_err(storage_error)?;
        let mut document = credential_document(before.as_deref()).map_err(storage_error)?;
        let current_device = device_id(&document)?;
        if grant_revision(&document)? != self.grant_revision
            || current_device != self.previous_device
        {
            return Err(AuthErrorCode::Conflict);
        }

        let records = records_mut(&mut document);
        if current_device.is_none() {
            records.insert(
                Yaml::from(DEVICE_RECORD),
                yaml_record(serde_json::json!({"id": self.device_id.to_string()}))?,
            );
        }
        records.insert(
            Yaml::from(GRANT_RECORD),
            yaml_record(serde_json::json!({
                "version": 1,
                "token": grant.token(),
                "issuer": grant.issuer(),
            }))?,
        );
        write_document(&files, before.as_deref(), &document)
    }
}

struct StoredGrant<'a> {
    token: &'a str,
    issuer: &'a str,
}

fn stored_grant(document: &Yaml) -> Result<Option<StoredGrant<'_>>, AuthErrorCode> {
    let record = &document["records"][GRANT_RECORD];
    if record.is_null() {
        return Ok(None);
    }
    let payload = &record["payload"];
    match (
        record["kind"].as_str(),
        payload["version"].as_u64(),
        payload["token"].as_str(),
        payload["issuer"].as_str(),
    ) {
        (Some("grant"), Some(1), Some(token), Some(issuer))
            if valid_token(token) && !issuer.is_empty() =>
        {
            Ok(Some(StoredGrant { token, issuer }))
        }
        _ => Err(AuthErrorCode::InvalidInput),
    }
}

fn grant_revision(document: &Yaml) -> Result<Option<[u8; 32]>, AuthErrorCode> {
    if stored_grant(document)?.is_none() {
        return Ok(None);
    }
    let bytes = serde_yaml::to_string(&document["records"][GRANT_RECORD])
        .map_err(|_| AuthErrorCode::InvalidInput)?;
    Ok(Some(Sha256::digest(bytes).into()))
}

fn device_id(document: &Yaml) -> Result<Option<Uuid>, AuthErrorCode> {
    let record = &document["records"][DEVICE_RECORD];
    if record.is_null() {
        return Ok(None);
    }
    match (record["kind"].as_str(), record["payload"]["id"].as_str()) {
        (Some("grant"), Some(id)) => {
            Uuid::parse_str(id).map(Some).map_err(|_| AuthErrorCode::InvalidInput)
        }
        _ => Err(AuthErrorCode::InvalidInput),
    }
}

fn records_mut(document: &mut Yaml) -> &mut Mapping {
    if document["records"].is_null() {
        document["records"] = Yaml::Mapping(Mapping::new());
    }
    document["records"].as_mapping_mut().expect("validated records mapping")
}

fn yaml_record(payload: serde_json::Value) -> Result<Yaml, AuthErrorCode> {
    serde_yaml::to_value(serde_json::json!({"kind": "grant", "payload": payload}))
        .map_err(|_| AuthErrorCode::StorageUnavailable)
}

fn write_document(
    files: &DshCredentials<'_>,
    before: Option<&[u8]>,
    document: &Yaml,
) -> Result<(), AuthErrorCode> {
    let next = serde_yaml::to_string(document).map_err(|_| AuthErrorCode::StorageUnavailable)?;
    files.replace(before, next.as_bytes()).map_err(storage_error)
}

fn storage_error(error: WriteError) -> AuthErrorCode {
    match error {
        WriteError::Conflict | WriteError::Recovered => AuthErrorCode::Conflict,
        WriteError::InvalidResource => AuthErrorCode::InvalidInput,
        WriteError::Io => AuthErrorCode::StorageUnavailable,
    }
}
