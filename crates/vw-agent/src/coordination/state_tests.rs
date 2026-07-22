use super::state::BusState;
use super::types::{InMemoryMessageBusLimits, InMemoryMessageBusStats};

#[test]
fn bus_state_preserves_supplied_limits() {
    let limits = InMemoryMessageBusLimits {
        max_inbox_messages_per_agent: 2,
        max_dead_letters: 3,
        max_context_entries: 4,
        max_seen_message_ids: 5,
    };

    let state = BusState::with_limits(limits);

    assert_eq!(state.limits, limits);
    assert_eq!(state.next_sequence, 1);
}

#[test]
fn bus_state_clamps_zero_limits_to_one() {
    let state = BusState::with_limits(InMemoryMessageBusLimits {
        max_inbox_messages_per_agent: 0,
        max_dead_letters: 0,
        max_context_entries: 0,
        max_seen_message_ids: 0,
    });

    assert_eq!(
        state.limits,
        InMemoryMessageBusLimits {
            max_inbox_messages_per_agent: 1,
            max_dead_letters: 1,
            max_context_entries: 1,
            max_seen_message_ids: 1,
        }
    );
}

#[test]
fn bus_state_starts_with_empty_storage_and_default_stats() {
    let state = BusState::with_limits(InMemoryMessageBusLimits::recommended());

    assert_eq!(state.next_sequence, 1);
    assert!(state.seen_message_ids.is_empty());
    assert!(state.seen_message_order.is_empty());
    assert!(state.inboxes.is_empty());
    assert!(state.inbox_correlation_counts.is_empty());
    assert!(state.dead_letters.is_empty());
    assert!(state.dead_letters_by_correlation.is_empty());
    assert!(state.context.is_empty());
    assert!(state.context_order.is_empty());
    assert!(state.delegate_context_order.is_empty());
    assert!(state.context_order_by_correlation.is_empty());
    assert!(state.delegate_context_order_by_correlation.is_empty());
    assert!(state.context_correlation_by_key.is_empty());
    assert_eq!(state.stats, InMemoryMessageBusStats::default());
}
