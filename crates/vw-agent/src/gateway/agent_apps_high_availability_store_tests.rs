use std::{sync::Barrier, thread};

use anyhow::Result;
use vw_agent_apps::providers::failover::{ProviderAuth, ProviderSnapshot};
use vw_api_types::{
    agent_apps::AppId::{Claude, Codex},
    agent_apps_high_availability::{FailoverQueueScope, FailoverQueueVersion},
    app_providers::AppProviderSummary,
    id::{ProjectScope, TenantScope},
};

use super::{
    agent_apps_high_availability_store::{
        FailoverQueueState, FailoverQueueStore, FailoverQueueVersionConflict, NAMESPACE, config_key,
    },
    settings_store::GatewaySettingsStore,
};
use crate::global_database::GlobalDatabase;

#[test]
fn ha_sqlite_queue_updates_are_atomic() -> Result<()> {
    let home = tempfile::tempdir()?;
    let database = GlobalDatabase::for_home(home.path())?;
    let store = FailoverQueueStore::new(database.clone())?;
    let settings = GatewaySettingsStore::new(database.clone())?;
    let tenant = TenantScope::new("platform", "company")?;
    let scope = FailoverQueueScope {
        project: ProjectScope::new(tenant.clone(), "project".into())?,
        target_host_id: "host-a".into(),
        app_id: Claude,
    };
    let key = config_key(&scope)?;
    let read_raw = || settings.get(&tenant, NAMESPACE, &key);
    let empty = store.get(&scope)?;
    assert_eq!(empty, FailoverQueueState::default());
    assert_eq!(read_raw()?, None);

    let add = |id: &str, version| {
        let candidate = ProviderSnapshot {
            provider: AppProviderSummary {
                id: id.into(),
                app_id: Claude,
                name: "Fixture".into(),
                configured: true,
            },
            auth: ProviderAuth::ThirdParty,
        };
        store.update(&scope, version, |queue, _| {
            queue.add(id, &[candidate]).map_err(|error| anyhow::anyhow!("{error:?}"))?;
            Ok(())
        })
    };

    let start = Barrier::new(2);
    let initial_version = empty.version;
    let [first, second] = thread::scope(|threads| {
        ["provider-a", "provider-b"]
            .map(|id| {
                let (start, add) = (&start, &add);
                threads.spawn(move || {
                    start.wait();
                    (id, add(id, initial_version))
                })
            })
            .map(|thread| thread.join().expect("queue writer panicked"))
    });
    let ((winner_id, winner), (loser_id, conflict)) = match (first, second) {
        ((id, Ok(saved)), (other, Err(error))) | ((other, Err(error)), (id, Ok(saved))) => {
            ((id, saved), (other, error))
        }
        (first, second) => {
            panic!("one empty-queue writer must hit a version conflict: {first:?}, {second:?}")
        }
    };
    assert!(conflict.is::<FailoverQueueVersionConflict>(), "{conflict:#}");
    assert_eq!(winner.version, FailoverQueueVersion(1));
    assert_eq!(winner.members, [winner_id]);

    let saved = add(loser_id, winner.version)?;
    assert_eq!(saved.version, FailoverQueueVersion(2));
    assert_eq!(saved.members, [winner_id, loser_id]);

    let before = read_raw()?.expect("queue persisted");
    let connection = database.connect_sqlite()?;
    // FAIL in an AFTER trigger leaves the statement's row change in place unless
    // the enclosing transaction is rolled back.
    connection.execute_batch(
        "CREATE TRIGGER reject_queue_write AFTER UPDATE ON gateway_settings
         WHEN NEW.namespace = 'agent_apps.high_availability'
         BEGIN SELECT RAISE(FAIL, 'fixture write rejected'); END;",
    )?;
    let error = store
        .update(&scope, saved.version, |queue, enabled| {
            assert!(queue.remove(winner_id).unwrap().is_some());
            *enabled = true;
            Ok(())
        })
        .expect_err("trigger must reject the update");
    assert!(format!("{error:#}").contains("fixture write rejected"));
    assert_eq!(read_raw()?.as_deref(), Some(before.as_str()));
    assert_eq!(store.get(&scope)?, saved);
    connection.execute_batch("DROP TRIGGER reject_queue_write")?;
    assert_eq!(add(loser_id, saved.version)?, saved);

    for other in [
        FailoverQueueScope {
            project: ProjectScope::new(
                TenantScope::new("platform", "another-company")?,
                "project".into(),
            )?,
            ..scope.clone()
        },
        FailoverQueueScope {
            project: ProjectScope::new(tenant, "another-project".into())?,
            ..scope.clone()
        },
        FailoverQueueScope { target_host_id: "host-b".into(), ..scope.clone() },
        FailoverQueueScope { app_id: Codex, ..scope.clone() },
    ] {
        assert_eq!(store.get(&other)?, empty);
    }
    assert_eq!(FailoverQueueStore::new(database)?.get(&scope)?, saved);
    Ok(())
}
