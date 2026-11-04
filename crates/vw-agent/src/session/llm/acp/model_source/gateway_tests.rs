use super::*;

#[tokio::test]
async fn acp_model_source_openclaw_owned_gateway_reaps_process_group() {
    let reservation = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = reservation.local_addr().unwrap();
    let mut command = Command::new("python3");
    command.args(["-c", "import socket,subprocess,sys,time; subprocess.Popen([sys.executable,'-c','import time;time.sleep(120)']); s=socket.socket();s.bind(('127.0.0.1',int(sys.argv[1])));s.listen();time.sleep(120)", &address.port().to_string()]);
    drop(reservation);
    let mut gateway = OwnedGateway::new(command, address);
    gateway.start().await.unwrap();
    let pid = gateway.child.as_ref().unwrap().id();
    drop(gateway);
    assert!(std::net::TcpStream::connect(address).is_err());
    assert_eq!(unsafe { libc::kill(pid as i32, 0) }, -1);
    // The sleeping grandchild must be gone too; killing only the root fails this.
    for _ in 0..100 {
        if unsafe { libc::kill(-(pid as i32), 0) } == -1 {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("owned Gateway left a descendant process group alive");
}

#[tokio::test]
async fn acp_model_source_openclaw_gateway_start_failure_is_explicit() {
    let mut command = Command::new("python3");
    command.args(["-c", "raise SystemExit(1)"]);
    let mut gateway = OwnedGateway::new(command, "127.0.0.1:1".parse().unwrap());
    assert!(gateway.start().await.unwrap_err().to_string().contains("exited before readiness"));
}

#[tokio::test]
async fn acp_model_source_openclaw_cancelled_startup_reaps_gateway() {
    let reservation = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = reservation.local_addr().unwrap();
    drop(reservation);
    let mut command = Command::new("python3");
    command.args(["-c", "import time; time.sleep(120)"]);
    let mut gateway = OwnedGateway::new(command, address);
    assert!(tokio::time::timeout(Duration::from_millis(100), gateway.start()).await.is_err());
    let pid = gateway.child.as_ref().unwrap().id();
    drop(gateway);
    assert_eq!(unsafe { libc::kill(pid as i32, 0) }, -1);
}
