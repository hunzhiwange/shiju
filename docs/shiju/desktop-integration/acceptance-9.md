# 提示词页入口收口（任务 9）

2026-11-14：仅代码实现交付，未验证。

直接依赖已核对 acceptance-2.md；领域交付见 agent-apps/prompts/desktop-list.md、desktop-editor.md、resources-sdk.md。沿用来源快照 ca58c0313360eae7c851fbd7553c1614c7af03e7。共享 checkout 的提示词领域代码由其他会话交付，本任务不提交其未提交改动。

应用概览卡片新增“提示词”按钮。折叠时通过 AppsCatalogMessage::ToggleDetails 展开详情，复用既有 Event::Open → GatewayClient::agent_app_prompts / agent_app_prompts_current → Event::Reply → list::view 结果链；已展开时直接刷新提示词，不折叠详情。沿用项目、主机、令牌与请求代数校验及 toast 错误处理。按钮遵循现有 busy 门控与统一样式。

Claude Desktop 保留详情中的显式 Claude Code 入口，OpenClaw 保留工作区文件入口；其余十二应用使用领域通用入口。入口可达不代表已经验证十四应用的运行能力。不重做 CRUD，不新增 SDK 或权限。

依用户本轮覆盖要求，未运行任何 Cargo 命令或脚本、单元测试、实机测试或最终回归，未领取槽位，CARGO_TARGET_DIR 未设置。未新增测试，未声称验证通过。后续统一验证应覆盖折叠打开、展开刷新、SDK 成功和错误、过期结果拒绝，以及 macOS 明暗主题与中文渲染。实现完成删除编号任务文件，plan.md 保持只读。
