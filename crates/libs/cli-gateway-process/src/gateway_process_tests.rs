use super::*;

#[test]
fn gateway_command_accepts_cli_gateway_and_global_config() {
    for command in [
        "/tmp/shiju gateway",
        "/tmp/shiju --config-dir /tmp/config gateway --port 8080",
        "/tmp/shiju --config-dir=/tmp/config gateway",
    ] {
        assert!(is_gateway_command("/tmp/shiju", command));
    }
    assert!(is_gateway_command("/tmp/my build/vibe-agent", "/tmp/my build/vibe-agent gateway"));
    assert!(is_gateway_command("/tmp/shiju", "target/debug/shiju gateway"));
    assert!(is_gateway_command("shiju", "/tmp/shiju gateway"));
}

#[test]
fn gateway_command_rejects_unrelated_processes_and_agent_prompts() {
    for (executable, command) in [
        ("/tmp/server", "/tmp/server gateway"),
        ("/tmp/shiju", "/tmp/shiju agent --message gateway"),
        ("/tmp/shiju", "/tmp/shiju daemon"),
        ("/tmp/shiju", "/tmp/shiju api gateway"),
        ("/tmp/shiju", "/tmp/shiju-other gateway"),
        ("/tmp/shiju", "/tmp/shiju --config-dir"),
    ] {
        assert!(!is_gateway_command(executable, command));
    }
}

#[tokio::test]
async fn prepare_gateway_port_allows_unused_address() {
    prepare_gateway_port("127.0.0.1", 0).await.unwrap();
}

#[tokio::test]
async fn prepare_gateway_port_preserves_unrelated_listener() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    assert!(prepare_gateway_port("127.0.0.1", address.port()).await.is_err());
    assert!(std::net::TcpStream::connect(address).is_ok());
}

#[test]
fn wait_for_port_observes_listener_release() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    assert!(!wait_for_port(address, Duration::ZERO).unwrap());
    drop(listener);
    assert!(wait_for_port(address, Duration::ZERO).unwrap());
}
