# Qwen3.8-Flash 重跑与输出上限对照

本轮按用户要求使用 `token-plan/qwen3.8-flash` 重跑 regex-log、sqlite-db-truncate。两题在用户授权的 32,000 单次输出额度下均正常结束，官方测试 2/2 通过。原五题样本 3/5 及所有历史失败保持不变；不同模型和预算的试验分开报告。

## 已确认修复的范围

已修的是外层重复重试：APIError 标记 `is_retryable=false` 时立即停止，不再将同一截断步骤执行 4 次。它不能让首次已经截断的任务自动完成。

本轮真正部署了该修复。为隔离其他会话改动，从提交 `470278872a22f3cc30a5ed18bd89c55ebf403e36` 和历史 diff 恢复已验证源码摘要 `5f586a428553e7cf69e5dfd64e120cc49a62b6bc22130154237c56d16921bde5`，只应用该生产修复，得到源码摘要 `ad57e0be7290755ca2885ecd7a1e1ee35cbbf603f9ae0c1ae78042ccb6ca788b`。

Linux ELF SHA256 为 `7ad73fbb2031953dc1fcdca03f16c5cc95229bb02968d17f331f70f066ba75de`；保留 SHA1DC 对齐访问修复，依赖锁和 sidecars 不变。该生产模块与此前通过 64 项聚焦测试及 Clippy 的模块逐字节一致。初次构建遇到 target 临时目录消失，失败收据保留；重建退出 0，源码摘要前后一致，构建槽位已释放，缓存保留。

## Qwen 原额度试验

模型目录回读为 context=200,000、input=200,000、output=4,096。两题都再次出现 max_tokens，且日志均为 attempt=1、retry_allowed=false，证明重试修复已在真实 Linux 运行时生效。

| 题目 | 模型终态 | 官方评分 |
| --- | --- | --- |
| regex-log | 第 1 步截断，无工具执行 | 0/1 |
| sqlite-db-truncate | 完成 5 步后在第 6 步截断 | APT 502/500 导致 curl、uvx 缺失；无 CTRF，属于基础设施失败 |

sqlite 的原始 reward=0 完整保留，但不把没有运行测试的这条记录算作有效 0/1。此阶段没有一道题正常完成。

## 用户授权的 32,000 输出对照

用户在明确知晓保持 Qwen、900 秒时限和评分不变、仅提高单次输出上限的方案后，回复“提高上限看看呢”。后续两题使用相同修复版 ELF，只将隔离 profile 的 `limit.output` 从 4,096 改为 32,000；实际目录已回读确认生效。context/input 仍为 200,000，推理参数、题目、测试、权限、镜像、1 CPU/2 GiB、Agent/verifier 各 900 秒均不变。串行，每题一次任务尝试，Harbor retries=0。

| 题目 | 正常终态 | 官方测试 | Agent 耗时 | 有效评分耗时 |
| --- | --- | --- | --- | --- |
| regex-log | stop | 1/1 | 512.394 秒 | 812.337 秒 |
| sqlite-db-truncate | stop | 1/1 | 601.561 秒 | 原评分 105.989 秒 + 仅验证恢复 24.736 秒 = 130.725 秒 |

每题 Agent 和 verifier 分别限时 900 秒，所有阶段均在各自额度内。regex 总试验耗时约 1,346 秒，包含安装、模型和评分，不能把两阶段误解为合计限时 900 秒。

实际生成的 regex.txt SHA256 为 `be2059c69eeb26c8c147aee89a48162ddcccb09e83c68df6cc29f869bb689258`；recover.json SHA256 为 `ecae180d3313a6553cb1e850f2ee926696c7949d86f77422d104497135637520`。两题均由本次 Qwen 作答生成，捕获后未修改。

SQLite 原官方 verifier 再次因 APT 500/502 未能安装 curl，未运行测试。原始 reward=0 和缺失 CTRF 保留；只对捕获的实际 recover.json 恢复评分，调用冻结 Harbor 的原 `Verifier.verify`，未重跑模型或改写产物。

独立评分镜像保留原基础层和 /app 输入文件摘要，以官方 Debian HTTPS、签名及已校验 CA 安装 curl，并缓存同版本 uv 0.9.5、CPython 3.13.9、pytest 8.4.1、pytest-json-ctrf 0.3.5。原题、测试和阈值未改。该镜像仅用于评分；oracle/nop 校准分别为 1/1、0/1，全部有效评分在原累计 900 秒时限内，恢复新增模型调用 0。两个模型运行容器始终使用原冻结镜像。

## 证据与解释

原额度运行收据在忽略目录 `evals/promptfoo/results/terminal-bench/qwen38-flash/`；32k 的授权、展开命令、manifest、实际模型上限、SSE、输出文件及评分在 `evals/promptfoo/results/terminal-bench/qwen38-flash-32k-review/`。源码、编译身份、模型、预算和评分分开记录，不覆盖历史成绩。

结论是原来的 4k 单次输出兜底额度不足，而不是必须扩大输入上下文：保持相同 Qwen、修复版 ELF 和 200k 输入窗口，只将输出改为 32k 后，两题均正常结束并通过。[阿里云官方参数文档](https://help.aliyun.com/en/model-studio/qwen-api-via-openai-chat-completions)说明 max_completion_tokens 包含推理和回答。上游精确 usage 仍缺失，不能把字符数当成 token 数或估算费用；一次对照也不能作为稳定提升的统计证明。

最终收据 `qwen38-flash-32k-review/verification.json` 为 allTasksPassed=true、有效评分 2、正常终态 2；两题不存在 max_tokens 失败。临时 host profile、模型任务容器及恢复评分容器均已清理，原始记录保留。

提交前在当前主工作树再次运行 `cargo clippy --all-targets --all-features -- -D warnings`，退出 0，仅有既存 block 依赖的 future-incompat 提示；动态槽位 1 已释放，缓存保留。验收收据和原始失败未被提交为测试答案。

32k 当前仅用于隔离试验，没有修改宿主全局默认输出上限。下一步建议为该模型显式保存已验证的 32k 输出配置；不要将本轮两个 Qwen 成绩回填成原 DeepSeek 的 5/5。
