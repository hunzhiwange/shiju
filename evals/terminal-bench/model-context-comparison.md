# 输出截断与上下文对照

原五题样本仍为 3/5；本轮换模型是用户授权的独立诊断，不覆盖历史失败，不计入原模型验收。单次模型对照只能帮助定位，不能证明稳定提升。

## 冻结条件

原模型 `token-plan/deepseek-v4-flash-0731`，对照模型 `token-plan/glm-5.2`。同一个 token-plan 端点、适配协议、冻结 Linux ELF（SHA256 `3176b9c4804234d7e97aba05d0b6822d51cdf850f6a6b2e10c897ff9ce234a61`）、工具与权限、v5 镜像、题目/测试文件，以及 Agent/verifier 各 900 秒；每题一次有效模型尝试，串行，Harbor retries=0。运行时内部原有重试行为保留在本次模型对照中。

通过同一 ELF 的 `/v1/provider` 读取实际模型目录，原模型与 GLM 均为 context=200,000、input=200,000、output=4,096，能力配置一致。原模型 profile digest 与 final-pilot 完全一致，不只是引用源码默认值。GLM 对照在调用模型前验证上限相同；未提高预算或调整推理参数。

仅在忽略目录复制适配器，将模型绑定和 manifest 模型 ID 换成 GLM，并增加只读目录观测；生产评测适配器与原冻结 manifest 未修改。题目目录从 final-pilot 逐文件复制并校验摘要。执行版本、命令和原始产物位于 `evals/promptfoo/results/terminal-bench/model-context-comparison/`。

## 原轨迹给出的上下文证据

| 指标 | regex-log | sqlite-db-truncate |
| --- | --- | --- |
| 题目字符数 | 1,021 | 285 |
| 截断前完成工具调用 | 0 | 4 |
| 工具输出字符合计 | 0 | 1,369 |
| 工具错误字符合计 | 0 | 77 |
| 日志中的系统提示 joined_len | 20,137 | 20,137 |
| 截断步骤实际失败尝试 | 4 | 4 |
| 已记录 context overflow 错误 | 0 | 0 |

上述字符数不等于输入 token 数，也不包含工具 schema 的完整序列化大小。但 regex 首步便失败，sqlite 的工具回传很小，未出现上下文溢出错误；现有证据不支持“工具返回或长历史撑爆输入窗口”。

前轮只读 chat.delta，将截断步骤累计 55,186/39,553 字符看成一段长输出，漏掉 Gateway 外层重试。Gateway 日志显示两题各有 4 次 `attempt failed`，每次均为 `is_retryable=false`、`retry_allowed=true`。regex 的四个推理段分别为 13,655/14,890/12,835/13,738 字符，不能将合计值当成单次请求长度。外层循环重发同一份 messages/system，没有把失败推理追加进下一次请求；每次日志的系统提示长度也保持一致，不能将重试次数理解为输入上下文持续膨胀。

[阿里云官方 API 文档](https://help.aliyun.com/en/model-studio/qwen-api-via-openai-chat-completions)说明 `max_completion_tokens` 限制推理与回答的总和，达到上限返回 length。该端点使用官方百炼工作空间域名格式。结合实际 output=4,096，更符合“推理消耗单次输出额度后截断”，而非 200k 输入窗口耗尽。由于旧 SDK 未完整传播 usage，原始精确输入/推理 token 数仍缺失，不能声称已由线上计数证明恰好 4,096。

## 已确认的底层缺陷及最小修复

`session/processor/llm_runner.rs` 的外层重试只排除若干文本错误，忽略结构化 APIError 的 `is_retryable=false`。SDK 正确发出长度错误后，外层仍对同样上下文重复调用，累计消耗与输出因此被放大。

修复仅让非 ACP 路径尊重已存在的 APIError 不可重试标记；保留原错误、失败状态及可重试错误的既有行为。没有将截断记为成功，也未增加输出额度。它解决重复消耗，不能让首次已截断的答案自动完成。

本地红测以真实结构化长度错误进入完整外层循环，原代码观察到 4 次请求，期望 1 次。原 macOS 测试构建在断言 panic 后异常中止；保存的 `--nocapture` 日志明确包含 left=4/right=1，不把中止误报成正常完成的测试失败统计。修复后 62 项 runner 测试及 2 项真实 HTTP 协议测试通过；完整 Clippy 退出 0。动态构建槽位 1 已释放，缓存保留。

## 模型对照记录

| 题目 | 原 DeepSeek 官方评分 | GLM 官方评分 | GLM 终态 | GLM 完成工具数 | GLM 内部截断尝试 |
| --- | --- | --- | --- | --- | --- |
| regex-log | 0/1 | 0/1 | max_tokens | 0 | 4 |
| sqlite-db-truncate | 0/1 | 0/1 | max_tokens | 4 | 4 |

GLM 两题均失败；sqlite 截断发生于第 3 步，regex 仍为第 1 步。两题总试验耗时分别约 476 秒和 563 秒（含安装与评分），各自产生真实 CTRF 和 reward=0。不是只凭 Agent 终态推定得分。

独立审计 `verification.json` 记录有效模型任务试验 2、有效评分 2、通过 0、`auditPassed=true`、`benchmarkPassed=false`。这是证据审计通过，不是题目通过。原始 3/5 成绩未改；64 项本地 Rust 测试和 Clippy 通过也不等于两题通过。两个模型试验均使用修复前的同一冻结二进制，不能把它们当成重试修复后的能力复测。

首次 regex 诊断探针依赖镜像内不存在的 Python，在调用模型前失败；记录与空轨迹保留为基础设施失败，模型调用数 0。已改用镜像自带 Bash 观测，停止该无模型尝试的评分进程后独立重跑，不把它算成模型失败或正式得分。

有效执行适配器 SHA256 为 `bfdad2c43297f95b797ecbca1225becaf4bcc266bb9f5c790177f020f3d5f62b`；初始 `evidence.json` 的 adapterSha256 属于失败的 Python 预检版本。每个实际 Agent result 的执行摘要和 `adapter-provenance.json` 明确区分两者，保留初始证据不覆盖。两次有效试验及无模型预检的临时 profile 已删除，所有本轮题目容器已回收，原始轨迹与评分完整保留。

结论：换 GLM 没有解除截断。实际配置、官方参数语义、两个模型的结果，以及短输入/少量工具输出共同支持“输出额度不足”优先于“输入上下文溢出”。仍缺精确 usage，暂不能把这一推断升级为线上 token 计数证明。下一项有价值的实验是保持模型与评分不变，只单独测试输出额度；本轮没有启动该预算实验，也未调整推理强度。

后续用户指定 Qwen3.8-Flash 并授权单独提高输出上限；新对照见 [Qwen 重跑结果](qwen38-flash-rerun.md)。该后续结果不覆盖本页 GLM 与原模型的失败。
