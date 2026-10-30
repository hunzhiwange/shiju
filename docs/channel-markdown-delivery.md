# 渠道 Markdown 分块投递

QQ、飞书/Lark、企业微信、微信与钉钉沿用现有 `Channel::send` 和工厂注册。Webhook 通过生产通道运行时进入 `process_channel_message`，执行完成后由 `handle_llm_result` 统一做金丝雀检查、发送钩子、净化和会话持久化，再调用适配器。展示分块不各自写成助手回合；历史保存完整逻辑回复，生成或保存成功不代表全部送达。

## 本地预算与预检

下列大小是本地保守投递策略，不宣称是平台官方上限，也不把参考实现的字符数当成 UTF-8 字节数。

| 渠道 | 单块正文 | 最终载荷 | 发送方式 |
| --- | --- | --- | --- |
| QQ | 3000 UTF-8 字节 | 16 KiB JSON 请求体 | 正文逐块后发图片，确认后递增 `msg_seq` |
| 飞书/Lark | 3000 UTF-8 字节 | 16 KiB 最终 JSON 请求体，包含内层内容转义 | 逐块 post/card，业务 `code` 确认 |
| 企业微信 | 3000 UTF-8 字节 | 16 KiB JSON WebSocket 帧 | 同次回复固定 `req_id`，每块独立 `stream.id`、`finish=true`，逐块等 ACK |
| 微信 iLink | 3000 UTF-8 字节 | 16 KiB JSON 请求体 | 文本 `item_list`，固定本次 context，逐块不同 `client_id` |
| 钉钉 | 3000 UTF-8 字节，且保持原 3500 字符阈值以内 | 16 KiB JSON 请求体，包含标题与转义 | 长回复仍用多条 Markdown webhook 消息 |

1 KiB = 1024 字节。适配器先完成整篇规整、分块和全部可预先构造载荷的大小检查，才开始第一个请求。补齐或重开围栏、重复表头以及规整产生的空格均计入预算。QQ 图片上传后才得到的 `file_info` 在媒体消息投递前另行检查；上传本身不计为聊天消息确认。QQ 被动回复正文与图片总部分数预检不超过 5，超出时整次拒绝，不先发送前五部分。

共享分块器优先段落和行边界，普通长行仅按 Unicode 字符边界切分。能整体放下的代码块和表格保持整体；代码跨块时补闭合并重开同种、同长度和语言的围栏，表格只在数据行间拆分并重复原始表头与分隔行。代码行、表头加单个数据行、围栏信息串等原子结构无法在预算内安全容纳时明确拒绝，不能截断结构或先发送可用前缀。未闭合源围栏可补闭合用于展示，源内容不丢弃。

微信按文本展示这些内容；不承诺客户端渲染 Markdown、代码高亮或表格。飞书/Lark 使用同一适配器，不新增工厂 key。

## 确认与失败

一次 send 的错误通过 `ChannelDeliveryError` 保留 `confirmed_parts`、`total_parts`、零基 `failed_part_index` 和 `kind`。错误不保存正文、原始令牌或 webhook URL。

- `NotSent`：当前失败部分没有提交；之前的部分仍可能已经确认。
- `Rejected`：平台明确拒绝当前部分。
- `Unknown`：提交结果无法确认，例如超时、断线、5xx 或无法解释的业务响应；不能断言当前部分未送达。

HTTP 200 或本地入队不等于成功。只有各渠道明确的业务成功证据才推进已确认前缀；首个失败立即停止后续部分。企业微信同 `req_id` 的帧串行关联 ACK，写出后最多等待 5 秒；结果不明时在本连接冻结该 `req_id`，迟到 ACK 不移给后继项，重连清理旧状态且不重放旧队列。HTTP 发送使用本地最多 30 秒的超时策略。

QQ 仅在明确 Markdown 类型拒绝时降级当前块，并仅在明确 URL 内容错误时清理 URL；重试使用同一当前 `msg_seq`，候选正文与 JSON 预算提前检查。飞书/Lark 仅在明确令牌失效时刷新一次并重试当前块。其它业务失败以及未知结果不触发整篇降级、替换投递路径或全文重发。

生产入口对草稿完成错误检查结构化进度：已有确认部分或 `Unknown` 时抑制全文 fallback；确认零投递的安全失败可沿用现有 fallback，旧的非结构化草稿错误保留兼容行为。普通 `send` 失败只记录结果，不再次发送整篇；不会自动向用户渠道追加完整错误通知。

## 后续人工处理边界

这些游标只属于本次调用，不提供跨调用、跨进程、进程重启或用户手动重试的去重，也不承诺 exactly-once。独立新回复可以包含相同正文。部分失败时先核对平台实际消息与确认进度，再决定是否人工补发缺失内容；`Unknown` 必须先核对当前部分，盲目重试完整回复可能产生重复。诊断仅记录渠道、确认数/总数、失败索引与性质，不复制敏感正文或带令牌的 URL。

## 参考与验证边界

- 本地 QwenPaw 参考：`/Users/Shared/work/dir/data/codes/QwenPaw-main/src/qwenpaw/app/channels/utils.py` 的 `split_text`，及 `qq/channel.py`、`feishu/utils.py`、`wecom/channel.py`、`dingtalk/channel.py` 中的相关出站处理。它们是思路参考，不是本项目接口规格或可靠投递保证。
- [腾讯 QQ 消息收发说明](https://github.com/tencent-connect/bot-docs/blob/main/docs/develop/api-v2/server-inter/message/send-receive/send.md)：回执与被动回复约束参考。
- [企业微信官方 aibot Node SDK WebSocket 实现](https://github.com/WecomTeam/aibot-node-sdk/blob/main/src/ws.ts)：回调 `req_id` 及 ACK 队列参考；本实现对未知结果停发，未照搬超时后继续队列。

验证使用真实分块器、适配器和生产响应入口；仅在本机 HTTP/HTTPS/WebSocket 边界模拟平台，覆盖有序投递、部分拒绝、未知回执、超时、断线和首请求前预检。未进行真实账号线上消息发送验证。
