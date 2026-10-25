# Promptfoo 0.122.0 重复导出取证

2026-10-25；仅本地固定响应，无真实模型调用。批次 2 产物来自
`61465656a`，11 项 Node 测试通过；其无关 Clippy 待回归不影响此格式实验。
本任务未运行 Cargo，未修改比较器或 Agent。

## 来源与复现

本机 npm 缓存 `promptfoo/package.json` 的 version 为 `0.122.0`，bin 为
`dist/src/entrypoint.js`。先运行该安装的 `promptfoo eval --help`（退出 0），
确认下述参数。初次猜测 `index.js` 入口失败（退出 1，MODULE_NOT_FOUND），
读取 package.json 后改用真实 bin；该失败不属于行为验收。

仅执行一次评测，在临时工作目录和临时 PROMPTFOO_CONFIG_DIR 中执行；
`config.json` 通过绝对路径定位，Provider 相对配置文件解析。等价命令
（`<fixture>` 为本目录下 `fixtures/result-format` 的绝对路径，
`<tmp>` 为临时目录；`promptfoo` 指固定版本安装的 bin）：

```sh
PROMPTFOO_DISABLE_TELEMETRY=1 PROMPTFOO_DISABLE_UPDATE=1 \
PROMPTFOO_CONFIG_DIR=<tmp> CI=1 promptfoo eval \
  -c <fixture>/config.json --repeat 3 --no-cache -j 1 \
  --no-write --no-share --no-progress-bar --no-table \
  -o <fixture>/two-cases.json
```

Provider 的 `callApi` 只返回 `fixed` 与合成 tokenUsage，无网络代码；
断言为本地 equals。关闭遥测及更新检查，无远程模型或裁判。
确切调用使用缓存安装
`/Users/xiangmin/.npm/_npx/3f410cde517faec3/node_modules/.bin/promptfoo`；
运行 cwd 和配置目录由 Python `tempfile.TemporaryDirectory` 提供，结束即删除。

`eval.txt` 为真实 CLI 输出（仓库绝对路径替换为 `<repo>`），退出码 **100**：
6 次尝试，3 通过、3 断言失败、0 错误；故意失败的第二题验证失败字段，
不是环境失败。Provider 合成用量合计 18（输入 12、输出 6），不是实际模型用量。

## 样本裁剪

- `two-cases.json`：保留真实导出的 `results.version`、全部 6 条
  `results.results` 和 `results.stats`；删除其余根字段和 prompts 聚合。
- `one-case.json`：从同一次真实导出投影前三条 case-pass 记录，保留
  `results.version`；不是第二次运行，也不伪造 stats。
- 行内容（包括 UUID、testIdx、延迟和 tokenUsage）原样保留。
  输入仅 alpha/beta，输出仅 fixed，无源码、工具载荷或凭证。
- 未裁剪导出根字段为 evalId、results、config、shareableUrl、metadata、vars、
  runtimeOptions；metadata.promptfooVersion 为 0.122.0，
  runtimeOptions.repeat 为 3，maxConcurrency 为 1，cache 为 false。

## 实际字段

以下路径中 `row` 指 `results.results[i]`。

| 信息 | 实际位置与观察 |
| --- | --- |
| 原始样本 | `results.results`，未聚合的 6 条记录 |
| 通过 | `row.success` 和 `row.gradingResult.pass`，本例一致 |
| 失败 | `row.error`（通过行不存在）、`row.gradingResult.reason`；`row.failureReason` 为通过 0、断言失败 1 |
| Provider | `row.provider.label` = offline-fixed；id = local-fixed |
| 题目 | `row.testCase.description` = case-pass / case-fail |
| 分数 | `row.score`、`row.gradingResult.score` |
| 命名分 | `row.namedScores.correctness`；gradingResult 内也有 namedScores |
| 用量 | `row.response.tokenUsage` 为 Provider 返回值；`row.tokenUsage` 为归一化结果，含 assertions 子项；裁判用量在 `row.gradingResult.tokensUsed` |
| 延迟 | `row.latencyMs`；汇总耗时在 `results.stats.durationMs` / evaluationDurationMs |
| 汇总 | `results.stats.successes=3`、failures=3、errors=0；tokenUsage.total=18 |

本例只观察断言失败，未制造 Provider 错误，不能据此推断其他 failureReason 数值。

## 重复身份与比较器重放

每行有不同 UUID `row.id`，可在同一份导出内识别具体记录；它不是跨运行稳定
attempt 身份。本次 `testIdx` 为 0、1、2（case-pass），3、4、5（case-fail），
`promptIdx` 均为 0，promptId 相同。没有 attempt、attemptId 或 repeatIndex 字段。
因此不能把 testIdx 当作原始题号，或把 UUID 当作可跨运行配对的重复序号。
本例只能按 provider label + description 分组计数，每题确有 3 行；不承诺其他
并发、恢复或多 Provider 场景的顺序。

现有比较器对两个样本各自与自身比较，确切命令和输出见
`one-case-compare.txt`、`two-cases-compare.txt`，均退出 **1**：

```text
对比失败：before 结果包含重复记录：offline-fixed :: case-pass
```

这证实当前比较器拒绝真实重复行；本任务保留该行为，后续聚合任务可使用这些
样本。样本数来自原始行计数与 stats，不来自比较器的任务平均数。

## 验证

确切验证命令：`python3 evals/promptfoo/fixtures/result-format/verify.py`。
Python JSON 解析与断言：两份样本分别 3/6 行，description 分组分别为
`{case-pass: 3}` / `{case-pass: 3, case-fail: 3}`；UUID 全部唯一；无虚构
attempt 字段；3 个 success=true、3 个 success=false；stats 总数为 6。
验证退出 0，6 条原始尝试，单题投影的 3 条不重复计为额外尝试。
重放退出 1 为预期取证结果。本任务不适用红绿测试。
