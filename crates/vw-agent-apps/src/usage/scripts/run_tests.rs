use std::time::Duration;

use reqwest::Url;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    time::{sleep, timeout},
};

use super::{
    config::{
        PreparedUsage, PreparedUsageScript, ProviderDefaults, ScriptParameters, UsageTemplate,
    },
    run::RunError,
};
use crate::apps::lifecycle::operations::Operation;

const DEADLINE: Duration = Duration::from_secs(2);

fn prepare(source: &str) -> PreparedUsageScript<'_> {
    let PreparedUsage::Script(script) = UsageTemplate::Custom
        .prepare(Some(source), ScriptParameters::default(), ProviderDefaults::default())
        .unwrap()
    else {
        panic!("expected prepared script");
    };
    script
}

async fn bind() -> (TcpListener, Url) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = Url::parse(&format!("http://{}/balance", listener.local_addr().unwrap())).unwrap();
    (listener, url)
}

fn query(url: &Url, extractor: &str) -> String {
    format!(
        "({{request: {{url: '{}', method: 'GET', headers: {{'X-Fixture': 'quota'}}}}, extractor: {extractor}}})",
        url.as_str()
    )
}

async fn receive_request(listener: &TcpListener) -> TcpStream {
    timeout(DEADLINE, async {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        while !request.ends_with(b"\r\n\r\n") {
            assert_ne!(stream.read_buf(&mut request).await.unwrap(), 0);
        }
        let request = String::from_utf8(request).unwrap().to_ascii_lowercase();
        assert!(request.starts_with("get /balance http/1.1\r\n"));
        assert!(request.contains("\r\nx-fixture: quota\r\n"));
        stream
    })
    .await
    .unwrap()
}

async fn respond(stream: &mut TcpStream) {
    stream
        .write_all(
            b"HTTP/1.1 200 OK\r\nContent-Length: 14\r\nConnection: close\r\n\r\n{\"balance\":12}",
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn usage_scripts_run_returns_expected_result() {
    let (listener, url) = bind().await;
    let source = query(&url, "(data) => ({remaining: data.balance})");
    let script = prepare(&source);
    let operation = Operation::new();
    let (result, ()) =
        tokio::join!(script.run(10, std::slice::from_ref(&url), &operation), async {
            respond(&mut receive_request(&listener).await).await
        },);
    let packages = result.unwrap();
    assert_eq!(packages.len(), 1);
    assert_eq!(packages[0].remaining.map(f64::from), Some(12.0));
}

#[tokio::test]
async fn usage_scripts_run_rejects_invalid_input() {
    let (listener, url) = bind().await;
    let operation = Operation::new();
    operation.cancel();
    let source = query(&url, "(data) => ({remaining: data.balance})");
    let script = prepare(&source);

    assert_eq!(script.run(10, &[url], &operation).await, Err(RunError::Cancelled));
    assert!(timeout(Duration::from_millis(50), listener.accept()).await.is_err());
}

#[tokio::test]
async fn usage_scripts_run_interrupts_request_construction() {
    let script = prepare("({get request() { while (true) {} }})");
    let operation = Operation::new();
    let (result, ()) = timeout(DEADLINE, async {
        tokio::join!(script.run(10, &[], &operation), async {
            sleep(Duration::from_millis(100)).await;
            operation.cancel();
        })
    })
    .await
    .unwrap();
    assert_eq!(result, Err(RunError::Cancelled));
}

#[tokio::test]
async fn usage_scripts_run_cancels_http_body_without_cancelling_another_query() {
    let (listener, url) = bind().await;
    let cancelled = Operation::new();
    let other = Operation::new();
    let source = query(&url, "(data) => ({remaining: data.balance})");
    let script = prepare(&source);
    let origins = std::slice::from_ref(&url);
    let (result, packages) = timeout(DEADLINE, async {
        tokio::join!(script.run(10, origins, &cancelled), async {
            let mut stalled = receive_request(&listener).await;
            stalled.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 14\r\n\r\n{").await.unwrap();
            let (result, ()) = tokio::join!(script.run(10, origins, &other), async {
                let mut next = receive_request(&listener).await;
                cancelled.cancel();
                assert_eq!(stalled.read(&mut [0]).await.unwrap(), 0);
                respond(&mut next).await;
            });
            result.unwrap()
        })
    })
    .await
    .unwrap();
    assert_eq!(result, Err(RunError::Cancelled));
    assert_eq!(packages[0].remaining.map(f64::from), Some(12.0));
}

#[tokio::test]
async fn usage_scripts_run_interrupts_extractor() {
    let (listener, url) = bind().await;
    let source = query(&url, "() => { while (true) {} }");
    let script = prepare(&source);
    let operation = Operation::new();
    let (result, ()) = timeout(DEADLINE, async {
        tokio::join!(script.run(10, std::slice::from_ref(&url), &operation), async {
            respond(&mut receive_request(&listener).await).await;
            sleep(Duration::from_millis(100)).await;
            operation.cancel();
        })
    })
    .await
    .unwrap();
    assert_eq!(result, Err(RunError::Cancelled));
}
