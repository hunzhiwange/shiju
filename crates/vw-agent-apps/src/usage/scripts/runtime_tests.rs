use std::time::{Duration, Instant};

use serde_json::json;

use super::{
    config::{
        PreparedUsage, PreparedUsageScript, ProviderDefaults, ScriptParameters, UsageTemplate,
    },
    runtime::RuntimeError,
};

fn prepare(source: Option<&str>) -> PreparedUsageScript<'static> {
    let template = if source.is_some() { UsageTemplate::Custom } else { UsageTemplate::General };
    let prepared = template
        .prepare(
            source,
            ScriptParameters::default(),
            ProviderDefaults {
                api_key: Some("fixture-key"),
                base_url: Some("https://example.com"),
            },
        )
        .unwrap();
    let PreparedUsage::Script(script) = prepared else {
        panic!("expected a JavaScript template");
    };
    script
}

#[tokio::test]
async fn usage_scripts_runtime_returns_expected_result() {
    let script = prepare(None);
    assert_eq!(
        script.build_request().await.unwrap(),
        json!({
            "url": "https://example.com/user/balance",
            "method": "GET",
            "headers": {
                "Authorization": "Bearer fixture-key",
                "User-Agent": "cc-switch/1.0"
            }
        })
    );
    assert_eq!(
        script.extract(r#"{"is_active":true,"balance":7.5}"#).await.unwrap(),
        json!({"isValid": true, "remaining": 7.5, "unit": "USD"})
    );
}

#[tokio::test]
async fn usage_scripts_runtime_rejects_invalid_input() {
    let script = prepare(Some(r#"throw new Error("private fixture credential")"#));
    let error = script.build_request().await.unwrap_err();
    assert_eq!(error, RuntimeError::Evaluation);
    assert!(!format!("{error:?} {error}").contains("private fixture credential"));

    let script = prepare(Some("({request: {}, extractor: response => response})"));
    assert_eq!(script.extract("private fixture payload").await, Err(RuntimeError::InvalidResponse));
}

#[tokio::test(flavor = "current_thread")]
async fn usage_scripts_runtime_interrupts_request_without_blocking_async_executor() {
    let script = prepare(Some("for (;;) {}"));
    let start = Instant::now();
    let request = script.build_request();
    tokio::pin!(request);
    tokio::select! {
        biased;
        result = &mut request => panic!("request finished before executor timer: {result:?}"),
        () = tokio::time::sleep(Duration::from_millis(20)) => {}
    }
    assert_eq!(request.await, Err(RuntimeError::TimedOut));
    assert!((Duration::from_secs(5)..Duration::from_secs(15)).contains(&start.elapsed()));
}

#[tokio::test]
async fn usage_scripts_runtime_interrupts_extractor_loop() {
    let script = prepare(Some("({request: {}, extractor: () => { for (;;) {} }})"));
    let start = Instant::now();
    assert_eq!(script.extract("{}").await, Err(RuntimeError::TimedOut));
    assert!((Duration::from_secs(5)..Duration::from_secs(15)).contains(&start.elapsed()));
}

#[tokio::test]
async fn usage_scripts_runtime_bounds_memory_in_both_stages() {
    let source = r#"({
        get request() { return { buffer: new ArrayBuffer(17 * 1024 * 1024) }; },
        extractor: () => ({ buffer: new ArrayBuffer(17 * 1024 * 1024) })
    })"#;
    let script = prepare(Some(source));
    assert_eq!(script.build_request().await, Err(RuntimeError::InvalidRequest));
    assert_eq!(script.extract("{}").await, Err(RuntimeError::InvalidResult));
}

#[tokio::test]
async fn usage_scripts_runtime_bounds_stack_in_both_stages() {
    // Local slots force the recursion to use the engine stack before exhausting its heap.
    let source = r#"
        function recurse(depth) {
            let a = depth, b = depth, c = depth, d = depth, e = depth;
            let f = depth, g = depth, h = depth, i = depth, j = depth;
            if (depth === 0) return a + b + c + d + e + f + g + h + i + j;
            return recurse(depth - 1);
        }
        ({
            get request() { return { depth: recurse(3000) }; },
            extractor: () => ({ depth: recurse(3000) })
        })
    "#;
    let script = prepare(Some(source));
    assert_eq!(script.build_request().await, Err(RuntimeError::InvalidRequest));
    assert_eq!(script.extract("{}").await, Err(RuntimeError::InvalidResult));
}
