//! # SOP 条件评估测试模块
//!
//! 本模块提供标准操作流程（SOP）条件评估功能的单元测试。
//!
//! ## 测试范围
//!
//! - **条件表达式评估**：测试 `evaluate_condition` 函数对不同类型条件表达式的处理
//! - **JSON 路径查询**：验证 JSONPath 表达式的解析和数据提取能力
//! - **数值比较**：测试各种比较运算符（>、>=、<、<=、==、!=）的正确性
//! - **字符串比较**：验证字符串类型的相等和不等比较
//! - **直接比较**：测试不使用 JSONPath 的直接数值比较
//! - **解析函数**：测试操作符解析和路径解析的正确性
//!
//! ## 测试场景
//!
//! 1. **边界情况**：空条件、空载荷、无效 JSON 等
//! 2. **数值比较**：整数和浮点数的大于、小于、等于比较
//! 3. **JSON 路径**：简单路径、嵌套路径、数组索引、缺失键
//! 4. **类型处理**：布尔值、字符串、数值等不同 JSON 类型的处理
//!
//! ## 安全性考虑
//!
//! 测试遵循"默认拒绝"原则：
//! - 缺少载荷时返回 `false`（fail-closed）
//! - 无效 JSON 时返回 `false`
//! - 路径不存在时返回 `false`

use super::*;

/// 测试空条件的匹配行为
///
/// 空条件或仅包含空白字符的条件应该始终匹配成功，
/// 这是一个合理的默认行为，允许某些触发器不需要条件检查。
///
/// # 测试场景
/// - 空字符串条件，有载荷 -> 应该匹配
/// - 纯空白条件，无载荷 -> 应该匹配
#[test]
fn empty_condition_matches() {
    assert!(evaluate_condition("", Some("anything")));
    assert!(evaluate_condition("  ", None));
}

/// 测试缺少载荷时的失败-关闭行为
///
/// 当条件表达式非空但载荷缺失或为空时，应该返回 `false`，
/// 遵循安全优先的 fail-closed 原则，防止误触发。
///
/// # 安全性
/// - 这是关键的安全特性，防止条件检查绕过
/// - 即使条件表达式有效，无载荷也必须拒绝
#[test]
fn missing_payload_fails_closed() {
    assert!(!evaluate_condition("$.value > 85", None));
    assert!(!evaluate_condition("$.value > 85", Some("")));
}

/// 测试 JSONPath 大于（>）比较
///
/// 验证从 JSON 载荷中提取值并进行大于比较的正确性。
#[test]
fn json_path_gt() {
    let payload = r#"{"value": 90}"#;
    assert!(evaluate_condition("$.value > 85", Some(payload)));
    assert!(!evaluate_condition("$.value > 95", Some(payload)));
}

/// 测试 JSONPath 大于等于（>=）比较
///
/// 验证边界值情况：恰好等于阈值时应该匹配成功。
#[test]
fn json_path_gte() {
    let payload = r#"{"value": 85}"#;
    assert!(evaluate_condition("$.value >= 85", Some(payload)));
    assert!(!evaluate_condition("$.value >= 86", Some(payload)));
}

/// 测试 JSONPath 小于（<）比较
///
/// 验证数值小于比较的正确性。
#[test]
fn json_path_lt() {
    let payload = r#"{"temp": 20}"#;
    assert!(evaluate_condition("$.temp < 25", Some(payload)));
    assert!(!evaluate_condition("$.temp < 15", Some(payload)));
}

/// 测试 JSONPath 小于等于（<=）比较
///
/// 验证边界值情况：恰好等于阈值时应该匹配成功。
#[test]
fn json_path_lte() {
    let payload = r#"{"temp": 25}"#;
    assert!(evaluate_condition("$.temp <= 25", Some(payload)));
    assert!(!evaluate_condition("$.temp <= 24", Some(payload)));
}

/// 测试 JSONPath 字符串相等（==）比较
///
/// 验证 JSON 字符串值的精确匹配比较。
/// 注意：字符串比较值需要使用引号包裹。
#[test]
fn json_path_eq() {
    let payload = r#"{"status": "critical"}"#;
    assert!(evaluate_condition(r#"$.status == "critical""#, Some(payload)));
    assert!(!evaluate_condition(r#"$.status == "normal""#, Some(payload)));
}

/// 测试 JSONPath 字符串不等（!=）比较
///
/// 验证 JSON 字符串值的不等比较。
#[test]
fn json_path_neq() {
    let payload = r#"{"status": "ok"}"#;
    assert!(evaluate_condition(r#"$.status != "error""#, Some(payload)));
    assert!(!evaluate_condition(r#"$.status != "ok""#, Some(payload)));
}

/// 测试 JSONPath 数值相等（==）比较
///
/// 验证 JSON 数值的精确匹配比较。
#[test]
fn json_path_numeric_eq() {
    let payload = r#"{"count": 42}"#;
    assert!(evaluate_condition("$.count == 42", Some(payload)));
    assert!(!evaluate_condition("$.count == 43", Some(payload)));
}

/// 测试 JSONPath 嵌套路径访问
///
/// 验证多层嵌套 JSON 对象的路径解析能力。
/// 路径 `.data.sensor.value` 应该能正确提取深层嵌套的值。
#[test]
fn json_nested_path() {
    let payload = r#"{"data": {"sensor": {"value": 87.3}}}"#;
    assert!(evaluate_condition("$.data.sensor.value > 85", Some(payload)));
    assert!(!evaluate_condition("$.data.sensor.value > 90", Some(payload)));
}

/// 测试 JSONPath 缺失键的处理
///
/// 当路径指向的键在 JSON 对象中不存在时，应该返回 `false`。
/// 这是 fail-closed 的安全行为。
#[test]
fn json_path_missing_key() {
    let payload = r#"{"value": 90}"#;
    assert!(!evaluate_condition("$.nonexistent > 0", Some(payload)));
}

/// 测试无效 JSON 载荷的处理
///
/// 当载荷不是有效的 JSON 时，应该返回 `false` 而不是 panic。
/// 这确保了系统的健壮性。
#[test]
fn json_invalid_payload() {
    assert!(!evaluate_condition("$.value > 0", Some("not json")));
}

/// 测试 JSONPath 数组索引访问
///
/// 验证通过索引访问 JSON 数组元素的能力。
/// 索引从 0 开始，`.readings.1` 访问第二个元素。
#[test]
fn json_path_array_index() {
    let payload = r#"{"readings": [10, 20, 30]}"#;
    assert!(evaluate_condition("$.readings.1 == 20", Some(payload)));
}

/// 测试 JSONPath 布尔值的比较
///
/// 注意：当前实现中，JSON 布尔值 `true` 需要作为字符串 `"true"` 进行比较。
/// 这可能是类型转换的实现细节。
#[test]
fn json_path_bool_value() {
    let payload = r#"{"active": true}"#;
    assert!(evaluate_condition(r#"$.active == "true""#, Some(payload)));
}

/// 测试直接数值大于（>）比较
///
/// 不使用 JSONPath，直接对载荷值进行数值比较。
/// 载荷应该是可以解析为数值的字符串。
#[test]
fn direct_gt() {
    assert!(evaluate_condition("> 0", Some("1")));
    assert!(!evaluate_condition("> 0", Some("0")));
    assert!(!evaluate_condition("> 0", Some("-1")));
}

/// 测试直接数值大于等于（>=）比较
///
/// 验证直接比较的边界值处理。
#[test]
fn direct_gte() {
    assert!(evaluate_condition(">= 5", Some("5")));
    assert!(evaluate_condition(">= 5", Some("6")));
    assert!(!evaluate_condition(">= 5", Some("4")));
}

/// 测试直接数值小于（<）比较
///
/// 验证直接数值小于比较的正确性。
#[test]
fn direct_lt() {
    assert!(evaluate_condition("< 100", Some("50")));
    assert!(!evaluate_condition("< 100", Some("100")));
}

/// 测试直接数值相等（==）比较
///
/// 验证直接数值精确匹配。
#[test]
fn direct_eq() {
    assert!(evaluate_condition("== 42", Some("42")));
    assert!(!evaluate_condition("== 42", Some("43")));
}

/// 测试直接数值不等（!=）比较
///
/// 验证直接数值不等比较。
#[test]
fn direct_neq() {
    assert!(evaluate_condition("!= 0", Some("1")));
    assert!(!evaluate_condition("!= 0", Some("0")));
}

/// 测试直接比较时非数值载荷的处理
///
/// 当载荷无法解析为数值时，应该返回 `false` 而不是 panic。
#[test]
fn direct_non_numeric_payload() {
    assert!(!evaluate_condition("> 0", Some("not a number")));
}

/// 测试直接浮点数比较
///
/// 验证浮点数的精确比较能力。
/// 注意：浮点数比较可能存在精度问题，但这里测试的是精确值比较。
#[test]
fn direct_float_comparison() {
    assert!(evaluate_condition("> 3.14", Some("3.15")));
    assert!(!evaluate_condition("> 3.14", Some("3.13")));
}

#[test]
fn direct_lte_accepts_equal_and_lower_payloads() {
    assert!(evaluate_condition("<= 10", Some("10")));
    assert!(evaluate_condition(" <= 10 ", Some("9.5")));
    assert!(!evaluate_condition("<= 10", Some("10.5")));
}

/// 测试 parse_op_value 函数的基本解析能力
///
/// 验证能正确解析操作符和比较值。
/// 输入 `"> 42"` 应该返回 `(Op::Gt, "42")`。
#[test]
fn parse_op_value_basic() {
    let (op, val) = parse_op_value("> 42").unwrap();
    assert_eq!(op, Op::Gt);
    assert_eq!(val, "42");
}

/// 测试 parse_op_value 能正确区分 >= 和 >
///
/// 确保解析器能正确识别 >= 操作符，而不是误解析为 > 后跟 =。
/// 这是解析器的边界情况测试。
#[test]
fn parse_op_value_gte_not_gt() {
    let (op, val) = parse_op_value(">= 10").unwrap();
    assert_eq!(op, Op::Gte);
    assert_eq!(val, "10");
}

/// 测试 parse_op_value 对缺失值的处理
///
/// 当操作符后没有比较值时，应该返回 `None`。
/// 验证解析器的健壮性。
#[test]
fn parse_op_value_no_value() {
    assert!(parse_op_value(">").is_none());
    assert!(parse_op_value("> ").is_none());
}

/// 测试 parse_path_op_value 函数的基本路径解析
///
/// 验证能正确解析简单路径、操作符和比较值。
/// 输入 `".value > 85"` 应该返回路径 `["value"]`、操作符 `Op::Gt`、值 `"85"`。
#[test]
fn parse_path_op_value_basic() {
    let (segments, op, val) = parse_path_op_value(".value > 85").unwrap();
    assert_eq!(segments, vec!["value"]);
    assert_eq!(op, Op::Gt);
    assert_eq!(val, "85");
}

/// 测试 parse_path_op_value 对嵌套路径的解析
///
/// 验证能正确解析多段嵌套路径。
/// 输入 `".data.temp >= 100"` 应该返回路径 `["data", "temp"]`。
#[test]
fn parse_path_op_value_nested() {
    let (segments, op, val) = parse_path_op_value(".data.temp >= 100").unwrap();
    assert_eq!(segments, vec!["data", "temp"]);
    assert_eq!(op, Op::Gte);
    assert_eq!(val, "100");
}

/// 测试 parse_path_op_value 对字符串比较值的解析
///
/// 验证能正确解析包含引号的字符串比较值。
/// 字符串值应该保留引号，用于后续的字符串比较。
#[test]
fn parse_path_op_value_string_comparand() {
    let (segments, op, val) = parse_path_op_value(r#".status == "critical""#).unwrap();
    assert_eq!(segments, vec!["status"]);
    assert_eq!(op, Op::Eq);
    assert_eq!(val, r#""critical""#);
}

/// 测试 resolve_json_path 函数的简单路径解析
///
/// 验证能从 JSON 对象中提取单层键值。
#[test]
fn resolve_path_simple() {
    let json: Value = serde_json::from_str(r#"{"a": 1}"#).unwrap();
    let v = resolve_json_path(&json, &["a"]).unwrap();
    assert_eq!(v, &Value::Number(1.into()));
}

/// 测试 resolve_json_path 函数的嵌套路径解析
///
/// 验证能从嵌套 JSON 对象中提取深层值。
/// 路径 `["a", "b", "c"]` 应该能正确解析多层嵌套。
#[test]
fn resolve_path_nested() {
    let json: Value = serde_json::from_str(r#"{"a": {"b": {"c": 42}}}"#).unwrap();
    let v = resolve_json_path(&json, &["a", "b", "c"]).unwrap();
    assert_eq!(v, &Value::Number(42.into()));
}

/// 测试 resolve_json_path 函数对缺失键的处理
///
/// 当路径中的某个键不存在时，应该返回 `None`。
/// 这是 fail-safe 的行为。
#[test]
fn resolve_path_missing() {
    let json: Value = serde_json::from_str(r#"{"a": 1}"#).unwrap();
    assert!(resolve_json_path(&json, &["b"]).is_none());
}

#[test]
fn json_path_invalid_expression_fails_closed() {
    let payload = r#"{"value": 90}"#;
    assert!(!evaluate_condition("$.value", Some(payload)));
    assert!(!evaluate_condition("$. > 0", Some(payload)));
}

#[test]
fn json_path_string_ordering_comparisons() {
    let payload = r#"{"status": "beta"}"#;
    assert!(evaluate_condition("$.status > alpha", Some(payload)));
    assert!(evaluate_condition("$.status < gamma", Some(payload)));
    assert!(evaluate_condition("$.status >= beta", Some(payload)));
    assert!(evaluate_condition("$.status <= beta", Some(payload)));
}

#[test]
fn json_path_numeric_string_uses_numeric_comparison() {
    let payload = r#"{"value": "42.5"}"#;
    assert!(evaluate_condition("$.value >= 42.5", Some(payload)));
    assert!(!evaluate_condition("$.value < 40", Some(payload)));
}

#[test]
fn direct_condition_invalid_comparand_fails_closed() {
    assert!(!evaluate_condition("> nope", Some("1")));
}

#[test]
fn parse_op_value_unknown_operator() {
    assert!(parse_op_value("= 10").is_none());
    assert!(parse_op_value("value").is_none());
}

#[test]
fn parse_path_op_value_rejects_missing_path_or_operator() {
    assert!(parse_path_op_value(" >= 10").is_none());
    assert!(parse_path_op_value(".value").is_none());
}

#[test]
fn resolve_path_array_index_out_of_bounds() {
    let json: Value = serde_json::from_str(r#"{"readings": [10, 20]}"#).unwrap();
    assert!(resolve_json_path(&json, &["readings", "3"]).is_none());
}

#[test]
fn value_as_string_handles_null_and_non_string_values() {
    assert_eq!(value_as_string(&Value::Null), "");
    assert_eq!(value_as_string(&Value::Number(7.into())), "7");
}
