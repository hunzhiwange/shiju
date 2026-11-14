# 长任务进度与取消（任务 20）

实现：沿用应用生命周期真实 SDK 调用链（`apps_lifecycle::execute`），读取 `AppOperationStatus` 的 phase/status；事件轮询使用 `agent_apps_operation_events_authorized`，取消使用 `agent_apps_operation_cancel_authorized`。类型合同为本仓库 `vw-api-types::agent_apps::actions`，未引入新协议或本地百分比。取消请求不会将任务标为已取消，最终结果以 SDK 回读为准。

补齐取消请求状态：请求期间禁止重复取消，继续显示真实服务端阶段；终态清除请求标记，失败释放请求标记以允许人工重试。上下文、主机和代次检查沿用现有实现，过期响应不能覆盖当前任务。已有技能任务使用独立 SkillJobResponse 合同，不混用生命周期状态。

新增独立行为测试源码 `desktop_integration_20_tests.rs`，由生命周期组件注册，覆盖取消等待期间状态、目标不匹配拒绝、完成与取消竞争及状态清理。复用现有生命周期 SDK HTTP 测试作为未来验证入口。

按本轮用户明确要求，未运行 Cargo、单元测试、实机测试或最终回归；未领取槽位，无 CARGO_TARGET_DIR，不声称验证通过。后续统一验证需运行本任务和现有生命周期定向测试，并人工验收真实长任务进度、取消及错误恢复；14 应用及 macOS 渲染均未在本轮验证。
