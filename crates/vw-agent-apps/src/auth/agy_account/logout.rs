use super::{
    AgyNativeTarget, KeychainEntry, credential_account_id, decode, keychain_error, locked_keychain,
    macos::{managed_record, same_grant},
    native_home, running, save_managed, validate_account_id,
};
use crate::auth::{
    agy_login::{KEYCHAIN_ITEM_NOT_FOUND, MANAGED_SERVICE, account_id_for},
    flow::{AuthAccountScope, AuthFlows},
    logout::LogoutSelection,
};
use security_framework::{
    item::{ItemClass, ItemSearchOptions, Limit},
    os::macos::keychain::SecKeychain,
};
use vw_api_types::agent_apps_auth::{AuthCredentialState, AuthErrorCode};

/// Invalidate the owned AGY records and any matching native login.
pub fn logout_accounts(
    flows: &AuthFlows,
    scope: AuthAccountScope<'_>,
    selection: LogoutSelection<'_>,
    native: AgyNativeTarget<'_>,
) -> Result<(), AuthErrorCode> {
    let entry = KeychainEntry::new("gemini", "antigravity").map_err(keychain_error)?;
    logout_accounts_in(flows, scope, selection, native, &entry)
}

pub(in crate::auth) fn logout_accounts_in(
    flows: &AuthFlows,
    scope: AuthAccountScope<'_>,
    selection: LogoutSelection<'_>,
    native: AgyNativeTarget<'_>,
    entry: &KeychainEntry<'_>,
) -> Result<(), AuthErrorCode> {
    let home = native_home(scope, native)?;
    let selected = {
        let (_lock, keychain) = locked_keychain(&home)?;
        match selection {
            LogoutSelection::Account(id) => {
                validate_account_id(id)?;
                managed_record(&keychain, scope, id)?;
                Some(id.to_owned())
            }
            LogoutSelection::Default => {
                let raw = entry.read().map_err(keychain_error)?.ok_or(AuthErrorCode::NotFound)?;
                let live = decode(&raw).ok_or(AuthErrorCode::Conflict)?;
                let id = credential_account_id(scope, &live).ok_or(AuthErrorCode::Conflict)?;
                let (record, _, _) = managed_record(&keychain, scope, &id)?;
                if !same_grant(&live, &record.credential) {
                    return Err(AuthErrorCode::Conflict);
                }
                Some(id)
            }
            LogoutSelection::All => None,
        }
    };
    // Flow completion can acquire the same keychain lock.
    flows.cancel_for_logout(scope, selected.as_deref())?;
    let (_lock, keychain) = locked_keychain(&home)?;
    let ids = match selected {
        Some(id) => vec![id],
        None => scoped_account_ids(&keychain, scope)?,
    };
    let current = entry.read().map_err(keychain_error)?;
    let live =
        current.as_deref().map(|bytes| decode(bytes).ok_or(AuthErrorCode::Conflict)).transpose()?;
    let live_id = live
        .as_ref()
        .map(|value| credential_account_id(scope, value).ok_or(AuthErrorCode::Conflict))
        .transpose()?;
    if matches!(selection, LogoutSelection::Default) && live_id.as_ref() != ids.first() {
        return Err(AuthErrorCode::Conflict);
    }
    for id in ids {
        let (mut record, item, _) = managed_record(&keychain, scope, &id)?;
        if live_id.as_deref() == Some(&id) {
            if !same_grant(live.as_ref().expect("live account exists"), &record.credential) {
                return Err(AuthErrorCode::Conflict);
            }
            if running()? {
                return Err(AuthErrorCode::AccountInUse);
            }
            entry
                .delete_matching(current.as_deref().expect("live account exists"))
                .map_err(keychain_error)?;
        }
        if record.account.credential_state != AuthCredentialState::SignedOut {
            record.account.credential_state = AuthCredentialState::SignedOut;
            record.account.is_default = false;
            record.account.login_expires_at = None;
            record.credential = serde_json::Value::Null;
            record.signed_out_owner = Some(signed_out_owner(scope)?);
            save_managed(&keychain, record, item)?;
        }
    }
    Ok(())
}

pub(super) fn signed_out_owner(scope: AuthAccountScope<'_>) -> Result<String, AuthErrorCode> {
    account_id_for(scope, "signed-out-owner")
}

fn scoped_account_ids(
    keychain: &SecKeychain,
    scope: AuthAccountScope<'_>,
) -> Result<Vec<String>, AuthErrorCode> {
    let found = ItemSearchOptions::new()
        .keychains(std::slice::from_ref(keychain))
        .class(ItemClass::generic_password())
        .service(MANAGED_SERVICE)
        .load_attributes(true)
        .limit(Limit::All)
        .search();
    let entries = match found {
        Ok(entries) => entries,
        Err(error) if error.code() == KEYCHAIN_ITEM_NOT_FOUND => return Ok(Vec::new()),
        Err(_) => return Err(AuthErrorCode::KeychainUnavailable),
    };
    let mut ids = Vec::new();
    for entry in entries {
        let mut attributes = entry.simplify_dict().ok_or(AuthErrorCode::KeychainUnavailable)?;
        let id = attributes.remove("acct").ok_or(AuthErrorCode::KeychainUnavailable)?;
        validate_account_id(&id)?;
        match managed_record(keychain, scope, &id) {
            Ok(_) => ids.push(id),
            Err(AuthErrorCode::NotFound) => {}
            Err(error) => return Err(error),
        }
    }
    Ok(ids)
}
