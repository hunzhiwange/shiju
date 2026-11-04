# Kiro CLI 3.0.0 模型来源调查

2026-11-05，在 Darwin arm64 使用官方 standalone universal ZIP，未执行安装器。

- 下载：<https://prod.download.cli.kiro.dev/stable/3.0.0/kirocli-universal-darwin.zip>
- SHA256：`0a8f067c4fe6a23a231aa3a18e3e4332120ef9ed3d2d7b6eea907beb8b4cdd8b`
- `kiro-cli --version`：退出0，`kiro-cli 3.0.0`。
- BUILD_HASH：`21067db0a7cc0ec7a0c933102e36e0a70084a013`。

## 已验证的原生入口

`KIRO_HOME` 隔离设置、登录和会话；`KIRO_RUNTIME_DIR` 隔离随包引擎的解压目录。使用白名单环境、空工作目录及 macOS sandbox-exec，只允许 localhost 出站连接。没有真实账号、生产模型调用、代理劫持或 TLS 修改。

实际运行 `kiro-cli acp --agent-engine v3 --auth-method client`，通过 `_kiro/auth/getAccessToken` 返回合成令牌；也运行 `--auth-method cli`，通过 `KIRO_API_KEY` 提供合成令牌。两者都成功初始化、新建会话并发出模型请求。

随包实现接受 `KIRO_KAS_ENDPOINT`、`KIRO_KAS_CONTROL_PLANE_ENDPOINT` 和 `KIRO_KAS_REGION`。回环控制面返回自定义 `offline-model`，实际收到 `KiroRuntimeService.GenerateAssistantResponse`，其 `conversationState.currentMessage.userInputMessage.modelId` 为该模型，包含 Kiro 原生工具定义。返回 Amazon eventstream 的 assistantResponseEvent 和 metadataEvent(stopReason=end_turn) 后，原 CLI 输出 `Offline Kiro reply` 并正常结束一轮。完整终止事件实验只有1次模型请求。

`providers/list` 和 `providers/set` 实际返回 -32601；随包包含 ACP 通用 schema 不代表 Kiro 实现这些方法。

## 现有 Responses 路径

随包 V3 引擎已有 `POST /v1/responses` 实现。其控制面 GetFeatureConfiguration 使用 SHA256(`kiro-feature-config-key-salt-3e9b1d7a` + `create_response_api`) 作为 wire key，值为 `create_response` 时选择该路径。对固定包使用这个响应配置，原 CLI 实际发送标准 Responses 的 model/input/tools/stream，另附 origin 和 Kiro 元数据。模型 `offline-model`、合成 Bearer 令牌和 fixture SSE 响应成功完成一轮。

这是一项固定版本的内部接口证据，不能推断其它版本兼容，也不等于厂商承诺通用 Provider API。接入必须保留版本与能力边界，拒绝未知控制面操作，不模拟厂商托管工具或授予企业权限。

## 生产链验收

原 CLI 经本仓库 `do_stream_request_acp`、driver profile 和模型中继完成了以下验收，未使用假 CLI：

| 来源 | 实际模型请求 | 模型 | 原生辅助请求 | 生命周期 |
| --- | ---: | --- | ---: | --- |
| session_model | 11 | primary 8，switched 3 | 3 | 新建、第二轮、取消、重启、恢复、切模均通过 |
| cli_local | 10 | 始终为原生 primary | 3 | 新建、第二轮、取消、重启、恢复均通过；客户端切模不覆盖原生模型 |

两种来源的错误请求均为0，实际工具结果最多为2份。辅助请求来自 Kiro 内置 `invoke_sub_agent` 调用的 `general-task-execution`，读取两个独立 fixture 文件，避免 Kiro 对重复读取相同文件的去重影响验收。客户端来源的原生配置故意含冲突模型和错误令牌；实际请求只使用绑定模型和中继能力。

- [客户端来源报告](kiro/session-model.json)：真实脚本退出0，命中1项验收测试。
- [原生来源报告](kiro/cli-local.json)：真实脚本退出0，命中1项验收测试。
- 聚焦命令：`cargo test -p vw-agent --lib --all-features acp_model_source_kiro -- --nocapture`，4项通过。红灯为原 driver 拒绝配置导致2项行为失败，退出101；实现后转绿。
- 测试覆盖独立子进程读取配置、凭据不落盘/argv、历史保持与令牌轮换、冲突拒绝、HTTP鉴权、绑定模型目录、未知控制面操作拒绝、禁用模型拒绝和撤销。
- 原 CLI 的独立缺失凭据实验返回认证错误，模型请求0；未启动交互登录或读取真实用户凭据。
- `python3 -m unittest discover -s scripts -p '*acp_model_source*test*.py'`：28项通过。

源码入口为 `model_source/drivers/kiro.rs::configure`、`model_relay/kiro.rs` 和 `model_relay/server.rs`。控制面与 Responses 请求共享身份、过期与撤销校验；本地控制面另有8个并发许可，原有上游模型并发4不变。控制面不执行厂商托管工具，不授予账户权限。模型请求继续使用已有低层 ProviderRelayBackend，不进入会话工具循环。

本轮仅开放已支持的 OpenAI Chat/Responses 上游；其他协议、不可转换内容和未知控制面操作明确报错。Linux/Windows及其它 CLI 版本未进行真实运行验收。任务33负责能力发布，本任务不修改 registry 或安装器。

Cargo 红绿阶段使用共享槽位4，集成和最终检查使用动态领取的共享槽位3；同时设置 CARGO_TARGET_DIR 和 CARGO_BUILD_BUILD_DIR 到同一槽位。完整绝对路径和过程日志保留于本地 `target/acp-model-source-evidence/kiro/`，仓库报告仅省略主机路径，不包含令牌、原始请求载荷或随包源码。

共享中继回归及全仓 Clippy：待回归。最终 binding/Responses 检查均退出101、有效测试0，因其它计划的 `session/agent_binding_tests.rs` 仍引用迁移中的 `resolve_full_access_scoped` 和 `AgentDefaults.full_access`。全仓 `cargo clippy --all-targets --all-features -- -D warnings` 已执行，退出101，具体为 `tools/decision/tests.rs:470` 对返回 `()` 的测试调用 `.await`（E0277）。这些文件未由本任务修改。权限重构结束后统一重跑；不将上述失败当作已通过，也不影响此前取得的两种真实 CLI 请求证据。

## 官方参考

- [CLI commands](https://kiro.dev/docs/reference/cli-commands/)
- [Settings](https://kiro.dev/docs/reference/settings/)
- [Agent harness](https://kiro.dev/docs/cli/agent-harness/)
- [Authentication](https://kiro.dev/docs/getting-started/authentication/)
