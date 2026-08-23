//! 验证动态上下文的嵌套作用域取值与外层值恢复。

use super::context;

#[test]
fn provide_scopes_values_and_restores_outer_value() {
    let ctx = context::create::<String>("test-context");

    assert_eq!(ctx.use_value().unwrap_err().to_string(), "No context found for test-context");
    ctx.provide("outer".to_string(), || {
        assert_eq!(ctx.use_value().unwrap().as_str(), "outer");
        ctx.provide("inner".to_string(), || {
            assert_eq!(ctx.use_value().unwrap().as_str(), "inner");
        });
        assert_eq!(ctx.use_value().unwrap().as_str(), "outer");
    });
    assert!(ctx.use_value().is_err());
}
