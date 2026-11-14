# 原生资源与同步 SDK 合同（任务 28）

来源：固定快照 ca58c0313360eae7c851fbd7553c1614c7af03e7；直接消费任务 24、25、26 的 pi-system-files-api.md、templates-api.md、sync-api.md 及已落地 handler。SDK 不执行文件业务，不解析或展开 Pi 原文变量。

`GatewayClient` 的 pi_prompt_file_read/replace/delete 对应三个原生文件 POST；读取/替换返回 exists、revision、content，删除返回布尔值。PiPromptFileKind 只允许 system_override/system_append。pi_prompt_templates/template_upsert/template_delete 对应模板 list/upsert/delete，保留 slug、original_slug、expected_revision、原文与元数据摘要。模板删除返回 deleted 对象。PiPromptResourcesScope 显式携带项目和执行主机，原生请求使用 snake_case 字段。

agent_app_prompts_sync 以 camelCase 提交项目、主机与目标列表。agent_app_prompts_sync_operation/events/cancel 复用生命周期操作路径与 GET/GET/DELETE，返回 AgentAppPromptSync，包含原始 AppOperationStatus 和逐目标 status、reason、duplicateSelection；failed、skipped、not_executed 和取消状态不转为成功，处理中空 targets 不表示完成。断线后按已有 operation ID 查询，不重放提交。

传输复用既有 agent_apps_lifecycle_request，携带敏感标记的 x-agent-apps-token 及网关上下文，沿用禁重定向/重试策略；HTTP 错误保留状态码和完整 body，传输与反序列化失败分别返回错误，不回退空列表。错误 Display 不输出响应正文。OMP 模板不提供伪支持。

本轮依用户要求只写代码与必要合同，未执行 Cargo、单元测试、实机测试或其他测试，未领取构建槽位，未启动最终回归，不声明验证通过。后续统一验证应覆盖请求字段及认证、原文/revision/改名往返、409 原始错误、模板空目录、同步部分失败和操作查询/长轮询/取消。UI 和 macOS 人工验收由后续任务负责。
