use super::binding::{
    UnboundUsageAccount, UsageAccount, UsageAccountSelection, resolve_usage_credential,
};
use crate::auth::flow::AuthAccountScope;
use vw_api_types::{
    agent_apps::AppId,
    agent_apps_auth::{AuthErrorCode, AuthService},
    id::TenantScope,
};

#[test]
fn usage_binding_returns_expected_result() {
    let tenant = TenantScope::default();
    let copilot = AuthAccountScope {
        tenant: &tenant,
        principal: "owner",
        host: "mac",
        service: AuthService::GithubCopilot,
    };
    let selection = UsageAccountSelection {
        app: AppId::ClaudeDesktop,
        account_id: None,
        bound_account_id: Some("bound"),
        unbound: UnboundUsageAccount::ManagedDefault,
    };
    let bound = resolve_usage_credential(
        copilot,
        copilot,
        selection,
        |_| panic!("an explicit binding must not consult the default"),
        |scope, id| {
            assert!(scope == copilot);
            assert_eq!(id, "bound");
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(bound.account(), &UsageAccount::Managed("bound".into()));
    assert!(bound.scope() == copilot);

    let default = resolve_usage_credential(
        copilot,
        copilot,
        UsageAccountSelection { bound_account_id: None, ..selection },
        |scope| {
            assert!(scope == copilot);
            Ok(Some("default-account".into()))
        },
        |scope, id| {
            assert!(scope == copilot);
            assert_eq!(id, "default-account");
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(default.account(), &UsageAccount::Managed("default-account".into()));

    let codex = AuthAccountScope { service: AuthService::CodexOauth, ..copilot };
    let native = resolve_usage_credential(
        codex,
        codex,
        UsageAccountSelection {
            app: AppId::Codex,
            bound_account_id: None,
            unbound: UnboundUsageAccount::NativeLogin { is_owner: true },
            ..selection
        },
        |_| Ok(Some("managed-codex-default".into())),
        |_, _| panic!("an unbound Codex official card must not use a managed account"),
    )
    .unwrap();
    assert_eq!(native.account(), &UsageAccount::NativeLogin);
    assert!(native.scope() == codex);

    let agy = AuthAccountScope { service: AuthService::AgyOauth, ..copilot };
    let bound = resolve_usage_credential(
        agy,
        agy,
        UsageAccountSelection {
            app: AppId::Agy,
            bound_account_id: Some("agy-bound"),
            unbound: UnboundUsageAccount::ExplicitOnly,
            ..selection
        },
        |_| panic!("an AGY binding must not consult the default"),
        |scope, id| {
            assert!(scope == agy);
            assert_eq!(id, "agy-bound");
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(bound.account(), &UsageAccount::Managed("agy-bound".into()));
}

#[test]
fn usage_binding_rejects_invalid_input() {
    use AuthErrorCode::{Conflict, Forbidden, InvalidInput, NotFound, ReauthRequired};

    let tenant = TenantScope::default();
    let other_tenant = TenantScope::new("other-platform", "other-company").unwrap();
    let scope = AuthAccountScope {
        tenant: &tenant,
        principal: "owner",
        host: "mac",
        service: AuthService::CodexOauth,
    };
    let bound = UsageAccountSelection {
        app: AppId::Codex,
        account_id: None,
        bound_account_id: Some("revoked"),
        unbound: UnboundUsageAccount::NativeLogin { is_owner: true },
    };
    let no_default = |_| panic!("a rejected selection must not consult the default");
    let no_account = |_: AuthAccountScope<'_>, _: &str| {
        panic!("a rejected selection must not look up a managed account")
    };
    for host_scope in [
        AuthAccountScope { tenant: &other_tenant, ..scope },
        AuthAccountScope { principal: "other-owner", ..scope },
        AuthAccountScope { host: "other-mac", ..scope },
        AuthAccountScope { service: AuthService::AgyOauth, ..scope },
    ] {
        let result = resolve_usage_credential(scope, host_scope, bound, no_default, no_account);
        assert_eq!(result.err(), Some(Forbidden));
    }
    let result = resolve_usage_credential(scope, scope, bound, no_default, |_, id| {
        assert_eq!(id, "revoked");
        Err(ReauthRequired)
    });
    assert_eq!(result.err(), Some(ReauthRequired));

    for (selection, error) in [
        (UsageAccountSelection { account_id: Some("other"), ..bound }, Conflict),
        (UsageAccountSelection { bound_account_id: Some("../other"), ..bound }, InvalidInput),
        (
            UsageAccountSelection {
                bound_account_id: None,
                unbound: UnboundUsageAccount::ManagedDefault,
                ..bound
            },
            NotFound,
        ),
        (
            UsageAccountSelection {
                bound_account_id: None,
                unbound: UnboundUsageAccount::NativeLogin { is_owner: false },
                ..bound
            },
            Forbidden,
        ),
    ] {
        let result = resolve_usage_credential(scope, scope, selection, no_default, no_account);
        assert_eq!(result.err(), Some(error));
    }

    let agy = AuthAccountScope { service: AuthService::AgyOauth, ..scope };
    for unbound in
        [UnboundUsageAccount::ManagedDefault, UnboundUsageAccount::NativeLogin { is_owner: true }]
    {
        let result = resolve_usage_credential(
            agy,
            agy,
            UsageAccountSelection {
                app: AppId::Agy,
                bound_account_id: None,
                account_id: Some("unbound-account"),
                unbound,
            },
            no_default,
            no_account,
        );
        assert_eq!(result.err(), Some(NotFound));
    }
}

#[cfg(unix)]
#[test]
fn usage_binding_rejects_credentials_outside_the_selected_scope() {
    use crate::auth::credential_file::{AccountCredential, CredentialFileStore};
    use std::os::unix::fs::PermissionsExt;

    let root = tempfile::Builder::new()
        .permissions(std::fs::Permissions::from_mode(0o700))
        .tempdir()
        .unwrap();
    let store = CredentialFileStore::new(root.path()).unwrap();
    let tenant = TenantScope::new("platform", "company").unwrap();
    let foreign_tenant = TenantScope::new("platform", "other-company").unwrap();
    let scope = AuthAccountScope {
        tenant: &tenant,
        principal: "owner",
        host: "mac",
        service: AuthService::GithubCopilot,
    };
    for (owner, id) in
        [(scope, "owned"), (AuthAccountScope { tenant: &foreign_tenant, ..scope }, "foreign")]
    {
        store
            .lease_copilot(owner, id, true)
            .unwrap()
            .save(&AccountCredential::CopilotToken("synthetic-token".into()))
            .unwrap();
    }
    let selection = UsageAccountSelection {
        app: AppId::ClaudeDesktop,
        account_id: Some("owned"),
        bound_account_id: None,
        unbound: UnboundUsageAccount::ExplicitOnly,
    };
    let verify = |authorized, id: &str| {
        store.read_managed_account(authorized, id)?.ok_or(AuthErrorCode::NotFound)?;
        Ok(())
    };
    let found = resolve_usage_credential(scope, scope, selection, |_| Ok(None), verify).unwrap();
    assert_eq!(found.account(), &UsageAccount::Managed("owned".into()));
    let denied = resolve_usage_credential(
        scope,
        scope,
        UsageAccountSelection { account_id: Some("foreign"), ..selection },
        |_| Ok(None),
        verify,
    );
    assert_eq!(denied.err(), Some(AuthErrorCode::NotFound));
}
