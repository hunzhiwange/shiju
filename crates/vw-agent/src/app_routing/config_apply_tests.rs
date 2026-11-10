use super::{
    RoutingListener,
    config_apply::{AppTimeouts, ListenerConfig, RoutingConfigStore},
};
use crate::global_database::{GlobalDatabase, test_support::PostgresTestDatabase};
use std::{
    net::{Ipv4Addr, TcpListener},
    sync::Arc,
};
use vw_api_types::{
    agent_apps::AppId,
    id::{ProjectScope, TenantScope},
};

#[test]
fn cc_route_011_config_apply_expected_behavior() -> anyhow::Result<()> {
    let home = tempfile::tempdir()?;
    verify_config_apply(GlobalDatabase::for_home(home.path())?)
}

#[test]
fn cc_route_011_config_apply_postgres_expected_behavior() -> anyhow::Result<()> {
    let fixture = PostgresTestDatabase::from_environment()?;
    verify_config_apply(fixture.database().clone())
}

fn verify_config_apply(database: GlobalDatabase) -> anyhow::Result<()> {
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
    let tenant = TenantScope::new("p", "c")?;
    let project = ProjectScope::new(tenant.clone(), "project".into())?;
    let other_project = ProjectScope::new(tenant.clone(), "other".into())?;
    let store = Arc::new(RoutingConfigStore::new(database.clone(), tenant.clone(), "mac".into())?);
    let claude = AppTimeouts {
        first_byte_timeout_secs: 60,
        idle_timeout_secs: 120,
        non_streaming_timeout_secs: 600,
    };
    let codex = AppTimeouts { first_byte_timeout_secs: 61, ..claude };
    store.save_app_timeouts(&project, AppId::Claude, claude)?;
    store.save_app_timeouts(&project, AppId::Codex, codex)?;

    let routing = Arc::new(RoutingListener::default());
    let initial = ListenerConfig {
        bind: (Ipv4Addr::LOCALHOST, 0).into(),
        enabled: true,
        logging_enabled: false,
    };
    runtime.block_on(store.apply_listener(&routing, initial, initial))?;
    let bound = runtime.block_on(async { routing.start_configured(initial) })?;
    let snapshot = routing.snapshot();

    let occupied = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
    let rejected =
        ListenerConfig { bind: occupied.local_addr()?, logging_enabled: true, ..initial };
    assert!(runtime.block_on(store.apply_listener(&routing, initial, rejected)).is_err());
    assert_eq!(
        store.listener()?,
        Some(initial),
        "failed rebind replaced the stored listener configuration"
    );
    assert_eq!(routing.snapshot(), snapshot, "failed rebind disturbed the original listener");
    assert!(!routing.logging_enabled());
    assert_eq!(http_status(&runtime, bound)?, reqwest::StatusCode::UNAUTHORIZED);
    let invalid = ListenerConfig { bind: (Ipv4Addr::UNSPECIFIED, 0).into(), ..initial };
    assert!(runtime.block_on(store.apply_listener(&routing, initial, invalid)).is_err());
    assert_eq!(store.listener()?, Some(initial));

    let hot = ListenerConfig { logging_enabled: true, ..initial };
    runtime.block_on(store.apply_listener(&routing, initial, hot))?;
    assert_eq!(routing.snapshot(), snapshot, "a hot update restarted the listener");
    assert!(routing.logging_enabled());
    let updated = AppTimeouts { idle_timeout_secs: 0, ..claude };
    store.save_app_timeouts(&project, AppId::Claude, updated)?;
    assert!(
        store
            .save_app_timeouts(
                &project,
                AppId::Claude,
                AppTimeouts { first_byte_timeout_secs: 0, ..updated }
            )
            .is_err()
    );
    assert_eq!(routing.snapshot(), snapshot, "an app update restarted the listener");

    let new_binding = occupied.local_addr()?;
    drop(occupied);
    let rebound = ListenerConfig { bind: new_binding, logging_enabled: false, ..hot };
    reject_database_updates(&database, true)?;
    assert!(runtime.block_on(store.apply_listener(&routing, initial, rebound)).is_err());
    assert_eq!(store.listener()?, Some(hot));
    assert_eq!(routing.snapshot(), snapshot, "a failed write disturbed the original listener");
    assert!(routing.logging_enabled());
    drop(TcpListener::bind(new_binding)?);
    assert_eq!(http_status(&runtime, bound)?, reqwest::StatusCode::UNAUTHORIZED);
    reject_database_updates(&database, false)?;

    runtime.block_on(store.apply_listener(&routing, hot, rebound))?;
    assert_eq!(routing.bound_address(), Some(new_binding));
    assert!(!routing.logging_enabled());
    assert_eq!(http_status(&runtime, new_binding)?, reqwest::StatusCode::UNAUTHORIZED);
    assert!(runtime.block_on(async { tokio::net::TcpStream::connect(bound).await }).is_err());

    let reopened = RoutingConfigStore::new(database.clone(), tenant.clone(), "mac".into())?;
    assert_eq!(reopened.listener()?, Some(rebound));
    assert_eq!(reopened.app_timeouts(&project, AppId::Claude)?, Some(updated));
    assert_eq!(reopened.app_timeouts(&project, AppId::Codex)?, Some(codex));
    assert_eq!(reopened.app_timeouts(&other_project, AppId::Claude)?, None);
    let other_host = RoutingConfigStore::new(database, tenant, "other-mac".into())?;
    assert_eq!(other_host.listener()?, None);
    assert_eq!(other_host.app_timeouts(&project, AppId::Claude)?, None);
    runtime.block_on(routing.stop())?;
    Ok(())
}

fn reject_database_updates(database: &GlobalDatabase, reject: bool) -> anyhow::Result<()> {
    use crate::global_database::config::DatabaseProvider;
    match (database.provider(), reject) {
        (DatabaseProvider::Sqlite, true) => database.connect_sqlite()?.execute_batch(
            "CREATE TRIGGER reject_update AFTER UPDATE ON gateway_settings
             BEGIN SELECT RAISE(ABORT, 'rejected'); END;",
        )?,
        (DatabaseProvider::Sqlite, false) => {
            database.connect_sqlite()?.execute_batch("DROP TRIGGER reject_update;")?;
        }
        (DatabaseProvider::Postgres, true) => database.connect_postgres()?.batch_execute(
            "CREATE FUNCTION reject_update() RETURNS trigger LANGUAGE plpgsql AS
             $$ BEGIN RAISE EXCEPTION 'rejected'; END $$;
             CREATE TRIGGER reject_update AFTER UPDATE ON gateway_settings
             FOR EACH ROW EXECUTE FUNCTION reject_update();",
        )?,
        (DatabaseProvider::Postgres, false) => database.connect_postgres()?.batch_execute(
            "DROP TRIGGER reject_update ON gateway_settings; DROP FUNCTION reject_update();",
        )?,
    }
    Ok(())
}

fn http_status(
    runtime: &tokio::runtime::Runtime,
    address: std::net::SocketAddr,
) -> anyhow::Result<reqwest::StatusCode> {
    runtime.block_on(async {
        let client = reqwest::Client::builder()
            .no_proxy()
            .timeout(std::time::Duration::from_secs(3))
            .build()?;
        Ok(client.get(format!("http://{address}/v1/messages")).send().await?.status())
    })
}
