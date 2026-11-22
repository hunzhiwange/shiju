use super::isolated_stdio::spawn_isolated_stdio;
use std::ffi::OsString;
use std::io::{BufRead, Write};
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
