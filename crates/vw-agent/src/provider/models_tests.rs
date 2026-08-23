//! 验证供应商模型初始化入口保持可用。

use super::*;

#[test]
fn models_init_function_is_available() {
    let _ = init as fn();
}
