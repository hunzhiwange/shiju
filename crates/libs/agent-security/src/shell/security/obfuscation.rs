//! shell 混淆类 finding 的筛选辅助函数。
//!
//! 该模块用于把完整安全报告中的混淆风险独立取出，便于 UI 或调用方给出更具体的风险提示。

use super::{SecurityCategory, SecurityFinding, SecurityReport, Severity};

/// 从安全报告中筛选混淆相关 finding。
///
/// 参数：
/// - `report`：完整 shell 安全报告。
///
/// 返回值：类别为混淆的 finding 引用列表。
/// 错误处理：该函数不返回错误。
pub fn obfuscation_findings(report: &SecurityReport) -> Vec<&SecurityFinding> {
    report
        .findings
        .iter()
        .filter(|finding| finding.category == SecurityCategory::Obfuscation)
        .collect()
}

/// 判断报告中是否存在阻断级混淆风险。
///
/// 参数：
/// - `report`：完整 shell 安全报告。
///
/// 返回值：存在阻断级混淆 finding 时返回 `true`。
/// 错误处理：该函数不返回错误。
pub fn has_blocking_obfuscation(report: &SecurityReport) -> bool {
    obfuscation_findings(report).into_iter().any(|finding| finding.severity == Severity::Block)
}
