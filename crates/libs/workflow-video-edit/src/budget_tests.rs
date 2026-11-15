use super::*;
use serde_json::json;
#[test]
fn video_edit_concurrency_budget_rejects_invalid_limits() {
    assert_eq!(VideoEditRunBudget::from_value(None).unwrap().limit(), 5);
    for value in [json!(0), json!(-1), json!(1.5), json!(6), json!("5"), Value::Null] {
        assert!(VideoEditRunBudget::from_value(Some(&value)).is_err(), "{value}");
    }
    for limit in 1..=5 {
        assert_eq!(VideoEditRunBudget::new(limit).unwrap().limit(), limit);
    }
}
#[tokio::test]
async fn video_edit_concurrency_budget_cancel_rejects_queue_and_releases_permit() {
    let budget = VideoEditRunBudget::new(1).unwrap();
    let permit = budget.acquire().await.unwrap();
    let mut queued = Box::pin(budget.acquire());
    assert!(futures_util::poll!(&mut queued).is_pending());
    budget.cancel();
    assert!(queued.await.is_err());
    drop(permit);
    assert!(budget.acquire().await.is_err());
}
