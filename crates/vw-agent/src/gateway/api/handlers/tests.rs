// 覆盖网关处理器保留的兼容重导出接口。

// 兼容重导出通过类型引用固定，避免路由拆分后旧调用方静默失效。
#[test]
fn reexported_legacy_handlers_are_available() {
    let _ = super::handle_api_config_get;
    let _ = super::handle_api_cron_list;
    let _ = super::handle_api_integrations;
    let _ = super::handle_api_memory_list;
    let _ = super::handle_api_health;
    let _ = super::handle_api_tools;
}
