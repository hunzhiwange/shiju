//! shell 注入类 finding 的筛选辅助函数。
//!
//! 该模块不新增校验规则，只从完整安全报告中抽取注入与提权相关 finding。

use super::{SecurityCategory, SecurityFinding, SecurityReport, Severity};

/// 从安全报告中筛选注入相关 finding。
///
/// 参数：
/// - `report`：完整 shell 安全报告。
///
/// 返回值：类别为注入或提权的 finding 引用列表。
/// 错误处理：该函数不返回错误。
pub fn injection_findings(report: &SecurityReport) -> Vec<&SecurityFinding> {
    report
        .findings
        .iter()
        .filter(|finding| {
            finding.category == SecurityCategory::Injection
                || finding.category == SecurityCategory::PrivilegeEscalation
        })
        .collect()
}

/// 判断报告中是否存在阻断级注入风险。
///
/// 参数：
/// - `report`：完整 shell 安全报告。
///
/// 返回值：存在阻断级注入或提权 finding 时返回 `true`。
/// 错误处理：该函数不返回错误。
pub fn has_blocking_injection(report: &SecurityReport) -> bool {
    injection_findings(report).into_iter().any(|finding| finding.severity == Severity::Block)
}
