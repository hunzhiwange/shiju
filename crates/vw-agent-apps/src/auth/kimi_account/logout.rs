use super::{
    AccountCredential, AppId, AuthAccountScope, AuthErrorCode, AuthRegion, CredentialFileStore,
    KimiCredential, KimiNativeTarget, NativeSlot, kimi, select_model, shares_grant, write_error,
};
use crate::{auth::credential_lease::CredentialLease, providers::live::commit::AppState};
use serde_json::Value;
use toml_edit::DocumentMut;

impl KimiNativeTarget<'_> {
    /// Resolve the account behind the selected model in the trusted native profile.
    pub fn default_managed_account(
        &self,
        credentials: &CredentialFileStore,
        scope: AuthAccountScope<'_>,
        account_ids: &[String],
    ) -> Result<String, AuthErrorCode> {
        self.check_scope(scope)?;
        let state = self.logout_state(scope)?;
        let config = kimi::read(self.home).map_err(write_error)?.ok_or(AuthErrorCode::NotFound)?;
        let document = std::str::from_utf8(&config)
            .ok()
            .and_then(|text| text.parse::<DocumentMut>().ok())
            .ok_or(AuthErrorCode::InvalidInput)?;
        let declarations = kimi::declarations(Some(&config)).map_err(write_error)?;
        let profile = declarations.get(self.provider_id).ok_or(AuthErrorCode::NotFound)?;
        let alias = document
            .get("default_model")
            .and_then(toml_edit::Item::as_str)
            .ok_or(AuthErrorCode::NotFound)?;
        if profile.declaration["models"].get(alias).is_none() {
            return Err(AuthErrorCode::NotFound);
        }
        if let Some(binding) = self.binding(&state, scope.tenant, scope.principal)? {
            let (slot, _) = select_model(&config, self.provider_id, binding.region)?;
            if slot != binding.slot {
                return Err(AuthErrorCode::Conflict);
            }
            let saved = read_managed_grant(credentials, scope, &binding.account_id)?
                .ok_or(AuthErrorCode::NotFound)?;
            if saved.region != binding.region {
                return Err(AuthErrorCode::IdentityMismatch);
            }
            return Ok(binding.account_id.clone());
        }

        // An unbound CLI model can resolve a managed account only by an exact grant match.
        let mut matched = None;
        for id in account_ids {
            let Some(saved) = read_managed_grant(credentials, scope, id)? else { continue };
            let Some(slot) = self.slot_for_region(&config, saved.region)? else { continue };
            if read_grant(&slot)?.is_some_and(|grant| shares_grant(&grant, &saved)) {
                if matched.is_some() {
                    return Err(AuthErrorCode::Conflict);
                }
                matched = Some(id.clone());
            }
        }
        matched.ok_or(AuthErrorCode::NotFound)
    }

    /// Clear a shared grant only after the private account proves it owns the exact tokens.
    /// A separately signed-in CLI account has an independent credential owner.
    pub(in crate::auth) fn clear_matching_grant(
        &self,
        scope: AuthAccountScope<'_>,
        account_id: &str,
        region: AuthRegion,
        lease: &CredentialLease,
    ) -> Result<(), AuthErrorCode> {
        self.check_scope(scope)?;
        let state = self.logout_state(scope)?;
        if let Some(binding) = self.binding(&state, scope.tenant, scope.principal)? {
            if binding.account_id != account_id {
                return Ok(());
            }
            if binding.region != region {
                return Err(AuthErrorCode::IdentityMismatch);
            }
        }
        let saved = match lease.read() {
            Ok(AccountCredential::KimiOauth(saved)) if saved.region == region => saved,
            Err(error) if error == "official account not found" => return Ok(()),
            _ => return Err(AuthErrorCode::StorageUnavailable),
        };
        let Some(config) = kimi::read(self.home).map_err(write_error)? else { return Ok(()) };
        let Some(slot) = self.slot_for_region(&config, region)? else { return Ok(()) };
        let Some(grant) = read_grant(&slot)? else { return Ok(()) };
        if !shares_grant(&grant, &saved) {
            return Ok(());
        }
        let files = super::KimiFiles::open(self.home).map_err(write_error)?;
        if self.logout_state(scope)? != state
            || files
                .guard
                .read(std::ffi::OsStr::new("config.toml"))
                .map_err(write_error)?
                .as_deref()
                != Some(&config)
        {
            return Err(AuthErrorCode::Conflict);
        }
        slot.remove(&grant)
    }

    fn slot_for_region(
        &self,
        config: &[u8],
        region: AuthRegion,
    ) -> Result<Option<NativeSlot>, AuthErrorCode> {
        match select_model(config, self.provider_id, region) {
            Ok((slot, _)) => {
                tokio::runtime::Handle::try_current()
                    .map_err(|_| AuthErrorCode::ExecutionHostUnavailable)?;
                NativeSlot::open(self.home, &slot).map(Some)
            }
            Err(AuthErrorCode::IdentityMismatch | AuthErrorCode::NotFound) => Ok(None),
            Err(error) => Err(error),
        }
    }

    fn logout_state(&self, scope: AuthAccountScope<'_>) -> Result<AppState, AuthErrorCode> {
        if self.transactions.pending(AppId::Kimi).map_err(write_error)?.is_some() {
            return Err(AuthErrorCode::Conflict);
        }
        let state = self.transactions.state(AppId::Kimi).map_err(write_error)?;
        if state.mode_tenant.as_ref().is_some_and(|tenant| tenant != scope.tenant) {
            return Err(AuthErrorCode::Forbidden);
        }
        Ok(state)
    }
}

pub(super) fn read_managed_grant(
    credentials: &CredentialFileStore,
    scope: AuthAccountScope<'_>,
    account_id: &str,
) -> Result<Option<KimiCredential>, AuthErrorCode> {
    let lease = match credentials.lease_kimi(scope, account_id, false) {
        Ok(lease) => lease,
        Err(error) if error == "official account not found" => return Ok(None),
        Err(error) => return Err(super::binding::lease_error(error)),
    };
    match lease.read() {
        Ok(AccountCredential::KimiOauth(saved)) => Ok(Some(saved)),
        Err(error) if error == "official account not found" => Ok(None),
        _ => Err(AuthErrorCode::StorageUnavailable),
    }
}

pub(super) fn read_grant(slot: &NativeSlot) -> Result<Option<Value>, AuthErrorCode> {
    match slot.read() {
        Ok(grant) => Ok(Some(grant)),
        Err(AuthErrorCode::ReauthRequired) => Ok(None),
        Err(error) => Err(error),
    }
}
