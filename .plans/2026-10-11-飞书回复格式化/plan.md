# 飞书回复格式化计划

目标：修复 Lark/飞书 Markdown 出站渲染，并让其他通道的模型输出契约与实际传输能力一致。

范围：调整 `vw-agent` 的 Lark/飞书出站格式化与发送请求；普通 Markdown 使用富文本 `post`，补齐软换行语义；代码块或表格使用 `interactive` Markdown 卡片。同时审计已注册聊天通道，为 Slack、Discord、Mattermost、钉钉、Nextcloud Talk 及纯文本通道补齐与输出能力一致的提示契约与测试。

范围外：不改入站解析、消息分块/流式更新、媒体发送，不新增渲染配置、通用格式中间层或全通道语法转换器。

假设：`SendMessage.content` 受 `channel_delivery_instructions` 引导；Lark/飞书共用 `LarkChannel::send`。Telegram 已实现 Markdown-to-HTML 与纯文本降级，IRC 在入站消息中注入纯文本约束，不在本计划重复改造。

## 设计决策

- 选择 OpenClaw 式自动分流：普通内容保持 `post`，代码块/表格切换为 Markdown 卡片；不全量改为卡片，以保持简单回复的原生消息体验。
- 不引入新依赖或渲染配置；利用已有 `pulldown-cmark` 识别 CommonMark 软换行，避免纯正则破坏代码块等结构。
- 其他通道仅修复已确认的“提示契约与传输能力不一致”；平台已原生支持合适 Markdown 时不改发送代码。

## 架构说明

- 在 `channels/lark` 内新增纯格式化边界，输出“消息类型 + content JSON”；`LarkChannel::send` 只负责传输、令牌失效和重试。
- `channels/format.rs` 继续是通道输出语法约束的唯一共享入口；不在各发送器复制提示规则。
- 保持现有安全边界：内容通过 `serde_json` 序列化，不拼接 JSON，错误不记录令牌或原始敏感载荷。

## 开发策略

- 对会改变行为的代码，使用失败验证-通过验证-重构。
- 在实施步骤前规划失败验证命令和预期失败。
- 将实施步骤限制在失败测试所证明的行为范围内。
- 测试真实行为，不验证模拟对象或实现细节。
