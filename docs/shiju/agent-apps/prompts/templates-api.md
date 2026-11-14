# Pi 模板网关合同

来源：cc-switch 固定快照 ca58c0313360eae7c851fbd7553c1614c7af03e7，commands/prompt.rs 的 list_pi_prompt_templates、upsert_pi_prompt_template、delete_pi_prompt_template。消费 templates.md 与 template-rename.md 的业务入口。

三个 POST 端点为 `/agent-apps/pi/prompt-templates/list`、`/upsert`、`/delete`（后两项沿用此前缀）。共同 JSON 字段为 project_id、target_host_id、可选 app（默认 pi）；app=omp 返回 422 unsupported 及可展示原因。未知字段拒绝，不接收路径。

list 返回模板数组；upsert 接收 slug、可选 original_slug、expected_revision、content，返回模板；delete 接收 slug、expected_revision，返回 {"deleted":bool}。模板字段为 slug、content、revision、summary；summary 包含 description 与 argument_hint，来自 Rust 元数据解析。原名通过 original_slug 原样传递；变量不展开，revision 不改写，missing 用于创建，陈旧 revision 返回 409。

路由复用网关 TenantScope 和现有 Pi 目录授权，核对项目归属、执行主机、读写授权及 pending 状态，发布时再次核对授权和状态。目录由 PiFiles::open_prompt_templates 定位，逐级 O_NOFOLLOW，仅操作授权 home 下配置的 Pi agent/prompts，读取缺失目录返回空，不把权限错误当缺失；写入准备 0700 目录，业务事务持有共享 Pi 锁。所有响应 no-store，内容上限和 slug 校验由业务包执行。SDK/UI 属于后续任务。

本轮按用户要求未新增或执行测试，未调用 Cargo、未领取槽位、未做实机或最终回归，不声明验证通过。后续统一验证需覆盖真实 router 的成功、无权限、越 scope、错误 host、OMP 拒绝、摘要与原文 roundtrip、改名和 revision 冲突、缺失目录、符号链接及事务恢复。
