# 提示词资源协议

子任务 2 实现 `vw_api_types::agent_apps` 的提示词 DTO，沿用现有 lib 导出链。
来源为子任务 1 的 coverage.md 固定提交及源仓 `src-tauri/src/prompt.rs`、`commands/prompt.rs`。来源的字符串应用参数替换为既有封闭 AppId；不透传 Tauri 命令。

- PromptResource 区分应用提示词、项目规则、识矩系统提示词、Pi 原生资源。Pi 只提供 system、append_system、template 三种选择；模板 slug 由模板所有者校验，不是路径。
- 项目规则与识矩系统提示词只作分类导航，由既有所有者处理。validate_application_access 拒绝本领域编辑这些资源，以及 ClaudeDesktop 和 OpenClaw。OpenClaw 投影仍由同步任务负责。
- 请求使用 deny_unknown_fields，未知应用与资源种类反序列化失败，不能携带任意 path、command 或客户端租户身份。主机选择及租户授权继承认证网关上下文，不增设第二份身份或调度模型。错误复用 AgentAppsErrorCode，异步操作复用 AppOperationStatus。
- PromptEntry 保留来源内容、时间和启用状态。PromptRevision 是所有者签发的不透明内容版本。快照 None 表示缺失，空字符串表示存在空文件；所有者必须给予不同版本。更新、激活必须在锁内重新核对版本、归属与授权。
- PromptTargetPreview 仅返回显示路径，任何写入都重新解析授权目标，不能把显示路径作为写入参数。writable 是展示信息，不能授予权限。
- Upsert 不接收 enabled：普通保存不能隐式激活。分享载荷不携带主机、路径、命令或激活授权；activationSuggested 仅供展示。导入 validate 要求 confirmed 且拒绝不支持目标，activation 由用户独立选择，KeepDisabled 表示停用副本。
- 当前只交付共享协议，无 API 路由、SDK 或 UI 改动；后续对应任务接入。业务端仍须验证名称、ID、slug、内容大小、revision、请求确认和主机权限，DTO 不代替授权。

按本轮用户要求未运行 Cargo、单元测试、实机测试或其他测试，未启动最终回归，未领取 Cargo 槽位。后续统一验证需覆盖未知 kind/AppId、额外 path/command、跨所有者拒绝、分享确认与独立激活、版本冲突，以及序列化往返；本轮不声明这些场景通过。
