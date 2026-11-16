use serde_json::json;

#[test]
fn thread_query_args_preserves_allowed_values() {
    let args = json!({"thread_id": "child", "max_chars": 512});
    let object = super::query_args(&args, &["thread_id", "max_chars"]).expect("allowed fields");
    assert_eq!(object, args.as_object().expect("object"));
    assert!(super::query_args(&json!({}), &[]).expect("empty object").is_empty());
}

#[test]
fn thread_query_args_rejects_non_objects_and_unknown_fields() {
    for args in [json!(null), json!([]), json!("child"), json!(true), json!(1)] {
        let error = super::query_args(&args, &[]).expect_err("object required");
        assert_eq!(error.to_string(), "thread arguments must be an object");
    }
    let args = json!({"thread_id": "child", "project_id": "other"});
    let error = super::query_args(&args, &["thread_id"]).expect_err("unknown field");
    assert_eq!(error.to_string(), "unsupported thread query field 'project_id'");
}
