use serde_json::{json, Value};
use std::path::Path;
use std::os::unix::fs::PermissionsExt;
use std::{fs, net::Ipv4Addr};
use tempfile::TempDir;
fn host(home: &Path, port: u16) -> Value {
    json!({
        "id": "route-test-host",
        "online": true,
        "binding": "local",
        "home": home,
        "search_paths": [],
        "applications": home.join("Applications"),
        "routing_startup": { "enabled": true, "listen_port": port },
    })
}

fn write_policy(path: &Path, hosts: &[Value]) {
    fs::write(path, json!({ "principals": [], "hosts": hosts }).to_string()).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}

#[test]
fn cc_route_007_gateway_owner_rejects_untrusted_startup() {
    use app_access_service::access::routing_setup;

    let fixture = TempDir::new().unwrap();
    let policy = fixture.path().join("access.json");
    let authorized = host(fixture.path(), 0);
    let read = || routing_setup(&policy).map(|setup| setup.startup_address);
    assert!(read().is_err(), "missing policy");

    for (field, value) in [("online", json!(false)), ("binding", json!("remote"))] {
        let mut denied = authorized.clone();
        denied[field] = value;
        write_policy(&policy, &[denied]);
        assert_eq!(read().unwrap(), None);
    }
    let mut denied = authorized.clone();
    denied["routing_startup"]["enabled"] = json!(false);
    write_policy(&policy, &[denied]);
    assert_eq!(read().unwrap(), None);
    let mut absent = authorized.clone();
    absent.as_object_mut().unwrap().remove("routing_startup");
    write_policy(&policy, &[absent]);
    assert_eq!(read().unwrap(), None);

    let mut non_loopback = authorized.clone();
    non_loopback["routing_startup"]["listen_address"] = json!("0.0.0.0");
    write_policy(&policy, &[non_loopback]);
    assert!(read().is_err());

    let mut second = authorized.clone();
    second["id"] = json!("another-host");
    write_policy(&policy, &[authorized.clone(), second]);
    assert!(read().is_err(), "ambiguous host policy");

    write_policy(&policy, &[authorized]);
    assert_eq!(read().unwrap().unwrap(), (Ipv4Addr::LOCALHOST, 0).into());
    fs::set_permissions(&policy, fs::Permissions::from_mode(0o666)).unwrap();
    assert!(read().is_err(), "policy is writable by another principal");
    fs::set_permissions(&policy, fs::Permissions::from_mode(0o600)).unwrap();
    fs::write(&policy, b"{broken").unwrap();
    assert!(read().is_err(), "damaged policy");
}
