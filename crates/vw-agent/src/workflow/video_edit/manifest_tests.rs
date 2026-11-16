use super::manifest::*;

pub(super) fn candidate() -> VideoEditRunManifest {
    VideoEditRunManifest {
        tenant: vw_api_transport::TenantScope::default(),
        run_id: "run".into(),
        run_key: "key".into(),
        source: VideoSourceIdentity {
            path: "/source.mp4".into(),
            bytes: 3,
            modified_ns: 1,
            file_id: "inode".into(),
            sha256: "digest".into(),
        },
        template_sha256: "template".into(),
        model_sha256: "model".into(),
        requirements_sha256: "requirements".into(),
        cloud_scope: CloudVideoScope { brand_id: Some("brand".into()), store_id: None },
        project_id: None,
        upload_id: None,
        asset_id: None,
        jobs: vec![],
        asset_jobs: vec![],
        asset_peak: 0,
        asset_cancelled: false,
        transcript: None,
        state: VideoPrepareState::New,
        revision: 0,
        lease_owner: String::new(),
        lease_until_ms: 0,
    }
}
