use super::*;
use crate::app_routing::capacity::RouteCapacity;
use std::sync::Arc;
use tokio::sync::Semaphore;

#[tokio::test(start_paused = true)]
async fn ha_idle_and_total_deadlines_bound_stream_releases_permit() {
    let capacity = RouteCapacity::default();
    let buffers = Arc::new(Semaphore::new(1));
    let permit = capacity.acquire(buffers.clone().acquire_owned().await.unwrap()).await.unwrap();
    let sample = crate::model_relay::stream_observer::tests::samples()
        .into_iter()
        .find(|(kind, _, _)| *kind == "anthropic")
        .unwrap()
        .2;
    let prelude = sample.split_inclusive("\n\n").take(3).collect::<String>();
    let upstream = stream::once(async move { Ok::<_, std::io::Error>(Bytes::from(prelude)) })
        .chain(stream::pending());
    let body = stream_body(
        Body::from_stream(upstream),
        Instant::now() + Duration::from_secs(1),
        Duration::from_secs(1),
        permit,
        Protocol::Messages,
    )
    .await
    .unwrap_or_else(|error| panic!("stream setup failed: {}", error.status()));
    let mut body = body.into_data_stream();
    assert!(body.next().await.unwrap().is_ok());
    tokio::time::advance(Duration::from_secs(2)).await;
    let terminal = body.next().await.unwrap().unwrap();
    assert!(String::from_utf8_lossy(&terminal).contains("The upstream response deadline elapsed."));
    // Keep the downstream body alive, without polling EOF or dropping it.
    assert_eq!(
        buffers.available_permits(),
        1,
        "terminal must release request capacity immediately"
    );
    assert!(body.next().await.is_none());
}
