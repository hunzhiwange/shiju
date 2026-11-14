# 激活与停用 API

来源：固定 cc-switch 快照 ca58c0313360eae7c851fbd7553c1614c7af03e7 的 enable_prompt / togglePrompt；消费普通激活、停用、Pi AGENTS 与 MCode 事务实现。

POST `/agent-apps/prompts/activate`、`/agent-apps/prompts/deactivate`，位于既有鉴权及 TenantScope 路由内。JSON 字段：`project_id`、`target_host_id`、`app_id`、`id`、`revision`（已读目录版本）、`expected_content`（已读原生文件全文；null 表示不存在，空字符串表示空文件）。禁止未知字段，不接受任意路径或系统提示词/项目规则资源。响应为 PromptCatalog；Pi 的 activeId 由原生内容及目录顺序推导，不持久化启用标志。

执行前先核对 prompts_write、租户、项目、执行主机及既有事务归属；物理目标加锁后比较目录版本和原生文件快照。提交前复核权限、主机路径、事务状态及目录 CAS。普通、Pi、MCode 分别使用对应原生服务；冲突返回 409，不自动重试。MCode 超限返回 413，缺条目返回 404，未授权返回 401/403；错误不返回路径、内容或凭据。文件与目录提交失败沿用服务的同步回滚契约，进程崩溃跨库原子性不在合同内。仅 macOS 接入，SDK/UI 由后续任务消费。

本轮按用户要求只实现代码：未执行 Cargo、单元测试、实机测试或回归，未领取 Cargo 槽位。后续统一验证须覆盖成功、无权限零副作用、越 scope、错误 host、文件/目录过期、归属冲突、Pi 重复内容选择与 MCode 回滚。
