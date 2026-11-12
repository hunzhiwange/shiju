use super::*;
use vw_api_types::{
    ProjectScope, TenantScope,
    agent_apps::AppId,
    agent_apps_diagnostics::{DiagnosticEndpointSource, DiagnosticScope},
};
fn owner() -> DiagnosticJobOwner {
    DiagnosticJobOwner {
        principal: "member".into(),
        scope: DiagnosticScope {
            project: ProjectScope::new(TenantScope::default(), "project".into()).unwrap(),
            target_host_id: "host".into(),
            app_id: AppId::Claude,
        },
    }
}
#[test]
fn diagnostic_endpoint_reference_is_scoped_bounded_and_expires() {
    let root = tempfile::tempdir().unwrap();
    let jobs = DiagnosticJobs::open(root.path()).unwrap();
    let owner = owner();
    let provider: ProviderId = "provider".into();
    let row = DiagnosticEndpointCandidate {
        id: DiagnosticEndpointId("opaque".into()),
        display_url: "https://example.test".into(),
        sources: vec![DiagnosticEndpointSource::Draft],
    };
    jobs.remember_endpoint_candidates(owner.clone(), provider.clone(), vec![row.clone()]).unwrap();
    let ids = [row.id.clone()];
    assert_eq!(
        jobs.resolve_endpoint_candidates(&owner, &provider, &ids).unwrap()[0].1.url,
        row.display_url
    );
    for dimension in 0..5 {
        let mut foreign = owner.clone();
        match dimension {
            0 => foreign.principal = "other".into(),
            1 => foreign.scope.target_host_id = "other".into(),
            2 => foreign.scope.app_id = AppId::Codex,
            3 => {
                foreign.scope.project =
                    ProjectScope::new(TenantScope::default(), "other".into()).unwrap()
            }
            _ => {
                foreign.scope.project =
                    ProjectScope::new(TenantScope::new("other", "other").unwrap(), "project".into())
                        .unwrap()
            }
        }
        assert!(jobs.resolve_endpoint_candidates(&foreign, &provider, &ids).is_none());
    }
    assert!(jobs.resolve_endpoint_candidates(&owner, &"other".into(), &ids).is_none());
    assert!(
        jobs.resolve_endpoint_candidates(&owner, &provider, &[row.id.clone(), row.id.clone()])
            .is_none()
    );
    assert_eq!(
        jobs.remember_endpoint_candidates(
            owner.clone(),
            provider.clone(),
            vec![row.clone(); MAX_PROBES_PER_JOB + 1]
        ),
        Err(DiagnosticJobError::InvalidInput)
    );
    jobs.0.endpoints.lock().0[0].expires = Instant::now() - std::time::Duration::from_secs(1);
    assert!(jobs.resolve_endpoint_candidates(&owner, &provider, &ids).is_none());
    assert!(jobs.0.endpoints.lock().0.is_empty());
    for n in 0..MAX_JOBS {
        jobs.remember_endpoint_candidates(
            owner.clone(),
            format!("provider-{n}").into(),
            vec![row.clone()],
        )
        .unwrap();
    }
    assert_eq!(
        jobs.remember_endpoint_candidates(owner, provider, vec![row]),
        Err(DiagnosticJobError::CapacityExceeded)
    );
}
