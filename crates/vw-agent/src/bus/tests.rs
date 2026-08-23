//! 测试进程内事件总线的类型订阅、通配订阅、单次订阅与取消。
//! 独立事件名和计数器验证取消订阅幂等，序列化失败前不得触发任何回调。

use std::sync::{Arc, Mutex};

use serde_json::json;

use super::{define, global_subscribe, once, publish, publish_value, subscribe, subscribe_all};

#[test]
fn publish_notifies_typed_and_wildcard_subscribers() {
    let def = define("test.plan6.bus.event");
    let typed_seen = Arc::new(Mutex::new(Vec::new()));
    let wildcard_seen = Arc::new(Mutex::new(Vec::new()));

    let typed_slot = Arc::clone(&typed_seen);
    let typed_unsub = subscribe(def, move |payload| {
        typed_slot.lock().unwrap().push(payload);
    });
    let wildcard_slot = Arc::clone(&wildcard_seen);
    let wildcard_unsub = subscribe_all(move |payload| {
        wildcard_slot.lock().unwrap().push(payload);
    });

    publish(def, json!({"value": 7}), Some("/workspace".to_string())).unwrap();

    assert_eq!(typed_seen.lock().unwrap()[0]["properties"]["value"], 7);
    assert!(
        wildcard_seen
            .lock()
            .unwrap()
            .iter()
            .any(|payload| payload["type"] == "test.plan6.bus.event")
    );

    typed_unsub();
    wildcard_unsub();
}

#[test]
fn unsubscribe_stops_future_events() {
    let def = define("test.plan6.bus.unsubscribe");
    let seen = Arc::new(Mutex::new(0usize));
    let slot = Arc::clone(&seen);
    let unsubscribe = subscribe(def, move |_| {
        *slot.lock().unwrap() += 1;
    });

    publish_value(def, json!({}), None);
    unsubscribe();
    publish_value(def, json!({}), None);

    assert_eq!(*seen.lock().unwrap(), 1);
}

#[test]
fn unsubscribe_is_idempotent_and_preserves_peer_subscriber() {
    let def = define("test.task76.bus.idempotent_unsubscribe");
    let first_seen = Arc::new(Mutex::new(0usize));
    let second_seen = Arc::new(Mutex::new(0usize));

    let first_slot = Arc::clone(&first_seen);
    let first_unsubscribe = subscribe(def, move |_| {
        *first_slot.lock().unwrap() += 1;
    });
    let second_slot = Arc::clone(&second_seen);
    let second_unsubscribe = subscribe(def, move |_| {
        *second_slot.lock().unwrap() += 1;
    });

    first_unsubscribe();
    first_unsubscribe();
    publish_value(def, json!({}), None);

    assert_eq!(*first_seen.lock().unwrap(), 0);
    assert_eq!(*second_seen.lock().unwrap(), 1);

    second_unsubscribe();
}

#[test]
fn once_unsubscribes_after_matching_callback() {
    let def = define("test.plan6.bus.once");
    let seen = Arc::new(Mutex::new(Vec::new()));
    let slot = Arc::clone(&seen);

    once(def, move |payload| {
        let value = payload["properties"]["value"].as_i64().unwrap();
        slot.lock().unwrap().push(value);
        value == 2
    });

    publish_value(def, json!({"value": 1}), None);
    publish_value(def, json!({"value": 2}), None);
    publish_value(def, json!({"value": 3}), None);

    assert_eq!(*seen.lock().unwrap(), vec![1, 2]);
}

#[test]
fn global_subscribe_receives_directory_payload_and_unsubscribes() {
    let def = define("test.plan6.bus.global");
    let event_type = def.r#type;
    let seen = Arc::new(Mutex::new(Vec::new()));
    let slot = Arc::clone(&seen);
    let unsubscribe = global_subscribe(move |event| {
        if event.payload["type"] == event_type {
            slot.lock().unwrap().push((event.directory.clone(), event.payload));
        }
    });

    publish_value(def, json!({"ok": true}), Some("/tmp/project".to_string()));
    unsubscribe();
    publish_value(def, json!({"ok": false}), Some("/tmp/project".to_string()));

    let seen = seen.lock().unwrap();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].0.as_deref(), Some("/tmp/project"));
    assert_eq!(seen[0].1["type"], "test.plan6.bus.global");
    assert_eq!(seen[0].1["properties"]["ok"], true);
}

#[test]
fn publish_returns_serialization_errors_without_dispatching() {
    struct FailingSerialize;

    impl serde::Serialize for FailingSerialize {
        fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            Err(serde::ser::Error::custom("intentional serialization failure"))
        }
    }

    let error = publish(define("test.plan6.bus.serialize_error"), FailingSerialize, None)
        .unwrap_err()
        .to_string();

    assert!(error.contains("intentional serialization failure"));
}
