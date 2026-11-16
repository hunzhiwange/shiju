use super::validate;
use serde_json::json;

#[test]
fn codex_plugin_compat_t24_validates_primitive_constraints() {
    let schema = json!({"properties":{
        "name":{"type":"string","minLength":2,"maxLength":4},
        "enabled":{"type":"boolean"},"choice":{"type":"string","enum":["a","b"]},
        "ratio":{"type":"number","minimum":0,"maximum":1}
    },"required":["name"]});
    assert!(
        validate(
            &schema,
            &json!({"name":"中文","enabled":true,"choice":"a","ratio":0.5})
        )
        .is_ok()
    );
    for invalid in [
        json!({"name":"x"}),
        json!({"name":"longer"}),
        json!({"name":"ok","enabled":"true"}),
        json!({"name":"ok","choice":"c"}),
        json!({"name":"ok","ratio":2}),
    ] {
        assert!(validate(&schema, &invalid).is_err());
    }
}

#[test]
fn codex_plugin_compat_t24_validates_isolation_constraints() {
    let schema =
        json!({"type":"object","properties":{"age":{"type":"integer"}},"required":["age"]});
    assert!(validate(&schema, &json!({"age":24,"foreign":true})).is_err());
    assert!(validate(&schema, &json!({})).is_err());
    assert!(validate(&schema, &json!({"age":"24"})).is_err());
}
