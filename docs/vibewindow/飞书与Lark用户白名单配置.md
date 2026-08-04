# 飞书与 Lark 用户白名单配置

VibeWindow 使用 `allowed_users` 限制哪些飞书或 Lark 用户可以向机器人发送消息。
白名单属于运行配置，不应写死在 Rust 源码、数据库迁移或默认配置中。

## 日志含义

出现下面的日志时，WebSocket 连接和事件解析均已成功，但发送者未通过白名单检查：

```text
Lark WS: ignoring ou_example (not in allowed_users)
```

其中 `ou_example` 表示事件携带的发送者 `open_id`。运行时会把它与当前通道实例
的 `allowed_users` 逐项精确比较；配置显示名、用户名或备注名不会匹配。

## 获取 open_id

可以从以下任一来源取得发送者 `open_id`：

- 上述拒绝日志中 `ignoring` 后面的 `ou_...` 值。
- 飞书或 Lark 的 `im.message.receive_v1` 事件中
  `event.sender.sender_id.open_id` 字段。

`open_id` 与应用和租户有关。切换应用或租户后，应重新确认实际事件中的值。

## 单实例配置

飞书中国区端点使用 `channels.feishu`：

```toml
[channels.feishu]
app_id = "cli_example"
app_secret = "<app-secret>"
allowed_users = ["ou_example_user"]
receive_mode = "websocket"
```

Lark 国际版端点使用 `channels.lark`：

```toml
[channels.lark]
app_id = "cli_example"
app_secret = "<app-secret>"
allowed_users = ["ou_example_user"]
receive_mode = "websocket"
```

允许多个用户时，逐项填写：

```toml
allowed_users = [
  "ou_example_user_a",
  "ou_example_user_b",
]
```

桌面端可在“系统设置 → 通道 → Feishu/Lark → 允许用户”中填写；每行一个
`open_id`，也支持使用逗号分隔。

## 多实例配置

每个实例维护独立白名单，用户 ID 必须写入接收该事件的实例：

```toml
[[channels.feishu_instances]]
id = "support"
app_id = "cli_support_example"
app_secret = "<app-secret>"
allowed_users = ["ou_example_support_user"]
receive_mode = "websocket"
```

不要只修改旧的 `channels.feishu.allowed_users`，却让实际消息由
`channels.feishu_instances` 中的命名实例接收。

## 通配符

以下配置允许所有用户：

```toml
allowed_users = ["*"]
```

这会扩大机器人访问范围。除明确面向所有用户的场景外，应优先配置具体
`open_id`。

空白名单不会放行任何用户，这是安全默认行为。

## 使配置生效

保存配置后重启正在运行的 VibeWindow daemon。通道在启动时读取白名单，
仅修改持久化配置不会改变已运行通道内存中的旧值。

重启后从已配置用户发送一条消息，并确认不再出现
`not in allowed_users` 日志。

## 排查清单

1. 确认填写的是 `open_id`，通常以 `ou_` 开头，而不是用户名或显示名。
2. 确认 ID 没有多余空格、引号或复制残缺。
3. 确认修改的是实际接收消息的 Feishu/Lark 实例。
4. 确认应用和租户与取得 `open_id` 时一致。
5. 保存配置后重启 daemon。

