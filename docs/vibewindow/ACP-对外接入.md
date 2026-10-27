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
| 提示 | `session/prompt`、`session/cancel` | 同一会话只允许一个活动提示 |
| 更新 | `session/update` | 文本、工具、计划和状态更新 |
| 输入内容 | Text、ResourceLink | ResourceLink 只传递引用，不代替 Client 解引用 |
| 权限 | `session/request_permission` | 当前提供单次允许和单次拒绝选项 |
| MCP | stdio、HTTP、SSE | 在新建或加载会话时注入，仅对该会话生效 |
| 身份验证 | 不支持 | `authenticate` 返回 `method not found` |
| 多媒体输入 | 不支持 | Image、Audio、内嵌 Resource 被明确拒绝 |
| 会话枚举等扩展 | 不支持 | 未声明或未知方法返回 `method not found` |

`initialize` 声明 `loadSession: true`，并声明 MCP HTTP 与 SSE 能力；MCP stdio 是协议基础类型。未声明 Image、Audio 或 embedded context。

## 会话和安全边界

- `cwd` 必须是存在的绝对目录，并在会话新建和加载时规范化。
- 会话绑定创建它的连接作用域；不同连接不能借用同名会话绕过所有权检查。
- 同一会话一次只运行一个提示。重复提示会返回明确错误。
- `session/cancel`、stdin EOF 或活动提示 future 被丢弃都会请求取消底层执行。
- MCP 服务按会话连接；重复名称、不支持的传输或启动失败会让会话准备整体失败，不留下部分注册。
- MCP 启动错误只暴露安全的服务名摘要，不回显环境变量值。
- 权限请求仅包含工具调用所需的中性信息；取消、超时、断连和未知选项都按拒绝处理。
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

协议验收测试位于 `crates/vw-acp/src/server/acceptance_tests.rs`，覆盖初始化、新建、提示、更新、权限、取消、加载回放和不支持方法。
