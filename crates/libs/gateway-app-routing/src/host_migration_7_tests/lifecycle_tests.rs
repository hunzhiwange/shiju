use super::fixture::*;
use serde_json::{Value, json};
use std::{fs, net::{Ipv4Addr, TcpListener}};
use vw_api_transport::id::TenantScope;
use vw_api_app_routing::RoutingState;

#[tokio::test]
async fn cc_route_011_lifecycle_reads_the_persisted_listener_config() -> anyhow::Result<()> {
    use {
        app_routing_services::{
            RoutingListener,
            config_apply::{ListenerConfig, RoutingConfigStore},
        },
        agent_database::GlobalDatabase,
    };
    use std::sync::Arc;

    let home = tempfile::tempdir()?;
    let policy = home.path().join("access.json");
    let occupied = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
    let fallback = occupied.local_addr()?;
    write_policy(&policy, home.path(), fallback.port());
    let mut value: Value = serde_json::from_slice(&fs::read(&policy)?)?;
    value["hosts"][0]["routing_startup"]["enabled"] = json!(true);
    fs::write(&policy, value.to_string())?;

    let store = Arc::new(RoutingConfigStore::new(
        GlobalDatabase::for_home(home.path())?,
        TenantScope::new("p", "owner")?,
        HOST.into(),
    )?);
    let saved = ListenerConfig {
        enabled: false,
        bind: (Ipv4Addr::LOCALHOST, 0).into(),
        logging_enabled: true,
    };
    store
        .apply_listener(
            &Arc::new(RoutingListener::default()),
            ListenerConfig { enabled: true, bind: fallback, logging_enabled: false },
            saved,
            || Ok(()),
        )
        .await?;

    let (_gateway, port) = spawn_gateway(home.path(), &policy).await;
    let client = client(port, "owner");
    assert_eq!(crate::GatewayAppRoutingClient::new(client.clone()).app_routing_status(HOST, READER).await?.state, RoutingState::Stopped);
    let started = crate::GatewayAppRoutingClient::new(client.clone()).app_routing_start(HOST, MANAGER).await?;
    assert_eq!(started.state, RoutingState::Running);
    assert!(started.bound_port.is_some_and(|port| port != 0 && port != fallback.port()));
    crate::GatewayAppRoutingClient::new(client.clone()).app_routing_stop(HOST, MANAGER).await?;
    Ok(())
}

#[tokio::test]
async fn cc_route_010_lifecycle_stop_preserves_attached_apps() {
    let home = tempfile::TempDir::new().unwrap();
    let policy = home.path().join("access.json");
    write_policy(&policy, home.path(), 0);
    let (_gateway, port) = spawn_gateway(home.path(), &policy).await;
    let owner = client(port, "owner");
    let running = crate::GatewayAppRoutingClient::new(owner.clone()).app_routing_start(HOST, MANAGER).await.unwrap();
    let address = (running.bound_address.unwrap(), running.bound_port.unwrap());

    let (host_state, attached) = write_attached_app(home.path());

    assert_error(crate::GatewayAppRoutingClient::new(owner.clone()).app_routing_stop(HOST, MANAGER).await.unwrap_err(), 409, "apps_attached");
    assert_eq!(fs::read_to_string(&host_state).unwrap(), attached);
    assert_eq!(crate::GatewayAppRoutingClient::new(owner.clone()).app_routing_status(HOST, READER).await.unwrap(), running);
    assert!(tokio::net::TcpStream::connect(address).await.is_ok());
    fs::remove_file(host_state).unwrap();
    assert_eq!(crate::GatewayAppRoutingClient::new(owner.clone()).app_routing_stop(HOST, MANAGER).await.unwrap().state, RoutingState::Stopped);
    assert!(tokio::net::TcpStream::connect(address).await.is_err());
}

#[tokio::test]
async fn cc_route_010_lifecycle_policy_reload_cannot_move_a_running_listener() {
    let home = tempfile::TempDir::new().unwrap();
    let policy = home.path().join("access.json");
    write_policy(&policy, home.path(), 0);
    let (_gateway, port) = spawn_gateway(home.path(), &policy).await;
    let owner = client(port, "owner");
    let other = client(port, "other");
    let running = crate::GatewayAppRoutingClient::new(owner.clone()).app_routing_start(HOST, MANAGER).await.unwrap();
    let address = (running.bound_address.unwrap(), running.bound_port.unwrap());

    let original = fs::read_to_string(&policy).unwrap();
    let mut edited: Value = serde_json::from_str(&original).unwrap();
    edited["principals"][0]["grants"][0]["company_id"] = json!("other");
    fs::write(&policy, edited.to_string()).unwrap();
    assert_error(crate::GatewayAppRoutingClient::new(owner.clone()).app_routing_stop(HOST, MANAGER).await.unwrap_err(), 403, "forbidden");
    assert_error(crate::GatewayAppRoutingClient::new(other.clone()).app_routing_stop(HOST, MANAGER).await.unwrap_err(), 403, "forbidden");
    assert_error(crate::GatewayAppRoutingClient::new(other.clone()).app_routing_status(HOST, MANAGER).await.unwrap_err(), 403, "forbidden");
    assert!(tokio::net::TcpStream::connect(address).await.is_ok());

    fs::write(&policy, original).unwrap();
    assert_eq!(crate::GatewayAppRoutingClient::new(owner.clone()).app_routing_stop(HOST, MANAGER).await.unwrap().state, RoutingState::Stopped);
}
