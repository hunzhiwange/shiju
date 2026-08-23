//! 验证占位导出项保持可调用且常量值稳定。

use super::scrap::{BAR, FOO, dummy_function, random_helper};

#[test]
fn dummy_exports_are_callable() {
    dummy_function();
    let _ = random_helper();
    assert_eq!(FOO, "42");
    assert_eq!(BAR, 123);
}
