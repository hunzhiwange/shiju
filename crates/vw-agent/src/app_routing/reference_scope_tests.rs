use super::reference_scope::{ApplicationReferenceScope, ReferenceScope, References};
use crate::model_relay::RelayIdentity;
use axum::http::StatusCode;
use vw_api_types::{
    agent_apps::AppId,
    id::{ProjectScope, TenantScope},
};

fn application() -> ApplicationReferenceScope {
    ApplicationReferenceScope {
        project: project("platform", "company", "project"),
        host: "host".into(),
        app: AppId::Codex,
        account: "account".into(),
        provider: "provider".into(),
        credential_generation: 1,
    }
}

fn project(platform: &str, company: &str, id: &str) -> ProjectScope {
    ProjectScope::new(TenantScope::new(platform, company).unwrap(), id.into()).unwrap()
}

fn assert_no_references(references: &References, scope: &ReferenceScope) {
    assert_eq!(references.resolve(scope, "public"), Err(StatusCode::FORBIDDEN));
    assert_eq!(references.public(scope, "upstream"), Err(StatusCode::BAD_GATEWAY));
    assert_eq!(references.check_item(scope, "item"), Err(StatusCode::FORBIDDEN));
}

#[test]
fn cc_route_003_reference_scope_expected_behavior() {
    let references = References::default();
    let app = application();
    let scope = ReferenceScope::Application(app.clone());
    references.remember(&scope, "public", "upstream").unwrap();
    references.remember_item(&scope, "item").unwrap();
    assert_eq!(references.resolve(&scope, "public").unwrap(), "upstream");
    assert_eq!(references.public(&scope, "upstream").unwrap(), "public");
    assert_eq!(references.check_item(&scope, "item"), Ok(()));

    for other in [
        ApplicationReferenceScope {
            project: project("other", "company", "project"),
            ..app.clone()
        },
        ApplicationReferenceScope {
            project: project("platform", "other", "project"),
            ..app.clone()
        },
        ApplicationReferenceScope {
            project: project("platform", "company", "other"),
            ..app.clone()
        },
        ApplicationReferenceScope { host: "other".into(), ..app.clone() },
        ApplicationReferenceScope { app: AppId::GrokBuild, ..app.clone() },
        ApplicationReferenceScope { account: "other".into(), ..app.clone() },
        ApplicationReferenceScope { provider: "other".into(), ..app.clone() },
        ApplicationReferenceScope { credential_generation: 2, ..app.clone() },
    ] {
        let other = ReferenceScope::Application(other);
        assert_no_references(&references, &other);
        references.remember(&other, "public", "other-upstream").unwrap();
        references.remember_item(&other, "item").unwrap();
        assert_eq!(references.resolve(&other, "public").unwrap(), "other-upstream");
        assert_eq!(references.public(&other, "other-upstream").unwrap(), "public");
        assert_eq!(references.check_item(&other, "item"), Ok(()));
        assert_eq!(references.resolve(&scope, "public").unwrap(), "upstream");
    }
}

#[test]
fn cc_route_003_session_and_application_references_are_isolated() {
    let references = References::default();
    let app = application();
    let session = ReferenceScope::Session(RelayIdentity {
        tenant: app.project.tenant_scope().clone(),
        project: app.project.project_id().0.clone(),
        session: app.host.clone(),
        provider: app.provider.clone(),
        model: app.account.clone(),
        generation: app.credential_generation,
    });
    let app = ReferenceScope::Application(app);
    references.remember(&session, "public", "upstream").unwrap();
    references.remember_item(&session, "item").unwrap();
    assert_no_references(&references, &app);
    references.remember(&app, "public", "other-upstream").unwrap();
    references.remember_item(&app, "app-item").unwrap();
    assert_eq!(references.resolve(&app, "public").unwrap(), "other-upstream");
    assert_eq!(references.resolve(&session, "public").unwrap(), "upstream");
    assert_eq!(references.check_item(&session, "app-item"), Err(StatusCode::FORBIDDEN));
    assert_eq!(references.check_item(&app, "item"), Err(StatusCode::FORBIDDEN));
}
