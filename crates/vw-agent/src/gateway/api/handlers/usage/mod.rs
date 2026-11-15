//! Composition of usage routes with the gateway's authorization and scan host.
use super::{agent_apps, external_sessions::usage_scan};
use gateway_usage::host::{File, Host, Source};

pub(crate) fn router() -> axum::Router {
    gateway_usage::router(Host {
        database: crate::usage_bridge::context::database,
        authorize_read: agent_apps::authorize_usage_ledger_read,
        authorize_events: agent_apps::authorize_usage_events,
        authorize_maintenance: |policy, headers, scope, host| {
            let (principal, home, instances, worktree) =
                agent_apps::authorize_external_history_maintenance(policy, headers, scope, host)?;
            Ok((principal, home, instances.into_iter().map(|i| Source {
                instance_id: i.instance_id, provider: i.provider, root: i.root,
            }).collect(), worktree))
        },
        authorized_owner: |policy, tenant, headers, intent| {
            usage_scan::authorized_owner(policy, tenant, headers, &usage_scan::Intent {
                project_id: intent.project_id.clone(), target_host: intent.target_host.clone(),
                active: intent.active, paused: intent.paused,
            })
        },
        apps: || usage_scan::ADAPTERS.iter().map(|(_, app, _)| *app).collect(),
        schedules: usage_scan::schedules,
        discover: |scope, host, provider, instances, worktree| {
            let instances = instances.iter().map(|(home, i)| (home.clone(),
                agent_apps::ExternalHistoryInstance {
                    instance_id: i.instance_id.clone(), provider: i.provider,
                    root: i.root.clone(), resume: None,
                })).collect::<Vec<_>>();
            // Discovery consumes source identity/root only; resume configuration
            // belongs to the separate native-session launch path.
            Ok(usage_scan::discover(scope, host, provider, &instances, worktree)?
                .into_iter().map(|f| File { path: f.path, identity: f.identity }).collect())
        },
    })
}
