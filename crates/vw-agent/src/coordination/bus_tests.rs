use super::InMemoryMessageBus;
use crate::app::agent::coordination::InMemoryMessageBusLimits;

#[test]
fn message_bus_default_uses_recommended_limits() {
    let bus = InMemoryMessageBus::default();

    assert_eq!(bus.limits(), InMemoryMessageBusLimits::recommended());
    assert_eq!(bus.subscriber_count(), 0);
}

#[test]
fn message_bus_with_limits_keeps_custom_limits() {
    let limits = InMemoryMessageBusLimits {
        max_inbox_messages_per_agent: 1,
        max_dead_letters: 2,
        max_context_entries: 3,
        max_seen_message_ids: 4,
    };
    let bus = InMemoryMessageBus::with_limits(limits);

    assert_eq!(bus.limits(), limits);
}

#[test]
fn message_bus_recovers_from_poisoned_state_lock() {
    let bus = InMemoryMessageBus::new();
    let inner = bus.inner.clone();

    let _ = std::thread::spawn(move || {
        let _guard = inner.lock().expect("lock bus state");
        panic!("poison bus state");
    })
    .join();

    assert_eq!(bus.subscriber_count(), 0);
    assert_eq!(bus.limits(), InMemoryMessageBusLimits::recommended());
}
