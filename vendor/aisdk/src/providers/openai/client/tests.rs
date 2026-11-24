use super::types::ResponseUsage;

#[test]
fn response_usage_deserializes_when_detail_fields_are_missing() {
    let usage: ResponseUsage = serde_json::from_str(
        r#"{
            "input_tokens": 100,
            "input_tokens_details": {},
            "output_tokens": 50,
            "output_tokens_details": {},
            "total_tokens": 150
        }"#,
    )
    .expect("response usage should deserialize when detail fields are omitted");

    assert_eq!(usage.input_tokens_details.cached_tokens, 0);
    assert_eq!(usage.output_tokens_details.reasoning_tokens, 0);
}
