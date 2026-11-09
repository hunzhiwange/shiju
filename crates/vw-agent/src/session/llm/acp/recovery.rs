//! Recover an ACP session whose selection lost its record ID.

use vw_acp::{SessionIndexEntry, SessionRecord};

use crate::{config::schema::AcpAgentConfig, model_relay::RelayIdentity};

use super::{build_acp_command_line, model_source::profile::launch_reference};

pub(super) enum Launch<'a> {
    Managed(RelayIdentity),
    Local { agent: &'a str, config: &'a AcpAgentConfig },
}

pub(super) struct RecoveredSession {
    pub generation: u64,
    pub record: SessionRecord,
}

pub(super) async fn recover_session(
    session: &str,
    identity: &str,
    cwd: &str,
    generation: u64,
    launch: &Launch<'_>,
) -> anyhow::Result<Option<RecoveredSession>> {
    let session_dir = vw_acp::default_session_base_dir()
        .ok_or(vw_acp::SessionRepositoryError::HomeDirUnavailable)?;
    let index = match vw_acp::load_or_rebuild_session_index(session_dir).await {
        Ok(index) => index,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let Some((generation, entry)) =
        recover_from_index(&index.entries, session, identity, cwd, generation, launch)?
    else {
        return Ok(None);
    };

    let record = vw_acp::resolve_session_record(&entry.vwacp_record_id).await?;
    anyhow::ensure!(
        record.vwacp_record_id == entry.vwacp_record_id
            && record.acp_session_id == entry.acp_session_id
            && record.name == entry.name
            && record.cwd == cwd
            && record.agent_command == entry.agent_command
            && record.closed != Some(true),
        "ACP session record changed during recovery; retry the original session"
    );
    if let Launch::Local { agent, config } = launch {
        // Compare the saved launch without reusing its environment. The scanner's
        // injected skill view can be different on every request.
        let mut configured = (*config).clone();
        super::skill_mounts::configure_native_scanner(agent, &mut configured, None)
            .map_err(|error| anyhow::anyhow!(error.to_string()))?;
        anyhow::ensure!(
            record.agent_config.as_ref().is_some_and(|saved| {
                saved.command == config.command
                    && saved.args == config.args
                    && config.env.iter().all(|(key, value)| saved.env.get(key) == Some(value))
                    && saved.env.keys().all(|key| configured.env.contains_key(key))
            }),
            "ACP session recovery requires its original local launch configuration"
        );
    }
    Ok(Some(RecoveredSession { generation, record }))
}

fn recover_from_index<'a>(
    entries: &'a [SessionIndexEntry],
    session: &str,
    identity: &str,
    cwd: &str,
    generation: u64,
    launch: &Launch<'_>,
) -> anyhow::Result<Option<(u64, &'a SessionIndexEntry)>> {
    let prefix = format!("{session}:acp-source-");
    let suffix = format!(":{identity}");
    let local_command = match launch {
        Launch::Local { config, .. } => Some(build_acp_command_line(config)),
        Launch::Managed(_) => None,
    };
    let mut scoped_history = false;
    let mut matches = Vec::new();
    for record in entries {
        if record.cwd != cwd || record.closed {
            continue;
        }
        let Some(scoped) = record.name.as_deref().and_then(|name| name.strip_prefix("vw-session:"))
        else {
            continue;
        };
        let Some(number) = scoped.strip_prefix(&prefix).and_then(|name| name.strip_suffix(&suffix))
        else {
            continue;
        };
        let Ok(candidate) = number.parse::<u64>() else { continue };
        if number != candidate.to_string() {
            continue;
        }
        scoped_history = true;
        if candidate > generation
            || record.acp_session_id.trim().is_empty()
            || record.vwacp_record_id.trim().is_empty()
        {
            continue;
        }
        let same_launch = match launch {
            Launch::Managed(identity) => {
                let mut identity = identity.clone();
                identity.session.insert_str(0, scoped);
                record.agent_command == launch_reference(&identity)?
            }
            Launch::Local { .. } => local_command.as_deref() == Some(record.agent_command.as_str()),
        };
        if same_launch {
            matches.push((candidate, record));
        }
    }

    matches.sort_unstable_by_key(|(generation, _)| std::cmp::Reverse(*generation));
    let Some(&(latest, record)) = matches.first() else {
        anyhow::ensure!(
            !scoped_history,
            "ACP session history exists but its original launch could not be verified"
        );
        return Ok(None);
    };
    anyhow::ensure!(
        matches.get(1).is_none_or(|(other, _)| *other != latest),
        "Multiple ACP records match the original session"
    );
    Ok(Some((latest, record)))
}

#[cfg(test)]
#[path = "recovery_tests.rs"]
mod tests;
