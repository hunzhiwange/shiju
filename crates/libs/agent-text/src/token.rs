//! 提供轻量 token 数估算。
//! 估算只用于预算和展示，不参与安全或计费级别的精确统计。

const CHARS_PER_TOKEN: f64 = 4.0;

/// 执行 estimate 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn estimate(input: &str) -> u64 {
    let len = input.len() as f64;
    let est = (len / CHARS_PER_TOKEN).round();
    if est.is_finite() && est > 0.0 { est as u64 } else { 0 }
}
