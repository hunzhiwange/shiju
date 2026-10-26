# 两题 max_tokens 轨迹诊断

分析对象为 final-pilot 的既有 regex-log、sqlite-db-truncate 轨迹。首轮只做本地协议验证，新增模型调用为 0；后续按用户授权另作 GLM 模型对照，详见后续记录。原始模型、预算、评分规则和历史结果均未修改。

现有证据支持：运行时收到了 SDK 的长度终止信号，未发现本地按文本误判或把正常 stop 改成失败的缺陷。首轮未追查到外层重试；再次读取 Gateway 日志发现同一截断步骤实际尝试 4 次，外层忽略 `is_retryable=false`。下述“SDK 不重试”不代表整个调用栈不重试。

## 既有轨迹

| 题目 | 已完成步骤 | 截断步骤 | 截断步骤文本字符数 | 截断步骤执行工具数 |
| --- | --- | --- | --- | --- |
| regex-log | 0 | 1 | 55,186 | 0 |
| sqlite-db-truncate | 3（均为 tool_calls） | 4 | 39,553 | 0 |

regex-log 的推理不断讨论日期匹配；sqlite-db-truncate 在第四步长时间手工推导数据库十六进制内容。两者均未完成当前步骤，最后为标记不可重试的 APIError `max_tokens`，metadata 的 `stream_failure_kind=incomplete`、`source=aisdk`；没有正常 chat.done。耗时分别约 262 秒、270 秒，未触及 900 秒墙钟限制。截断步骤的字符数是多次尝试合计，不是单次请求的输出。

字符数不是 token 数。chat.delta 合并后的文本和 `<think>` 标记不能重建上游独立推理通道或计费数据。原记录无可信 usage，也没有原始 HTTP 请求体/上游 SSE，不能据此声称“刚好用完 32k”。

原始文件保留在忽略目录 `evals/promptfoo/results/terminal-bench/regression/final-pilot/jobs/`：

- regex-log events.jsonl SHA256：`9d170126ff8c4c54645c9165b824612787fc5686b5e2cf1b506bc606e89c9cc2`
- sqlite-db-truncate events.jsonl SHA256：`f1d8058707c491d10b35a17081c04052041b6f1b636b4d2fdc29f343804098e5`

## 底层调用链

冻结 Cargo.lock 使用 vibewindow-aisdk 0.7.1，revision `1a87cb54ca09f01125bbf985a3886e3ea19ca679`。其 OpenAI Chat Completions 流解析器在 `finish_reason=length` 时产生 `Incomplete("max_tokens")`；`stop` 走正常完成。本地 `crates/vw-agent/src/session/llm/aisdk/stream.rs` 将 Incomplete 转成上述错误，不搜索生成文本，SDK 层不重试；外层 `session/processor/llm_runner.rs` 原先仍允许重试，日志已证明两题各有 4 次失败尝试。这里的结论来自冻结源码映射与现存错误，不能代替已经缺失的原始上游报文。

请求经 `session/llm/stream.rs` 和 provider transform 计算上限，再由 SDK 序列化为 `max_completion_tokens`。全局默认封顶 32,000，实际值还受模型 output limit 和请求覆盖值约束；未知模型的共享默认 output limit 为 4,096。冻结模型配置没有显式 limit；仅列举模型 ID 的远程发现不等于发现容量。不能只看全局常量推定历史有效预算。

再次使用同一冻结 ELF 和相同 profile（provider digest 一致），通过真实 `/v1/provider` 回读得到 context=200,000、input=200,000、output=4,096。端点属于阿里云百炼的工作空间域名格式；此前将其视为未知自定义代理不够准确。[官方 API 文档](https://help.aliyun.com/en/model-studio/qwen-api-via-openai-chat-completions)说明 `max_completion_tokens` 包含推理和回答，超过后返回 length。由此更支持“推理耗尽输出额度”，不支持把 4,096 误解为输入上下文限制；原始请求/usage 缺失仍使历史精确 token 数无法确认。本次不替换字段、不提高上限、不改变推理参数。

另有用量可观测性缺口：SDK 对 choices 为空的 usage-only 尾包未在选择循环内处理，本地遇到 Incomplete 又会立即返回，后续 Done 的 usage 不会正常传播。这可以解释用量缺失，但没有证据表明它导致服务端提前停止；本次不将其包装成截断修复。

## 本地验证及后续

新增独立 `truncation_tests.rs`，通过真实 OpenAICompatible HTTP 流解析链连接本地 SSE fixture：

- 文本含 `max_tokens` 且终态 stop：成功并发出 Done。
- 相同文本且终态 length：失败，不发出 Done 或执行工具。
- 两种请求均验证模型 ID、给定 32,000 上限原样序列化为 `max_completion_tokens`，以及 include_usage=true；这是夹具输入，不能证明历史线上请求的上限。

两项聚焦测试通过；`cargo clippy --all-targets --all-features -- -D warnings` 退出 0（仅有既存 block 依赖的 future-incompat 提示）。动态构建槽位 1 已释放，缓存保留；完整验证收据见忽略目录 `evals/promptfoo/results/terminal-bench/max-tokens-analysis/verification.json`。原始失败评分继续保留。

后续已取得同一冻结运行时的有效模型上限，并核对官方参数语义，发现及修复外层重复重试；独立模型对照与验证详见 [输出截断与上下文对照](model-context-comparison.md)。

TODO：历史线上请求和精确 usage 仍缺失；若继续补取证，应只保留脱敏参数与计数，不记录凭据或原始任务载荷，不用追加预算来掩盖证据缺口。
