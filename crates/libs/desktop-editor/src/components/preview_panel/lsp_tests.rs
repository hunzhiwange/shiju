//! LSP 界面适配测试模块的连线哨兵。
//! 确保语言服务相关的桌面端测试入口持续参与编译。

#[test]
fn lsp_test_module_is_linked() {
    let name = "lsp";
    assert_eq!(name.len(), 3);
}
