use super::snippets::*;
use serde_json::json;
#[test]
fn cc04_snippets_dto_rejects_unknown_inputs_and_conceals_payloads() {
    let request = SnippetExtract {
        target_host_id: "host".into(),
        source: SnippetSource::Draft { settings: json!({"secret":"fixture-key"}) },
    };
    let value = serde_json::to_value(&request).unwrap();
    assert_eq!(value["source"]["kind"], "draft");
    let _: SnippetExtract = serde_json::from_value(value).unwrap();
    assert!(!format!("{request:?}").contains("fixture-key"));
    for value in [
        json!({"targetHostId":"host","content":"{}","path":"/tmp/config"}),
        json!({"targetHostId":"host","content":null}),
        json!({"content":"{}"}),
    ] {
        assert!(serde_json::from_value::<SnippetWrite>(value).is_err());
    }
    assert!(
        serde_json::from_value::<SnippetExtract>(
            json!({"targetHostId":"host","source":{"kind":"saved","providerId":"p","settings":{}}})
        )
        .is_err()
    );
    let read: ConfigSnippet = serde_json::from_value(json!({"content":null})).unwrap();
    assert!(read.content.is_none());
}
