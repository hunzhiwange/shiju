/// 解析并验证温度参数
///
/// 温度参数用于控制 AI 模型输出的随机性，取值范围为 0.0 到 2.0：
/// - 0.0：最确定性，输出最一致
/// - 1.0：平衡的随机性
/// - 2.0：最高随机性，输出最多样化
///
/// # 参数
///
/// * `s` - 温度值的字符串表示
///
/// # 返回值
///
/// - `Ok(f64)` - 解析成功的温度值（0.0 到 2.0 之间）
/// - `Err(String)` - 解析失败或值超出范围时的错误信息
///
/// # 示例
///
/// ```ignore
/// let temp = parse_temperature("0.7").unwrap(); // 返回 0.7
/// let err = parse_temperature("3.0"); // 返回错误，超出范围
/// ```
pub(crate) fn parse_temperature(s: &str) -> std::result::Result<f64, String> {
    let t: f64 = s.parse().map_err(|e| format!("{e}"))?;
    if !(0.0..=2.0).contains(&t) {
        return Err("temperature must be between 0.0 and 2.0".to_string());
    }
    Ok(t)
}
