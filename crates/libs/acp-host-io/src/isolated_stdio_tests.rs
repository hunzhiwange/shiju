use super::isolated_stdio::{
    ISOLATED_UNIX_RELAY_SOCKET, ISOLATED_UNIX_RELAY_WORKSPACE, spawn_isolated_stdio,
    spawn_isolated_stdio_with_unix_relay,
};
use std::ffi::OsString;
use std::io::{self, BufRead, Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::time::{Duration, timeout};

#[test]
#[ignore = "launched only inside the isolated stdio acceptance sandbox"]
fn stdio_fixture() {
    let mut line = String::new();
    std::io::stdin().lock().read_line(&mut line).unwrap();
    let (secret, port) = line.trim_end().split_once('\t').unwrap();
    assert_eq!(std::fs::read(secret).unwrap_err().kind(), std::io::ErrorKind::NotFound);
    assert_eq!(std::env::current_dir().unwrap(), Path::new("/tmp/work"));
    assert_eq!(std::env::var_os("HOME").unwrap(), "/tmp/home");
    assert!(std::env::var_os("PATH").is_none());
    assert!(std::env::var_os("USER").is_none());
    assert!(!Path::new("/proc/self").exists());
    std::fs::write("/tmp/work/private", "sandbox").unwrap();
    for address in [format!("127.0.0.1:{port}"), format!("[::1]:{port}")] {
        let error = std::net::TcpStream::connect(address).unwrap_err();
        assert_eq!(error.raw_os_error(), Some(libc::EPERM));
    }
    let error = std::os::unix::net::UnixStream::pair().unwrap_err();
    assert_eq!(error.raw_os_error(), Some(libc::EPERM));

    keep_stdout_open_after_setsid();
}

fn keep_stdout_open_after_setsid() {
    // The descendant leaves the original process group but keeps stdout open.
    let pid = unsafe { libc::fork() };
    assert!(pid >= 0);
    if pid == 0 {
        let ready = b"isolated-descendant-ready\n";
        unsafe {
            if libc::setsid() < 0 || libc::write(1, ready.as_ptr().cast(), ready.len()) < 0 {
                libc::_exit(1);
            }
            loop {
                libc::pause();
            }
        }
    }
    std::io::stdout().flush().unwrap();
    loop {
        std::thread::park();
    }
}

#[tokio::test]
async fn acp_platform_isolated_stdio_hides_host_and_cancels_descendants() {
    let root = tempfile::Builder::new().prefix("ACP 私有 ").tempdir().unwrap();
    let secret = root.path().join("host-secret");
    std::fs::write(&secret, "host marker").unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let entry = std::env::current_exe().unwrap();
    let args = ["--exact", "isolated_stdio_tests::stdio_fixture", "--ignored", "--nocapture"]
        .map(OsString::from);
    let mut sandbox = spawn_isolated_stdio(&entry, &args).unwrap();
    let mut stdout = BufReader::new(sandbox.take_stdout().unwrap());
    let mut stderr = sandbox.take_stderr().unwrap();
    let diagnostics = tokio::spawn(async move {
        let mut output = String::new();
        let _ = stderr.read_to_string(&mut output).await;
        output
    });
    let mut stdin = sandbox.take_stdin().unwrap();
    let input = format!("{}\t{port}\n", secret.display());
    let ready = timeout(Duration::from_secs(15), async {
        stdin.write_all(input.as_bytes()).await?;
        let mut line = String::new();
        loop {
            line.clear();
            if stdout.read_line(&mut line).await? == 0 {
                return Ok::<_, std::io::Error>(false);
            }
            if line == "isolated-descendant-ready\n" {
                return Ok(true);
            }
        }
    })
    .await;

    drop(stdin);
    timeout(Duration::from_secs(5), sandbox.cancel()).await.unwrap().unwrap();
    timeout(Duration::from_secs(5), stdout.read_to_end(&mut Vec::new())).await.unwrap().unwrap();
    let diagnostics = timeout(Duration::from_secs(5), diagnostics).await.unwrap().unwrap();
    assert!(matches!(ready, Ok(Ok(true))), "sandbox did not reach fixture: {diagnostics}");
    assert_eq!(std::fs::read_to_string(secret).unwrap(), "host marker");
}

#[test]
fn acp_platform_isolated_stdio_rejects_path_search() {
    assert!(matches!(
        spawn_isolated_stdio(Path::new("bash"), &[]),
        Err(error) if error.kind() == std::io::ErrorKind::InvalidInput
    ));
}

#[test]
#[ignore = "launched only inside the isolated Unix relay acceptance sandbox"]
fn unix_relay_fixture() {
    let mut line = String::new();
    io::stdin().lock().read_line(&mut line).unwrap();
    let (host_socket, port) = line.trim_end().split_once('\t').unwrap();
    assert_eq!(std::env::current_dir().unwrap(), Path::new(ISOLATED_UNIX_RELAY_WORKSPACE));
    assert_eq!(std::env::var("VW_ACP_LAUNCH_REFERENCE").unwrap(), "isolated-unix-relay");
    assert_eq!(
        std::fs::write("forbidden", b"write").unwrap_err().raw_os_error(),
        Some(libc::EROFS)
    );
    assert!(std::env::var_os("PATH").is_none());
    assert_eq!(UnixStream::connect(host_socket).unwrap_err().kind(), io::ErrorKind::NotFound);
    assert_eq!(
        UnixStream::connect("/run/shiju/other.sock").unwrap_err().kind(),
        io::ErrorKind::NotFound
    );
    assert_eq!(
        std::net::TcpStream::connect(format!("127.0.0.1:{port}")).unwrap_err().raw_os_error(),
        Some(libc::EPERM)
    );
    assert_eq!(
        UnixListener::bind("/tmp/other.sock").unwrap_err().raw_os_error(),
        Some(libc::EPERM)
    );

    let mut relay = UnixStream::connect(ISOLATED_UNIX_RELAY_SOCKET).unwrap();
    // A null message would yield EFAULT without the syscall filter.
    assert_eq!(unsafe { libc::recvmsg(relay.as_raw_fd(), std::ptr::null_mut(), 0) }, -1);
    assert_eq!(io::Error::last_os_error().raw_os_error(), Some(libc::EPERM));
    relay.write_all(b"GET /health HTTP/1.0\r\n\r\n").unwrap();
    let mut response = String::new();
    relay.read_to_string(&mut response).unwrap();
    assert_eq!(response, "HTTP/1.0 200 OK\r\nContent-Length: 2\r\n\r\nOK");
    drop(relay);
    keep_stdout_open_after_setsid();
}

#[tokio::test]
async fn acp_platform_isolated_unix_relay_reaches_only_the_host_request_socket() {
    let root = tempfile::Builder::new().prefix("ACP Unix 中继 ").tempdir().unwrap();
    let selected = root.path().join("relay.sock");
    let other = root.path().join("other.sock");
    let listener = UnixListener::bind(&selected).unwrap();
    let other_listener = UnixListener::bind(&other).unwrap();
    std::fs::set_permissions(&selected, std::fs::Permissions::from_mode(0o600)).unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let tcp_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let input = format!("{}\t{}\n", other.display(), tcp_listener.local_addr().unwrap().port());
    let args = ["--exact", "isolated_stdio_tests::unix_relay_fixture", "--ignored", "--nocapture"]
        .map(OsString::from);
    let entry = std::env::current_exe().unwrap();
    let mut sandbox = spawn_isolated_stdio_with_unix_relay(&entry, &args, &selected).unwrap();
    let mut stdout = BufReader::new(sandbox.take_stdout().unwrap());
    let mut stderr = sandbox.take_stderr().unwrap();
    let mut stdin = sandbox.take_stdin().unwrap();
    let diagnostics = tokio::spawn(async move {
        let mut output = String::new();
        let _ = stderr.read_to_string(&mut output).await;
        output
    });
    listener.set_nonblocking(true).unwrap();
    let host_relay = tokio::task::spawn_blocking(move || {
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        let mut stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    assert!(std::time::Instant::now() < deadline, "sandbox did not connect");
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("{error}"),
            }
        };
        stream.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
        stream.set_write_timeout(Some(Duration::from_secs(3))).unwrap();
        let mut request = [0; b"GET /health HTTP/1.0\r\n\r\n".len()];
        stream.read_exact(&mut request).unwrap();
        assert_eq!(&request, b"GET /health HTTP/1.0\r\n\r\n");
        stream.write_all(b"HTTP/1.0 200 OK\r\nContent-Length: 2\r\n\r\nOK").unwrap();
    });
    let ready = timeout(Duration::from_secs(15), async {
        stdin.write_all(input.as_bytes()).await?;
        let mut line = String::new();
        loop {
            line.clear();
            if stdout.read_line(&mut line).await? == 0 {
                return Ok::<_, io::Error>(false);
            }
            if line == "isolated-descendant-ready\n" {
                return Ok(true);
            }
        }
    })
    .await;

    drop(stdin);
    timeout(Duration::from_secs(5), sandbox.cancel()).await.unwrap().unwrap();
    timeout(Duration::from_secs(5), stdout.read_to_end(&mut Vec::new())).await.unwrap().unwrap();
    let diagnostics = timeout(Duration::from_secs(5), diagnostics).await.unwrap().unwrap();
    assert!(matches!(ready, Ok(Ok(true))), "sandbox did not reach fixture: {diagnostics}");
    host_relay.await.unwrap();
    other_listener.set_nonblocking(true).unwrap();
    assert_eq!(other_listener.accept().unwrap_err().kind(), io::ErrorKind::WouldBlock);
}

#[test]
fn acp_platform_isolated_unix_relay_rejects_missing_symlink_and_insecure_socket() {
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let entry = std::env::current_exe().unwrap();
    let missing = root.path().join("missing.sock");
    let selected = root.path().join("relay.sock");
    let link = root.path().join("link.sock");
    let _listener = UnixListener::bind(&selected).unwrap();
    std::fs::set_permissions(&selected, std::fs::Permissions::from_mode(0o666)).unwrap();
    symlink(&selected, &link).unwrap();
    for (path, kind) in [
        (&missing, io::ErrorKind::NotFound),
        (&link, io::ErrorKind::InvalidInput),
        (&selected, io::ErrorKind::PermissionDenied),
    ] {
        assert!(matches!(
            spawn_isolated_stdio_with_unix_relay(&entry, &[], path),
            Err(error) if error.kind() == kind
        ));
    }
}
