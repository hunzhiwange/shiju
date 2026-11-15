use serde_json::json;

#[test]
fn mcp_form_rejects_invalid_primitive_values_and_accepts_decline() {
    fn form() -> question_types::McpForm {
        serde_json::from_value(json!({"server":"表单验收", "schema":{"type":"object","properties":{"age":{"type":"integer","minimum":18},"choice":{"type":"string","enum":["a","b"]}},"required":["age"]}})).unwrap()
    }
    let form = form();
    let answers = |age: &str, choice: &str| {
        vec![vec![age.into()], vec![choice.into()], vec!["accept".into()]]
    };
    assert!(super::valid(&form, &answers("24", "a")));
    for bad in [answers("17", "a"), answers("24", "c"), answers("text", "a")] {
        assert!(!super::valid(&form, &bad));
    }
    assert!(super::valid(&form, &[vec![], vec![], vec!["decline".into()]]));
}
