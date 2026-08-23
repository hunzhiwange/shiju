//! 验证令牌估算按每四字节向上取整。
//! 空输入和非整除长度共同固定这一近似算法的边界行为。

use super::token::estimate;

#[test]
fn estimate_rounds_by_four_bytes() {
    assert_eq!(estimate(""), 0);
    assert_eq!(estimate("abcd"), 1);
    assert_eq!(estimate("abcdef"), 2);
}
