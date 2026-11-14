# 41 MCP 深链预览

实现：macOS 启动队列按 ID 交接 MCP 链接，支持 shiju/ccswitch v1 import 的 apps/config/enabled。解析 Base64 mcpServers，限制长度、重复参数及未知字段；不下载远程内容。

继续预览仅生成草稿，保存需再次确认。调用当前仓库 vw-gateway-client 的 mcp_management_create/McpCreateRequest，沿用授权上下文和 generation 防止陈旧响应。按序号显示每项格式失败、冲突、保存成功或未确认成功；不会启用、分发或执行链接命令。apps/enabled 只作为来源声明，不转换成权限。

直接依赖：任务7的 MCP 导入确认/逐项结果与 SDK 已存在；任务38的 deep_links pending/acknowledge 已存在。母任务11/18消费现有工作区源码契约，无外部 SDK 新增。

按本轮用户覆盖要求，未运行 Cargo、脚本验证、单元测试、实机或最终回归；未领取槽位，CARGO_TARGET_DIR 未设置。不声称验证通过。后续统一验证应覆盖两种 scheme、拒绝保留/移除队列、批量混合成功与失败、重复参数、作用域变化及暗色实际渲染。
