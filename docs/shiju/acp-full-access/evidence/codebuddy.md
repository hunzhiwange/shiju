# CodeBuddy 完全访问接入

日期：2026-11-06；本次为固定发行包静态核对与代码接入（E1/E2），没有真实协议或工具执行验收（E3）。用户明确要求不运行测试、不申请账号或受控上游，实机未测不作为阻塞。

## 依赖与来源

批次 17 已完成：`b2b4dd148` 提供当前 host deny 拦截与逐次客户端批准机制。本任务复用公共会话控制、超时和 pending_controls 屏障，不修改安装、认证、模型路由或全局配置。

- [官方 ACP 文档](https://www.codebuddy.ai/docs/cli/acp)：入口 `codebuddy --acp`，会话配置通过 ACP 发现及设置。
- [官方权限模式](https://www.codebuddy.ai/docs/cli/permission-modes)、[CLI 参考](https://www.codebuddy.ai/docs/cli/cli-reference)：`bypassPermissions` 与 `dontAsk` 语义不同，后者拒绝未授权操作。
- [固定 npm 制品](https://registry.npmjs.org/@tencent-ai/codebuddy-code/-/codebuddy-code-2.179.3.tgz)：本次只下载读取 JS，不执行发行包或访问凭据。包版本 2.179.3；`dist/codebuddy-headless.js` SHA256 `abe2d02f74f208b4ea7a0c8621fb7c7cf5822ef6d88af7cf413a6de820a987a4`，与仓库契约 original 哈希一致；`dist-server/codebuddy.js` SHA256 `0d0da98e63e10cf6dbe7005b161fe42388439b8b7ea9ffae8ff22f2d5b45c51c`。

## 源码确认与接入行为

固定包 `getConfigOptions` 公布 select `id=mode`，值为 `default/acceptEdits/plan/auto/dontAsk/bypassPermissions/fullAccess`。`setSessionConfigOption` 校验 mode 并调用 `applyRuntimeSessionPermissionMode`，返回完整 configOptions；底层更新当前 session 的 permissionModeSubject 和 override。普通 ACP 不是托管 job 时不写 job respawn 偏好；host 不注入 job 身份或写用户 settings。

`sync_codebuddy_access` 在 new/load/resume 后、首次 prompt 前，以及现有权限更新同步点执行：只有当前 full_access 且 ApproveAll、并且会话同时公布 default 和 bypassPermissions，才请求后者；关闭回到 default。设置必须返回当前值确认，失败保留 prompt 屏障。未公布控制的普通旧入口保留客户端审批；已知高权限且无可用撤销控制时明确拒绝继续。计划模式不被权限开关隐式替换；活动 turn 的手动 mode 更改先取消并等待。

自动授权选用 bypassPermissions，不选择发行包描述为连危险命令检查也跳过的 fullAccess，不调整 sandbox、模型、思考、agent 或工具配置。设置响应若改变其他配置则停止 prompt。运行时确认不等于原生工具行为已验证，能力显示为 Partial；客户端回调退路为 ClientOnly。

`build_event_client` 对 CodeBuddy 使用逐次审批，剔除 AllowAlways，避免当前授权留下持久规则；取消和无允许选项沿共享拒绝路径。固定包普通选项 ID 是 allow/allow_always/reject；问答与退出计划用 `toolCall._meta["codebuddy.ai/toolName"]` 标识，后者还有 reject_and_exit_plan。`is_workflow_request` 将这些交互独立拒绝自动答复。DenyAll 在 prompt 前阻止原生执行；客户端文件及终端沿当前权限限制。内置终端与客户端 terminal 并非同一执行边界。

启动参数中的 `-y`、bypass 和提升权限的 permission-mode 被明确拒绝，改由可撤销的会话控制处理；正常 `--acp` 路径保持可用。普通/托管及 cli_local/session_model 共用该层，模型来源与平台门控保持原样。供应商企业拒绝、原生沙箱和剩余强制询问不据此宣称已解除。

## 验证记录

用户要求跳过红绿测试与账号实机验收；有效行为测试数量：未运行。静态验证：`cargo fmt --all` 退出 0；`cargo clippy --all-targets --all-features -- -D warnings`（全 features、全 targets）退出 0；`git diff --check` 退出 0。实际动态领取槽位 2，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`；阶段结束已删除自己的 owner 和锁目录，保留缓存。开启→关闭→恢复、工具审批计数、供应商 deny 和跨平台效果未实机验证。
