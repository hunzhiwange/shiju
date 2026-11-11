use super::*;
#[tokio::test]
async fn voice_task_06_request_registry_deduplicates_and_cancels_without_crossing_owner() {
    let registry = Arc::new(Requests::default());
    let owner = Owner { tenant: TenantScope::new("p", "a").unwrap(), principal: [1; 32] };
    let other = Owner { tenant: TenantScope::new("p", "b").unwrap(), principal: [1; 32] };
    let Start::Run(pending) = registry.start(owner.clone(), "id", [9; 32]).unwrap() else {
        panic!("new request")
    };
    let Start::Wait(mut receiver) = registry.start(owner.clone(), "id", [9; 32]).unwrap() else {
        panic!("duplicate must wait")
    };
    assert!(matches!(registry.start(owner.clone(), "id", [8; 32]), Err(Code::Conflict)));
    assert!(matches!(registry.start(owner, "new-id", [9; 32]), Err(Code::Conflict)));
    assert!(matches!(registry.start(other, "id", [9; 32]), Ok(Start::Run(_))));
    drop(pending);
    receiver.changed().await.unwrap();
    assert_eq!(receiver.borrow().clone(), Some(Err(Code::Cancelled)));
}

#[tokio::test]
async fn voice_task_06_cancellation_tombstone_prevents_late_upload() {
    let registry = Arc::new(Requests::default());
    let owner = Owner { tenant: TenantScope::new("p", "a").unwrap(), principal: [1; 32] };
    registry.cancel(owner.clone(), "early".into());
    let Start::Wait(receiver) = registry.start(owner, "early", [9; 32]).unwrap() else {
        panic!("cancelled upload restarted")
    };
    assert_eq!(receiver.borrow().clone(), Some(Err(Code::Cancelled)));
}

#[tokio::test]
async fn voice_task_06_cancel_is_scoped_to_principal_and_terminal_records_expire() {
    let registry = Arc::new(Requests::default());
    let owner = Owner { tenant: TenantScope::new("p", "a").unwrap(), principal: [1; 32] };
    let other = Owner { principal: [2; 32], ..owner.clone() };
    let Start::Run(pending) = registry.start(owner.clone(), "id", [9; 32]).unwrap() else {
        panic!("new request")
    };
    registry.cancel(other, "id".into());
    assert!(!pending.cancel.is_cancelled());
    pending.finish(Err(Code::ProviderUnavailable));
    registry.0.lock().unwrap().get_mut(&owner).unwrap().get_mut("id").unwrap().completed =
        Some(Instant::now() - Duration::from_secs(301));
    assert!(matches!(registry.start(owner, "id", [8; 32]), Ok(Start::Run(_))));
}

#[test]
fn voice_task_06_http_intake_has_bounded_concurrency() {
    let registry = Requests::default();
    let slots = (0..4).map(|_| registry.upload_slot().unwrap()).collect::<Vec<_>>();
    assert!(matches!(registry.upload_slot(), Err(Code::Backpressure)));
    drop(slots);
    assert!(registry.upload_slot().is_ok());
}
