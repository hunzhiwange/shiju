//! LLM 请求选项合并工具。
//!
//! 本模块提供 JSON 值的深度合并，用于把模型默认选项、 provider 选项和运行时覆盖项
//! 组合成最终请求参数。合并规则保持直白：对象递归合并，非对象直接替换。

use serde_json::Value;

/// 将 `source` 深度合并到 `target`。
///
/// 当两侧都是对象时逐键递归合并；其他类型直接用 `source` 克隆值替换 `target`。
/// 函数不返回错误，调用方应在合并前完成选项来源的合法性校验。
pub fn merge_deep_value(target: &mut Value, source: &Value) {
    match (target, source) {
        (Value::Object(t), Value::Object(s)) => {
            for (k, sv) in s {
                match t.get_mut(k) {
                    Some(tv) => merge_deep_value(tv, sv),
                    None => {
                        t.insert(k.clone(), sv.clone());
                    }
                }
            }
        }
        (t, s) => {
            *t = s.clone();
        }
    }
}
#[cfg(test)]
#[path = "options_tests.rs"]
mod options_tests;
