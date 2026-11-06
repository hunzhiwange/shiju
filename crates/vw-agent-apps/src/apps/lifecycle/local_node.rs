//! Fresh execution-host Node metadata gate; called again by preview under the write lock.
use super::WriteEnvironment;
use crate::apps::{
    discovery::version::probe_local_version,
    versions::node::{self, NodeRequirement},
};
use std::{path::Path, time::Duration};
use vw_api_types::agent_apps::AppInstallation;

pub(super) fn check(
    installation: &AppInstallation,
    write: &WriteEnvironment,
) -> Result<(), String> {
    check_at(installation, write, "https://registry.npmjs.org/")
}
pub(super) fn check_at(
    installation: &AppInstallation,
    write: &WriteEnvironment,
    registry: &str,
) -> Result<(), String> {
    // Preview and run are blocking host operations, outside async runtime workers.
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| "Node preflight runtime unavailable")?;
    let requirement = runtime.block_on(async {
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| "Node metadata transport unavailable")?;
        let requirement = node::update_node_gap_at(
            &client,
            installation,
            "latest",
            |inst| {
                let fallback = write.npm.parent()?.join("node");
                let path =
                    node::node_candidate(inst, fallback.to_str(), |p| Path::new(p).is_file())?;
                probe_local_version(Path::new(&path), Duration::from_secs(5)).ok()?.version
            },
            registry,
        )
        .await;
        Ok::<_, String>(requirement)
    })?;
    if matches!(requirement, NodeRequirement::Gap { .. }) {
        return Err("Node version does not satisfy update requirement".into());
    }
    Ok(())
}
