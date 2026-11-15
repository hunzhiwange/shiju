use app_versions::node::{node_candidate, node_range_satisfied};
use std::time::Duration;
use vw_api_app_identity::*;

fn installation() -> vw_api_app_identity::AppInstallation {
    AppInstallation {
        id: AppInstallationId("selected".into()),
        app_id: AppId::OpenClaw,
        target_host_id: "host".into(),
        environment: AppInstallationEnvironment::Macos,
        path: "/selected/bin/openclaw".into(),
        real_path: Some("/selected/lib/node_modules/openclaw/bin.js".into()),
        source: AppInstallationSource::Nvm,
        is_path_default: false,
        version: None,
        runtime_status: AppInstallationRuntimeStatus::Runnable,
    }
}

#[cfg(unix)]
#[test]
fn cc_apps_node_requirement_gap_real_sibling_reprobe() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("node");
    let mut inst = installation();
    inst.path = dir.path().join("openclaw").to_str().unwrap().into();
    for version in ["24.16.0", "22.0.0"] {
        std::fs::write(&path, format!("#!/bin/sh\nprintf 'v{version}\\n'\n")).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        let candidate = node_candidate(&inst, None, |p| std::path::Path::new(p).is_file()).unwrap();
        let probe = crate::version::probe_local_version(
            std::path::Path::new(&candidate),
            Duration::from_secs(1),
        )
        .unwrap();
        let satisfied = node_range_satisfied(">=24.16.0", probe.version.as_deref().unwrap());
        assert_eq!(satisfied, Some(version != "22.0.0"));
    }
}
