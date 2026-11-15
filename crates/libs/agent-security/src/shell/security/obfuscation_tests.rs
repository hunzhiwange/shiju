//! 混淆风险筛选辅助函数的行为测试。

use super::SecurityPipeline;
use super::obfuscation::{has_blocking_obfuscation, obfuscation_findings};

#[test]
fn obfuscation_helpers_filter_obfuscation_findings() {
    let report = SecurityPipeline::new(true).validate_command("echo\u{3000}hello");
    assert!(has_blocking_obfuscation(&report));
    assert!(!obfuscation_findings(&report).is_empty());
}
