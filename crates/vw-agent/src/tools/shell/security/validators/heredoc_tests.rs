//! 验证 heredoc 校验器对可展开边界与安全形式的判定。
//! 引用分隔符的安全形式必须与未引用、可变量展开的正文明确区分。

use super::*;
use crate::tools::shell::ast::parse_command;
use crate::tools::shell::security::{SecurityValidator, Severity};

fn findings(command: &str) -> Vec<crate::tools::shell::security::SecurityFinding> {
    HeredocValidator.validate(&parse_command(command))
}

#[test]
fn name_is_stable() {
    assert_eq!(HeredocValidator.name(), "heredoc");
}

#[test]
fn blocks_representative_risky_input() {
    let findings = findings("cat <<EOF");
    assert!(findings.iter().any(|finding| finding.severity == Severity::Block));
}

#[test]
fn allows_representative_plain_input() {
    assert!(findings("cat <<'EOF'").is_empty());
}
