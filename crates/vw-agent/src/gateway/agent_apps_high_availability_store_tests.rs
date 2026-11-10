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
use crate::global_database::{GlobalDatabase, test_support::PostgresTestDatabase};

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

#[test]
fn ha_reorder_rejects_stale_queue_version() -> Result<()> {
    let home = tempfile::tempdir()?;
    let store = FailoverQueueStore::new(GlobalDatabase::for_home(home.path())?)?;
    let scope = FailoverQueueScope {
        project: ProjectScope::new(TenantScope::new("platform", "company")?, "project".into())?,
        target_host_id: "host-a".into(),
        app_id: Claude,
    };
    let original = ["provider-z", "provider-a"];
    let providers = original.map(|id| ProviderSnapshot {
        provider: AppProviderSummary {
            id: id.into(),
            app_id: Claude,
            name: id.into(),
            configured: true,
        },
        auth: ProviderAuth::ThirdParty,
    });
    let initial = store.update(&scope, FailoverQueueVersion(0), |queue, enabled| {
        for id in original {
            queue.add(id, &providers).map_err(|error| anyhow::anyhow!("{error:?}"))?;
        }
        *enabled = true;
        Ok(())
    })?;
    let reorder = |version, ids: &[&str]| {
        store.update(&scope, version, |queue, _| {
            queue
                .reorder(ids.iter().copied().map(str::to_owned).collect())
                .map_err(|error| anyhow::anyhow!("{error:?}"))
        })
    };
    let desired = ["provider-a", "provider-z"];
    let saved = reorder(initial.version, &desired)?;
    assert_eq!(saved.version, FailoverQueueVersion(2));
    assert_eq!(saved.members, desired);
    assert!(saved.enabled);

    let conflict = reorder(initial.version, &original).expect_err("old version must conflict");
    assert!(conflict.is::<FailoverQueueVersionConflict>(), "{conflict:#}");
    assert!(reorder(saved.version, &["provider-z"]).is_err());
    assert_eq!(store.get(&scope)?, saved);
    assert_eq!(reorder(saved.version, &desired)?, saved);
    Ok(())
}

#[test]
fn ha_postgres_queue_updates_are_atomic() -> Result<()> {
    let fixture = PostgresTestDatabase::from_environment()?;
    let database = fixture.database();
    let settings = GatewaySettingsStore::new(database.clone())?;
    let store = FailoverQueueStore::new(database.clone())
        .expect("the failover queue must accept the configured PostgreSQL provider");
    let tenant = TenantScope::new("platform", "company")?;
    let other_tenant = TenantScope::new("platform", "other-company")?;
    let scope = FailoverQueueScope {
        project: ProjectScope::new(tenant.clone(), "project".into())?,
        target_host_id: "host-a".into(),
        app_id: Claude,
    };
    let other_scope = FailoverQueueScope {
        project: ProjectScope::new(other_tenant.clone(), "project".into())?,
        ..scope.clone()
    };
    let key = config_key(&scope)?;
    assert_eq!(config_key(&other_scope)?, key);
    let read_raw = || settings.get(&tenant, NAMESPACE, &key);
    let empty = FailoverQueueState::default();
    assert_eq!(store.get(&scope)?, empty);
    assert_eq!(store.update(&scope, empty.version, |_, _| Ok(()))?, empty);
    assert_eq!(read_raw()?, None);
    let conflict = store
        .update(&scope, FailoverQueueVersion(1), |_, _| panic!("stale change ran"))
        .expect_err("a stale first write must conflict");
    assert!(conflict.is::<FailoverQueueVersionConflict>(), "{conflict:#}");
    assert_eq!(read_raw()?, None);

    let add = |scope: &FailoverQueueScope, id: &str, version| {
        let candidate = ProviderSnapshot {
            provider: AppProviderSummary {
                id: id.into(),
                app_id: Claude,
                name: "Fixture".into(),
                configured: true,
            },
            auth: ProviderAuth::ThirdParty,
        };
        store.update(scope, version, |queue, _| {
            queue.add(id, &[candidate]).map_err(|error| anyhow::anyhow!("{error:?}"))?;
            Ok(())
        })
    };
    let other = add(&other_scope, "provider-other", empty.version)?;
    assert_eq!(other.members, ["provider-other"]);
    let other_raw = settings.get(&other_tenant, NAMESPACE, &key)?;

    let (winner, added_a) = race_postgres_queue_updates(
        || add(&scope, "provider-a", empty.version),
        || add(&scope, "provider-b", empty.version),
    );
    let winner_id = if added_a { "provider-a" } else { "provider-b" };
    assert_eq!(winner.members, [winner_id]);
    assert_eq!(winner.version, FailoverQueueVersion(1));
    assert_eq!(store.get(&scope)?, winner);

    let remove_winner = |version| {
        store.update(&scope, version, |queue, _| {
            assert!(queue.remove(winner_id).unwrap().is_some());
            Ok(())
        })
    };
    let (updated, added_c) = race_postgres_queue_updates(
        || add(&scope, "provider-c", winner.version),
        || remove_winner(winner.version),
    );
    assert_eq!(updated.version, FailoverQueueVersion(2));
    assert_eq!(store.get(&scope)?, updated);
    let saved = if added_c {
        assert_eq!(updated.members, [winner_id, "provider-c"]);
        remove_winner(updated.version)?
    } else {
        assert!(updated.members.is_empty());
        add(&scope, "provider-c", updated.version)?
    };
    assert_eq!(saved.version, FailoverQueueVersion(3));
    assert_eq!(saved.members, ["provider-c"]);

    let reopened = FailoverQueueStore::new(fixture.reopen()?)?;
    assert_eq!(reopened.get(&scope)?, saved);
    assert_eq!(reopened.get(&other_scope)?, other);
    assert_eq!(settings.get(&other_tenant, NAMESPACE, &key)?, other_raw);
    fixture.cleanup()
}

fn race_postgres_queue_updates(
    first: impl FnOnce() -> Result<FailoverQueueState> + Send,
    second: impl FnOnce() -> Result<FailoverQueueState> + Send,
) -> (FailoverQueueState, bool) {
    let start = Barrier::new(2);
    let (first, second) = thread::scope(|threads| {
        let first = threads.spawn(|| {
            start.wait();
            first()
        });
        let second = threads.spawn(|| {
            start.wait();
            second()
        });
        (first.join().expect("first queue writer"), second.join().expect("second queue writer"))
    });
    let (winner, conflict, first_won) = match (first, second) {
        (Ok(winner), Err(conflict)) => (winner, conflict, true),
        (Err(conflict), Ok(winner)) => (winner, conflict, false),
        (first, second) => panic!("expected one successful queue write: {first:?}, {second:?}"),
    };
    assert!(conflict.is::<FailoverQueueVersionConflict>(), "{conflict:#}");
    (winner, first_won)
}
