use super::*;
use std::{
    future::{pending, poll_fn},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    task::Poll,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::oneshot,
};

async fn upstream(status: u16) -> (String, Arc<AtomicUsize>, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/", listener.local_addr().unwrap());
    let requests = Arc::new(AtomicUsize::new(0));
    let count = requests.clone();
    let server = tokio::spawn(async move {
        loop {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            while !request.ends_with(b"\r\n\r\n") {
                request.push(stream.read_u8().await.unwrap());
            }
            count.fetch_add(1, Ordering::Relaxed);
            let response =
                format!("HTTP/1.1 {status} Test\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
            stream.write_all(response.as_bytes()).await.unwrap();
        }
    });
    (url, requests, server)
}

fn http_failure(status: u16, replay: ReplayEvidence) -> RetryFailure {
    RetryFailure {
        phase: FailurePhase::ResponseHeaders,
        code: FailureCode::HttpStatus(status),
        replay,
    }
}

#[tokio::test]
async fn ha_retry_budget_bounds_all_attempts() {
    let (primary, primary_requests, primary_server) = upstream(503).await;
    let (backup, backup_requests, backup_server) = upstream(429).await;
    let client = reqwest::Client::builder()
        .no_proxy()
        .retry(reqwest::retry::never())
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();

    for (retries, candidates) in [(0, vec![&primary]), (2, vec![&primary, &backup, &backup])] {
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut budget = RetryBudget::new(retries, Duration::ZERO, deadline).unwrap();
        let mut failure = None;
        for url in candidates {
            assert_eq!(
                budget.begin_attempt(failure, pending()).await,
                Ok(()),
                "a proven replayable failure must allow the next send from the same budget"
            );
            let status = client.get(url).send().await.unwrap().status().as_u16();
            // The fixture only reads headers and returns a fixed error; no action is executed.
            failure = Some(http_failure(status, ReplayEvidence::ProvenSafe));
        }
        assert_eq!(budget.begin_attempt(failure, pending()).await, Err(RetryStop::Exhausted));
    }
    assert_eq!(primary_requests.load(Ordering::Relaxed), 2);
    assert_eq!(backup_requests.load(Ordering::Relaxed), 2);

    for (status, replay) in [
        (400, ReplayEvidence::ProvenSafe),
        (401, ReplayEvidence::ProvenSafe),
        (403, ReplayEvidence::ProvenSafe),
        (501, ReplayEvidence::ProvenSafe),
        (429, ReplayEvidence::Unknown),
        (503, ReplayEvidence::Committed),
    ] {
        let failure = http_failure(status, replay);
        let mut budget =
            RetryBudget::new(2, Duration::ZERO, Instant::now() + Duration::from_secs(10)).unwrap();
        budget.begin_attempt(None, pending()).await.unwrap();
        assert_eq!(
            budget.begin_attempt(Some(failure), pending()).await,
            Err(RetryStop::Rejected(failure))
        );
        assert_eq!(budget.attempts, 1);
    }

    primary_server.abort();
    backup_server.abort();
    assert!(primary_server.await.unwrap_err().is_cancelled());
    assert!(backup_server.await.unwrap_err().is_cancelled());
}

#[test]
fn ha_retry_budget_bounds_all_attempts_by_deadline_and_backoff_cap() {
    let now = Instant::now();
    let deadline = now + Duration::from_secs(10);
    let mut budget = RetryBudget::new(10, Duration::from_millis(750), deadline).unwrap();
    let failure = Some(RetryFailure {
        phase: FailurePhase::Connect,
        code: FailureCode::Connection,
        replay: ReplayEvidence::ProvenSafe,
    });
    for (attempts, millis) in [(1, 750), (2, 1_500), (3, 3_000), (4, 5_000), (10, 5_000)] {
        budget.attempts = attempts;
        assert_eq!(budget.next_delay(failure, now), Ok(Duration::from_millis(millis)));
    }
    assert_eq!(
        budget.next_delay(failure, deadline - Duration::from_secs(5)),
        Err(RetryStop::DeadlineExceeded)
    );
    assert_eq!(budget.next_delay(failure, deadline), Err(RetryStop::DeadlineExceeded));
    assert!(matches!(
        RetryBudget::new(11, Duration::ZERO, deadline),
        Err(RetryStop::InvalidMaxRetries)
    ));
}

#[tokio::test]
async fn ha_retry_budget_bounds_all_attempts_when_cancelled_during_backoff() {
    let mut budget =
        RetryBudget::new(2, Duration::from_secs(5), Instant::now() + Duration::from_secs(10))
            .unwrap();
    budget.begin_attempt(None, pending()).await.unwrap();
    let failure = Some(http_failure(503, ReplayEvidence::ProvenSafe));
    let (send_cancel, cancelled) = oneshot::channel();
    {
        let begin = budget.begin_attempt(failure, async { cancelled.await.unwrap() });
        tokio::pin!(begin);
        poll_fn(|cx| {
            assert!(begin.as_mut().poll(cx).is_pending(), "retry must wait before sending");
            Poll::Ready(())
        })
        .await;
        send_cancel.send(()).unwrap();
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(1), begin).await.unwrap(),
            Err(RetryStop::Cancelled)
        );
    }
    assert_eq!(budget.attempts, 1);
    assert_eq!(budget.begin_attempt(failure, pending()).await, Err(RetryStop::Cancelled));
}
