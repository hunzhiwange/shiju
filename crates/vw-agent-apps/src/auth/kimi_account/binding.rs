//! Binds an existing official Kimi provider to its verified native grant.
use super::*;
use crate::providers::live::commit::{AppState, KimiBinding};

impl KimiNativeTarget<'_> {
    pub fn bound_account(
        &self,
        scope: AuthAccountScope<'_>,
        catalog_connection: &Value,
    ) -> Result<Option<String>, AuthErrorCode> {
        self.check_scope(scope)?;
        if self.transactions.pending(AppId::Kimi).map_err(write_error)?.is_some() {
            return Err(AuthErrorCode::Conflict);
        }
        let state = self.transactions.state(AppId::Kimi).map_err(write_error)?;
        let Some(binding) = self.binding(&state, scope.tenant, scope.principal)? else {
            return Ok(None);
        };
        let config = kimi::read(self.home).map_err(write_error)?.ok_or(AuthErrorCode::NotFound)?;
        check_catalog(&config, self.provider_id, catalog_connection)?;
        let (slot, _) = select_model(&config, self.provider_id, binding.region)?;
        if slot != binding.slot
            || self.transactions.state(AppId::Kimi).map_err(write_error)? != state
            || self.transactions.pending(AppId::Kimi).map_err(write_error)?.is_some()
        {
            return Err(AuthErrorCode::Conflict);
        }
        Ok(Some(binding.account_id.clone()))
    }

    /// Executor-only. The scope, catalog connection and directory come from trusted owners.
    /// A separately signed-in CLI grant must prove the same identity. Binding
    /// neither rotates a grant nor copies it to the other credential owner.
    pub async fn bind_account(
        &self,
        credentials: &CredentialFileStore,
        account: KimiAccountTarget<'_>,
        expected_account: Option<&str>,
        catalog_connection: &Value,
        catalog_unchanged: impl Fn() -> bool + Send + 'static,
    ) -> Result<(), AuthErrorCode> {
        self.bind_account_with(
            credentials,
            account,
            expected_account,
            catalog_connection,
            catalog_unchanged,
            |_| {},
        )
        .await
    }

    pub(crate) async fn bind_account_with(
        &self,
        credentials: &CredentialFileStore,
        account: KimiAccountTarget<'_>,
        expected_account: Option<&str>,
        catalog_connection: &Value,
        catalog_unchanged: impl Fn() -> bool + Send + 'static,
        mut route: impl FnMut(&mut Request),
    ) -> Result<(), AuthErrorCode> {
        let KimiAccountTarget { scope, account_id, region, identity_sha256 } = account;
        self.check_scope(scope)?;
        crate::providers::validate_identity(account_id).map_err(|_| AuthErrorCode::InvalidInput)?;
        let home = self.home.to_owned();
        let transactions = Arc::clone(self.transactions);
        let provider_id = self.provider_id.to_owned();
        let tenant = scope.tenant.clone();
        let principal = scope.principal.to_owned();
        let account_id = account_id.to_owned();
        let lease = credentials.lease_kimi(scope, &account_id, false).map_err(lease_error)?;
        let AccountCredential::KimiOauth(saved) =
            lease.read().map_err(|_| AuthErrorCode::StorageUnavailable)?
        else {
            return Err(AuthErrorCode::StorageUnavailable);
        };
        if saved.region != region {
            return Err(AuthErrorCode::IdentityMismatch);
        }

        let config = kimi::read(&home).map_err(write_error)?.ok_or(AuthErrorCode::NotFound)?;
        check_catalog(&config, &provider_id, catalog_connection)?;
        let (slot, next) = select_model(&config, &provider_id, region)?;
        let binding = KimiBinding { provider_id, principal, account_id, region, slot };
        let slot = NativeSlot::open(&home, &binding.slot)?;
        let grant = slot.read()?;
        let expires_at = expiry(&grant)?.min(saved.expires_at.0 / 1_000);
        if expires_at <= utc_now()?.0 / 1_000 {
            return Err(AuthErrorCode::ReauthRequired);
        }
        if !shares_grant(&grant, &saved) {
            let access =
                token(&grant, "access_token").map_err(|_| AuthErrorCode::ReauthRequired)?;
            let adapter =
                KimiDeviceAuth::new().map_err(|_| AuthErrorCode::ExecutionHostUnavailable)?;
            adapter
                .verify_identity(region, &slot.device_id()?, identity_sha256, access, &mut route)
                .await?;
        }
        let expected_account = expected_account.map(str::to_owned);
        tokio::task::spawn_blocking(move || {
            let _lease = lease;
            let mut files = KimiFiles::open(&home).map_err(write_error)?;
            if transactions.recover(&mut files.guard).map_err(write_error)?.is_some() {
                return Err(AuthErrorCode::Conflict);
            }
            let state = transactions.state(AppId::Kimi).map_err(write_error)?;
            let native = KimiNativeTarget {
                home: &home,
                transactions: &transactions,
                provider_id: &binding.provider_id,
            };
            let current = native.binding(&state, &tenant, &binding.principal)?;
            let unchanged = || {
                catalog_unchanged()
                    && kimi::read(&home).is_ok_and(|latest| latest.as_ref() == Some(&config))
                    && utc_now().is_ok_and(|now| expires_at > now.0 / 1_000)
                    && slot.read().is_ok_and(|latest| latest == grant)
            };
            if current == Some(&binding) {
                // An acknowledged binding may have been followed by another model selection.
                return unchanged().then_some(()).ok_or(AuthErrorCode::Conflict);
            }
            if current.map(|b| b.account_id.as_str()) != expected_account.as_deref() {
                return Err(AuthErrorCode::Conflict);
            }
            files
                .apply_with_binding(
                    &transactions,
                    &tenant,
                    &state,
                    "config.toml",
                    Some(&config),
                    next,
                    Some(binding),
                    unchanged,
                )
                .map_err(write_error)
        })
        .await
        .map_err(|_| AuthErrorCode::ExecutionHostUnavailable)?
    }

    pub fn check_scope(&self, scope: AuthAccountScope<'_>) -> Result<(), AuthErrorCode> {
        scope.validate()?;
        if scope.service != AuthService::KimiOauth
            || std::env::var_os("KIMI_DISABLE_OAUTH_LOCK").as_deref() == Some("1".as_ref())
        {
            return Err(AuthErrorCode::Unsupported);
        }
        if &ExecutionHostId::new(scope.host).map_err(write_error)?
            != self.transactions.execution_host()
        {
            return Err(AuthErrorCode::Forbidden);
        }
        kimi::validate_identity(self.provider_id).map_err(write_error)
    }

    pub(super) fn binding<'a>(
        &self,
        state: &'a AppState,
        tenant: &vw_api_types::id::TenantScope,
        principal: &str,
    ) -> Result<Option<&'a KimiBinding>, AuthErrorCode> {
        let binding = state.kimi_bindings.get(self.provider_id);
        if binding.is_some_and(|b| {
            state.mode_tenant.as_ref() != Some(tenant)
                || b.principal != principal
                || b.provider_id != self.provider_id
        }) {
            return Err(AuthErrorCode::Forbidden);
        }
        Ok(binding)
    }
}

pub(super) fn check_catalog(
    config: &[u8],
    id: &str,
    catalog_connection: &Value,
) -> Result<(), AuthErrorCode> {
    let profiles = kimi::declarations(Some(config)).map_err(write_error)?;
    let native = profiles.get(id).ok_or(AuthErrorCode::NotFound)?;
    if &native.declaration["provider"] != catalog_connection {
        return Err(AuthErrorCode::Conflict);
    }
    Ok(())
}

pub(super) fn lease_error(error: String) -> AuthErrorCode {
    if error == "official account is in use" {
        AuthErrorCode::AccountInUse
    } else {
        AuthErrorCode::StorageUnavailable
    }
}
