//! Authorized use of an existing Kimi grant without changing model selection.

use reqwest::Request;
use vw_api_types::{TimestampMs, agent_apps::AppId, agent_apps_auth::AuthErrorCode};

use super::{
    AccountCredential, CredentialFileStore, KimiAccountTarget, KimiDeviceAuth, KimiNativeTarget,
    NativeSlot, binding, kimi, oauth_slot, save_rotated_grant, shares_grant, token, write_error,
};

impl KimiDeviceAuth {
    pub(crate) async fn refresh_usage_token(
        &self,
        credentials: &CredentialFileStore,
        account: KimiAccountTarget<'_>,
        native: KimiNativeTarget<'_>,
        now: TimestampMs,
        route: &mut impl FnMut(&mut Request),
    ) -> Result<String, AuthErrorCode> {
        let KimiAccountTarget { scope, account_id, region, identity_sha256 } = account;
        native.check_scope(scope)?;
        let lease =
            credentials.lease_kimi(scope, account_id, false).map_err(binding::lease_error)?;
        let AccountCredential::KimiOauth(saved) =
            lease.read().map_err(|_| AuthErrorCode::StorageUnavailable)?
        else {
            return Err(AuthErrorCode::StorageUnavailable);
        };
        if saved.region != region {
            return Err(AuthErrorCode::IdentityMismatch);
        }
        // Another authorized caller may have finished the rotation.
        if saved.expires_at.0 > now.0 {
            return Ok(saved.access_token);
        }

        let config =
            kimi::read(native.home).map_err(write_error)?.ok_or(AuthErrorCode::NotFound)?;
        let profiles = kimi::declarations(Some(&config)).map_err(write_error)?;
        let profile = profiles.get(native.provider_id).ok_or(AuthErrorCode::NotFound)?;
        let slot_name = oauth_slot(&profile.declaration["provider"], region)?;
        let state = native.transactions.state(AppId::Kimi).map_err(write_error)?;
        if state.mode_tenant.as_ref().is_some_and(|tenant| tenant != scope.tenant) {
            return Err(AuthErrorCode::Forbidden);
        }
        let binding = native.binding(&state, scope.tenant, scope.principal)?;
        if binding.is_some_and(|binding| {
            binding.account_id != account_id
                || binding.region != region
                || binding.slot != slot_name
        }) {
            return Err(AuthErrorCode::IdentityMismatch);
        }

        let slot = NativeSlot::open(native.home, slot_name)?;
        let original = slot.read()?;
        let shared = shares_grant(&original, &saved);
        if !shared && binding.is_none() {
            return Err(AuthErrorCode::IdentityMismatch);
        }
        let check_profile = || {
            if native.transactions.pending(AppId::Kimi).map_err(write_error)?.is_some()
                || native.transactions.state(AppId::Kimi).map_err(write_error)? != state
                || kimi::read(native.home).map_err(write_error)?.as_ref() != Some(&config)
            {
                return Err(AuthErrorCode::Conflict);
            }
            Ok(())
        };
        check_profile()?;
        let current = self.resolve_native(&slot, &original, &saved, identity_sha256, route).await?;
        if shared && original != current {
            save_rotated_grant(&lease, &slot, saved, &current)?;
        }
        check_profile()?;
        if slot.read()? != current {
            return Err(AuthErrorCode::Conflict);
        }
        Ok(token(&current, "access_token")?.to_owned())
    }
}
