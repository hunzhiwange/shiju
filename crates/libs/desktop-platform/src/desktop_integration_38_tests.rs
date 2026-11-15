use super::Inbox;

#[test]
fn desktop_integration_38_preserves_expected_behavior() {
    let mut inbox = Inbox::default();
    let cold = "ccswitch://v1/import?resource=provider&name=cold";
    let warm = "ccswitch://v1/import?resource=skill&repo=owner/warm";
    inbox.receive(cold).unwrap();
    let cold_id = inbox.pending.front().unwrap().id;
    inbox.receive(cold).unwrap();
    inbox.receive(warm).unwrap();
    assert_eq!(inbox.pending.len(), 2);
    let warm_id = inbox.pending.back().unwrap().id;
    assert_ne!(cold_id, warm_id);
    assert!(!inbox.acknowledge(warm_id + 1));
    assert_eq!(inbox.pending.len(), 2);
    assert!(inbox.acknowledge(cold_id));
    assert!(!inbox.acknowledge(cold_id));
    assert_eq!(inbox.pending.front().unwrap().url, warm);
    inbox.receive(cold).unwrap();
    assert!(inbox.pending.back().unwrap().id > warm_id);
    assert!(inbox.receive("ccswitch://bad\nvalue").is_err());
    assert_eq!(inbox.pending.len(), 2);
}
