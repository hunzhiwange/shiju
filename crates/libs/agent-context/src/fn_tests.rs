//! 验证函数包装器先解析输入再执行，并支持绕过解析的强制调用。
//! 解析失败不得调用内部函数，强制入口则只接受已验证类型。

use super::r#fn::fn_wrap;

#[test]
fn call_parses_before_running_and_force_skips_parse() {
    let wrapped = fn_wrap(|input: &str| input.parse::<u32>(), |value| value + 1);

    assert_eq!(wrapped.call("41"), Ok(42));
    assert!(wrapped.call("bad").is_err());
    assert_eq!(wrapped.force(9), 10);
}
