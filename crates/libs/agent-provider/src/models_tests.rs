//! 验证供应商模型初始化入口保持可用。

use super::*;

#[test]
// 取函数指针而不执行初始化，用编译期连线验证公共入口仍存在。
fn models_init_function_is_available() {
    let _ = init as fn();
}
