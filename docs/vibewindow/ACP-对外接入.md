# ACP 对外接入

VibeWindow 提供两个方向不同的 ACP 入口：

- `acp`：VibeWindow 作为 ACP **Client**，连接或启动外部 Agent 的现有桥接程序。
- `shiju acp`：VibeWindow 作为 ACP **Agent**，由外部 ACP Client 通过标准输入输出启动。

本文只描述 `shiju acp` 的对外 Agent 接入。

## 启动与配置

从源码启动：

```bash
cargo run -p vw-cli --bin shiju -- acp
```

安装后启动：

```bash
shiju acp
```

外部 Client 的配置格式各不相同，通用结构如下：

```json
{
  "agents": {
    "shiju": {
      "command": "shiju",
      "args": ["acp"]
    }
  }
}
```

需要独立配置目录时，把全局参数放在子命令前：

```json
{
  "command": "shiju",
  "args": ["--config-dir", "/absolute/path/to/config", "acp"]
}
```

Client 必须把 ACP JSON-RPC 帧写入进程 stdin，并从 stdout 读取响应与通知。进程按一条 stdio 连接提供服务；stdin 关闭后结束并释放活动提示。

## 支持矩阵

| 范围 | 支持情况 | 说明 |
| --- | --- | --- |
| 协议版本 | ACP v1 | `initialize` 返回版本 1 |
| 传输 | stdio | 不提供 TCP、HTTP 或 WebSocket 监听器 |
| 会话 | `session/new`、`session/load` | 加载时回放用户与 Agent 文本历史 |
| 模型选择 | `session/set_config_option`、`session/set_model` | 新建与加载返回模型目录和当前选择 |
| 提示 | `session/prompt`、`session/cancel` | 同一会话只允许一个活动提示 |
| 更新 | `session/update` | 文本、工具、计划和状态更新 |
| 输入内容 | Text、ResourceLink | ResourceLink 只传递引用，不代替 Client 解引用 |
| 权限 | `session/request_permission` | 当前提供单次允许和单次拒绝选项 |
| MCP | stdio、HTTP、SSE | 在新建或加载会话时注入，仅对该会话生效 |
| 身份验证 | 不支持 | `authenticate` 返回 `method not found` |
| 多媒体输入 | 不支持 | Image、Audio、内嵌 Resource 被明确拒绝 |
| 会话枚举等扩展 | 不支持 | 未声明或未知方法返回 `method not found` |

`initialize` 声明 `loadSession: true`，并声明 MCP HTTP 与 SSE 能力；MCP stdio 是协议基础类型。未声明 Image、Audio 或 embedded context。

## 模型选择

`session/new` 和 `session/load` 同时返回 `models` 与 `configOptions`。模型目录来自当前租户和项目已启用的原生模型；模型 ID 使用完整的 `provider/model`，避免同名歧义。`auto` 表示沿用原生自动模型选择，默认选中。

推荐使用 [ACP 会话配置选项](https://agentclientprotocol.com/protocol/v1/session-config-options)：

```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "method": "session/set_config_option",
  "params": {
    "sessionId": "<sessionId>",
    "configId": "model",
    "value": "<provider/model>"
  }
}
```

响应返回完整 `configOptions`，并发送 `config_option_update` 通知。旧 Client 可以使用 `session/set_model`，参数为 `sessionId` 与 `modelId`；两个入口共用选择状态。未知或禁用模型被明确拒绝，不改变原选择。

模型选择只影响该会话的后续提示，活动提示保留启动时的模型。选择按租户、目录和会话持久化，重新连接并加载会话后恢复。已选模型后来被禁用时，加载返回明确错误；重新启用后可恢复加载。

## 使用自带客户端联调

`acp --agent 'shiju acp'` 可以直接启动本地 Agent。命名会话的提示、切模、取消、状态和关闭使用同一个名称；也可向控制命令传递记录 ID。

```bash
acp --agent 'shiju acp' --cwd /absolute/project --model provider/model-a sessions new --name integration
acp --agent 'shiju acp' --cwd /absolute/project -s integration prompt '记住标记 ACP_DEMO，只回复标记'
acp --agent 'shiju acp' --cwd /absolute/project -s integration set model provider/model-b
acp --agent 'shiju acp' --cwd /absolute/project -s integration prompt '刚才的标记是什么？'
acp --agent 'shiju acp' --cwd /absolute/project -s integration status
# 在另一终端取消正在生成的提示：
acp --agent 'shiju acp' --cwd /absolute/project -s integration cancel
acp --agent 'shiju acp' --cwd /absolute/project sessions close integration
```

`--model` 在会话新建、一次性执行与持久化提示时生效；未知模型会直接报错。生成中切换模型不会中断当前输出，后续提示使用新模型。取消会保存已提交的用户提示与收到的部分回复，恢复后可继续对话。

DashScope 的 Chat 接口不接受 `developer` 消息；适配层将可信指令合并到 `system` 通道，保留用户内容的原角色。Responses 接口继续保留 `developer`。接口角色范围见[阿里云 Chat 文档](https://docs.bailian.console.aliyun.com/en/model-studio/qwen-api-via-openai-chat-completions)。

## 会话和安全边界

- `cwd` 必须是存在的绝对目录，并在会话新建和加载时规范化。
- 会话绑定创建它的连接作用域；不同连接不能借用同名会话绕过所有权检查。
- 同一会话一次只运行一个提示。重复提示会返回明确错误。
- 空闲会话可以在同一连接重复加载；活动提示期间加载会返回忙碌错误。
- 历史按时间和原生消息 ID 正序回放，保证同毫秒写入的用户消息先于回复。
- `session/cancel`、stdin EOF 或活动提示 future 被丢弃都会请求取消底层执行。
- MCP 服务按会话连接；重复名称、不支持的传输或启动失败会让会话准备整体失败，不留下部分注册。
- 加载会话先校验归属，再启动请求的 MCP 服务。
- MCP 启动错误只暴露安全的服务名摘要，不回显环境变量值。
- 权限请求仅包含工具调用所需的中性信息；取消、超时、断连和未知选项都按拒绝处理。
- 等待权限回复不阻塞提示结束与取消；原生审批停止等待时，对应的 Client 权限请求会被释放。
- stdout 专用于 ACP JSON-RPC。诊断和日志写入 stderr（以及启用的日志文件），外部 Client 不应把 stderr 当作协议流解析。

## 常见故障

| 现象 | 检查 |
| --- | --- |
| 会话创建失败 | 确认 `cwd` 是存在的绝对目录，且进程有访问权限 |
| 加载返回会话不存在或无权访问 | 使用原连接作用域创建的原生会话 ID，并保持相同 `cwd` |
| 提示被拒绝为已有活动提示 | 等待当前提示停止，或先发送 `session/cancel` |
| MCP 准备失败 | 检查服务名是否重复、命令/URL 和传输类型是否有效；敏感环境变量不会出现在错误文本中 |
| stdout 无法解析 | 确认启动脚本、shell 初始化和包装程序没有向 stdout 打印横幅或日志 |
| `method not found` | 对照支持矩阵；身份验证、会话枚举和未知扩展当前不支持 |
| 进程立即退出 | 检查 Client 是否保持 stdin 打开，并查看 stderr 的配置或运行时错误 |

协议验收测试位于 `crates/vw-acp/src/server/acceptance_tests.rs` 和 `model_tests.rs`，覆盖初始化、新建、提示、更新、权限、取消、加载回放、模型选择和不支持方法。原生后端测试位于 `crates/vw-agent/src/acp/`，验证选择进入运行时、持久化恢复、会话隔离及 MCP 归属检查。

## 开发验证

聚焦服务端和原生后端：

```bash
cargo test -p vw-acp server:: --lib -- --test-threads=1
cargo test -p vw-agent acp:: --lib -- --test-threads=1
cargo clippy --all-targets --all-features -- -D warnings
```

真实进程联调用 debug 构建的客户端和服务端连接本地流式模型端点，隔离 HOME、配置和会话，并保留请求及验证结果：

```bash
cargo build -p vw-cli --bin shiju -p vw-acp --bin acp
python3 scripts/verify_acp_integration.py
```

脚本覆盖两种模型选择协议、命名会话控制、`--model`、多轮历史、空闲及生成中切模、取消延迟、部分回复持久化、进程重启恢复与按名称关闭。`--shiju`、`--acp` 可指定其他目录内的 debug 二进制，`--output` 指定新的证据目录。

macOS 上，现有链接参数下的完整 `vw-acp` 库测试会在预期捕获 panic 的用例中异常中止。可临时覆盖 `RUSTFLAGS` 并使用独立构建目录验证，不修改仓库配置：

```bash
CARGO_TARGET_DIR=/tmp/vw-acp-unwind-target \
RUSTFLAGS='-C target-cpu=apple-m1 -C force-unwind-tables=yes' \
cargo --config 'build.build-dir="/tmp/vw-acp-unwind-build"' \
  test -p vw-acp --lib -- --test-threads=1
```

上述临时参数用于 Apple Silicon；其他架构应省略或调整 `target-cpu`。
