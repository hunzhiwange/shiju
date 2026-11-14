# MCP 页入口收口（任务 7）

2026-11-14：实现已交付，未验证。按本轮用户约束未运行 Cargo、单元测试、实机测试或最终回归；未领取槽位，未设置 Cargo 缓存路径。编号任务卡按要求删除。

直接依赖：已读取 acceptance-2.md，领域导航通过 AgentConfigTab::Mcp 到达既有页面。母任务 11 的真实产物已存在：mcp_management_read.rs 的 GatewayClient::mcp_management_catalog、桌面 mcp_management_list 的上下文/代次保护，以及 system_settings_mcp 的目录、绑定和工具详情展示。已核对领域任务 32、38 的正式证据；其来源版本为 ca58c0313360eae7c851fbd7553c1614c7af03e7，历史验证不作为本次通过证据。

缺口：导航及页面刷新发送 McpMessage::Refresh，但该分支只刷新全局配置和运行时 inspect，受管目录必须再单独点击刷新。

修改：Refresh 分支并行返回既有 refresh_all_task 与 CatalogMessage::Refresh 的任务，接通导航/刷新 → 目录消息 → mcp_management_catalog → CatalogMessage::Loaded → 既有目录与能力展示。复用当前网关、主机、租户、项目及凭据代次检查；无主机授权时沿用明确错误；旧上下文及旧代次响应仍由原处理器拒绝。未新增领域 API 或配置写入。

十四应用能力仍按服务端 McpSupport 的 Supported、Unsupported、NotVerified 显示，不把未验证改写为不支持。该入口只增加目录读取，工具发现、原生导入与分发仍需用户显式操作。

未来统一验证：检查导航和刷新加载 SDK 目录结果、缺少授权/服务端拒绝的错误展示、切换主机后迟到响应隔离，以及 macOS 明暗主题实际渲染。本次没有新增或执行测试，不声称验证通过。
