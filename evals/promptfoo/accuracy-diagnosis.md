# 首要失败归因：推理展示流进入最终答案

2026-10-25，任务23，仅诊断。批次22依赖已满足：提交 `132967ca9`、
`results/accuracy-baseline/recheck-audit.json` 与 `noise.md` 记录独立第二轮45次尝试；
两轮共90次，分别通过23/45、22/45，失败22、23次。未开启保留集或新模型请求。

## 合同与限制

筛选为 `--suite runtime-accuracy --metadata split=dev`，15题各3次，单并发、无缓存；
Provider label `runtime-accuracy-unisolated-token-plan`，模型
`token-plan/deepseek-v4-flash-0731`，endpoint `http://127.0.0.1:42617/v1/chat/stream`。
冻结题集摘要 `2acc7e6cbb8c96645f4d3a00a9cdbeddd564f37efa99e2279c9b797563490187`；
实验合同摘要 `398092ef8d388ffc292771b803750f9c2321538f777874b4b8f54eee59c4b671`；
两轮 Gateway build ID 均为 `5ffdbb231a45abb15e4f98c792c041aa1715579c3300fb2ad47f62ce94444e03`。
非隔离合同；安全边界与后台回收未验证。源码与该二进制未建立构建对应关系。
工作树有既有 Rust 改动及其他任务状态变化，本任务不覆盖、不暂存这些内容。

## 唯一主要假设与证据

**假设 H：session processor 将类型化 reasoning 转为普通 Delta 展示，评测 Provider
再把整轮展示流当作最终答案，导致正确事实被严格 JSON oracle 拒绝。**
最低可见语义合并点为 `collect_llm_step_from_stream`，而非文件工具返回错误。
这是跨输出通道的契约不匹配假设，尚不能据此认定线上二进制的具体实现缺陷。

- `before/attempts/facts-01-1`：两次 `file_read` 均 completed、isError=false；
  读取版本字段后，输出末尾为匹配参考值的对象，但前面有两个 think 块；
  `final_json=false`，文件不变检查全部通过，终态/预算均通过。
- `recheck/recheck-attempts/facts-01-1`：同一题的独立 attempt 再次出现 think 块；
  `final_json=false`，工具无失败，终态/预算均通过。
- 所有 facts-01/02/03 两轮18次均有 think 块，工具失败0次，严格最终 JSON 全失败。
  一次离线反事实解析实验仅移除成对 think 块，17/18成为完全匹配的 JSON；
  `recheck/facts-03-3` 还包含解释与 Markdown，因此仍失败，属于模型输出格式违约的剩余证据。
  这不是重评分或修复：原始45条失败和冻结 oracle 原样保留，也不声称通过率提升。

原记录定位规则：before 对应 `results/accuracy-baseline/attempts/<attempt>/result.json`，
recheck 对应 `results/accuracy-baseline/recheck-attempts/<attempt>/result.json`。
`results/task23/audit.json` 为逐条 attemptId、文件 SHA-256、原始文件位置、工具事件、
oracle失败项及18条反事实实验结果；原始内容与载荷只留 Git 忽略的 results 目录。

## 最低责任层、调用方、工厂与相邻独立测试

通过 RustCodeGraph status（4304文件）及 node/callers/query 读取当前索引；未更新索引或构建。
以下是当前源码的精确定位，不将其冒充采样二进制对应源码：

| 环节 | 文件与符号、行 | 观察 |
| --- | --- | --- |
| 入口/调用方 | `crates/vw-agent/src/session/processor/runner.rs:423`，`run` | 图记录调用 `run_llm_step_with_retry`，入口未沿旧 agent/loop_ 推断 |
| LLM桥接 | `crates/vw-agent/src/session/processor/llm_runner.rs:180`，`spawn_llm_stream_thread` | 192行进入 `stream_chat_with_tools_for_session_with_abort` |
| Provider工厂分派 | `crates/vw-agent/src/session/llm/aisdk/driver.rs:72`，`dispatch_stream_request` | 88行 send_model 宏、93行调用 `do_stream_request_aisdk_with_model` |
| 原始通道 | `crates/vw-agent/src/session/llm/aisdk/stream.rs:393` / `:401` | TextDelta与ReasoningDelta分开回调；当前文件有既有修改 |
| 最低合并点 | `crates/vw-agent/src/session/processor/llm_runner.rs:207`，`collect_llm_step_from_stream` | 322–344行把 reasoning 加 think 标签后作为普通 StreamEvent::Delta 发出；299–303行关闭标签 |
| 独立测试 | `crates/vw-agent/src/session/processor/llm_runner_tests.rs:374`，`collect_stream_wraps_reasoning_before_text` | 385–387行分别验证 step.reasoning_content、step.text，但转发 deltas 包含 think |
| 工厂相邻测试 | `crates/vw-agent/src/session/llm/aisdk/stream_tests.rs:322`，`stream_sends_deltas_tool_calls_usage_and_clamped_request_options` | 图显示直接调用流函数；本任务未运行 Rust 测试 |
| 评测聚合 | `evals/promptfoo/vibewindow-provider.mjs:166`，`aggregateEvents` | 190–193行无通道区分地追加所有 chat.delta |
| 独立验收 | `evals/promptfoo/artifact-oracle.mjs:17`，`exactJson` | JSON.parse整个response，94行执行final_json；按冻结要求正确拒绝额外文本 |

## 全部失败逐条分诊

两轮正常终态和预算全部通过，无正式采样运行错误。基础设施问题为两轮各45条
Promptfoo命名评分函数加载错误，已由既有离线恢复保留同一90条模型响应；
另有首轮采样前5次鉴权错误、0模型请求，单列在 zero-upstream-auth-failure，不混入90条。
这些基础设施问题不是下表45条恢复后 assertion 失败的直接原因。

分类边界：事实题为上下文/输出通道契约候选；recovery为工具契约/执行待区分，
不能将命令输出中的“退出码”当工具的真实 exitCode。其他文件结果失败归入
上下文/执行结果待区分，工具失败单列。停止类别无预算/终态失败证据；
模型推理错误没有足够证据，facts-03-3额外解释只证明格式违约。
不对这些次要模式提出修复，避免混入多个主因。

表中每一行均读取对应原始工具事件；顺序与原始 toolCalls 一致。
`completed/exit=0` 表示外层命令成功，不证明内部子命令成功；缺失 exitCode 不补零。

| 轮次 | taskId/attempt | 导出行 | 分类 | oracle失败项 | 评分失败项 | 真实工具事件（name:status/exit） |
| --- | --- | ---: | --- | --- | --- | --- |
| before | facts-01-1 | 1 | 上下文/输出通道契约 | final_json | 缺少可信 oracle 或产物验收失败 | file_read:completed, file_read:completed |
| before | facts-01-2 | 2 | 上下文/输出通道契约 | final_json | 缺少可信 oracle 或产物验收失败 | file_read:completed, file_read:completed |
| before | facts-01-3 | 3 | 上下文/输出通道契约 | final_json | 缺少可信 oracle 或产物验收失败 | file_read:completed, file_read:completed |
| before | facts-02-1 | 4 | 上下文/输出通道契约 | final_json | 缺少可信 oracle 或产物验收失败 | file_read:completed, bash:completed/exit=0 |
| before | facts-02-2 | 5 | 上下文/输出通道契约 | final_json | 缺少可信 oracle 或产物验收失败 | file_read:completed |
| before | facts-02-3 | 6 | 上下文/输出通道契约 | final_json | 缺少可信 oracle 或产物验收失败 | glob:completed, file_read:completed |
| before | facts-03-1 | 7 | 上下文/输出通道契约 | final_json | 缺少可信 oracle 或产物验收失败 | file_read:completed, file_read:completed |
| before | facts-03-2 | 8 | 上下文/输出通道契约 | final_json | 缺少可信 oracle 或产物验收失败 | ls:completed, glob:completed, file_read:completed, file_read:completed |
| before | facts-03-3 | 9 | 上下文/输出通道契约 | final_json | 缺少可信 oracle 或产物验收失败 | ls:completed, glob:completed, grep:completed, file_read:completed, file_read:completed |
| before | edit-01-1 | 10 | 工具契约/执行（待区分） | 全部通过 | 未声明的工具失败不得豁免 | ls:completed, glob:completed, file_read:completed, file_edit:completed, file_read:completed, grep:failed, grep:completed |
| before | recovery-01-1 | 19 | 工具契约/执行（待区分） | 全部通过 | 恢复契约无效或失败次数不为一次 | ls:completed, glob:completed, file_read:completed, file_read:completed, file_read:completed, bash:completed/exit=0, file_write:completed, bash:completed/exit=0, bash:completed/exit=0, file_read:completed |
| before | recovery-01-2 | 20 | 工具契约/执行（待区分） | 全部通过 | 恢复契约无效或失败次数不为一次 | glob:completed, ls:completed, file_read:completed, file_read:completed, file_read:completed, bash:completed/exit=0, file_write:completed, bash:completed/exit=0, bash:completed/exit=0, file_read:completed, bash:completed/exit=0, file_write:completed, bash:completed/exit=0, bash:completed/exit=0, bash:completed/exit=0, file_read:completed |
| before | recovery-01-3 | 21 | 工具契约/执行（待区分） | 全部通过 | 缺少真实退出码或恢复次序不符 | ls:completed, bash:completed/exit=0, file_read:completed, file_read:completed, file_read:completed, bash:completed/exit=0, file_write:completed, bash:failed, bash:completed/exit=0, file_write:completed, bash:completed/exit=0, file_write:completed, bash:completed/exit=0, bash:completed/exit=0 |
| before | recovery-02-1 | 22 | 工具契约/执行（待区分） | 全部通过 | 恢复契约无效或失败次数不为一次 | ls:completed, file_read:completed, file_read:completed, bash:completed/exit=0, file_write:completed, bash:completed/exit=0, bash:completed/exit=0, bash:completed/exit=0, bash:completed/exit=0, bash:completed/exit=0, file_read:completed |
| before | recovery-02-2 | 23 | 工具契约/执行（待区分） | 全部通过 | 缺少真实退出码或恢复次序不符 | ls:completed, bash:completed/exit=0, file_read:completed, file_read:completed, bash:completed/exit=0, file_write:completed, bash:failed, bash:completed/exit=0, bash:completed/exit=0, bash:completed/exit=0 |
| before | recovery-02-3 | 24 | 工具契约/执行（待区分） | 全部通过 | 恢复契约无效或失败次数不为一次 | bash:completed/exit=0, ls:completed, file_read:completed, file_read:completed, bash:completed/exit=0, bash:completed/exit=0, bash:completed/exit=0 |
| before | recovery-03-1 | 25 | 工具契约/执行（待区分） | 全部通过 | 恢复契约无效或失败次数不为一次 | TodoWrite:completed, ls:completed, file_read:completed, file_read:completed, file_read:completed, bash:completed/exit=0, TodoWrite:completed, file_write:completed, bash:completed/exit=0, bash:completed/exit=0, bash:completed/exit=0, bash:completed/exit=0, bash:completed/exit=0, bash:completed/exit=0, TodoWrite:completed |
| before | recovery-03-2 | 26 | 工具契约/执行（待区分） | 全部通过 | 缺少真实退出码或恢复次序不符 | bash:completed/exit=0, ls:completed, file_read:completed, file_read:completed, file_read:completed, bash:completed/exit=0, bash:completed/exit=0, file_write:completed, bash:completed/exit=0, bash:completed/exit=0, file_edit:failed, file_read:completed, bash:completed/exit=0, bash:completed/exit=0, bash:completed/exit=0, bash:completed/exit=0, bash:completed/exit=0 |
| before | recovery-03-3 | 27 | 工具契约/执行（待区分） | 全部通过 | 恢复契约无效或失败次数不为一次 | ls:completed, file_read:completed, file_read:completed, file_read:completed, bash:completed/exit=0, bash:completed/exit=0, file_write:completed, bash:completed/exit=0, file_write:completed, bash:completed/exit=0, bash:completed/exit=0 |
| before | state-02-2 | 32 | 工具契约/执行（待区分） | 全部通过 | 未声明的工具失败不得豁免 | ls:completed, file_read:completed, file_read:completed, memory_store:completed, file_read:completed, file_edit:failed, file_read:completed, bash:completed/exit=0, bash:completed/exit=0, bash:completed/exit=0, bash:completed/exit=0 |
| before | state-02-3 | 33 | 上下文/执行结果（待区分） | expectedFiles | 缺少可信 oracle 或产物验收失败 | glob:completed, ls:completed, file_read:completed, file_read:completed, memory_store:completed, file_read:completed, file_write:completed, file_read:completed, bash:completed/exit=0 |
| before | finish-01-1 | 37 | 工具契约/执行（待区分） | 全部通过 | 未声明的工具失败不得豁免 | ls:completed, file_write:completed, bash:failed, bash:completed/exit=0, bash:completed/exit=0, bash:completed/exit=0 |
| recheck | facts-01-1 | 1 | 上下文/输出通道契约 | final_json | 缺少可信 oracle 或产物验收失败 | file_read:completed, file_read:completed |
| recheck | facts-01-2 | 2 | 上下文/输出通道契约 | final_json | 缺少可信 oracle 或产物验收失败 | glob:completed, file_read:completed, file_read:completed |
| recheck | facts-01-3 | 3 | 上下文/输出通道契约 | final_json | 缺少可信 oracle 或产物验收失败 | bash:completed/exit=0, file_read:completed, file_read:completed |
| recheck | facts-02-1 | 4 | 上下文/输出通道契约 | final_json | 缺少可信 oracle 或产物验收失败 | ls:completed, glob:completed, file_read:completed |
| recheck | facts-02-2 | 5 | 上下文/输出通道契约 | final_json | 缺少可信 oracle 或产物验收失败 | file_read:completed |
| recheck | facts-02-3 | 6 | 上下文/输出通道契约 | final_json | 缺少可信 oracle 或产物验收失败 | ls:completed, file_read:completed |
| recheck | facts-03-1 | 7 | 上下文/输出通道契约 | final_json | 缺少可信 oracle 或产物验收失败 | ls:completed, glob:completed, grep:completed, file_read:completed, file_read:completed |
| recheck | facts-03-2 | 8 | 上下文/输出通道契约 | final_json | 缺少可信 oracle 或产物验收失败 | ls:completed, glob:completed, file_read:completed, file_read:completed |
| recheck | facts-03-3 | 9 | 上下文/输出通道契约 | final_json | 缺少可信 oracle 或产物验收失败 | ls:completed, file_read:completed, file_read:completed |
| recheck | edit-01-3 | 12 | 工具契约/执行（待区分） | 全部通过 | 未声明的工具失败不得豁免 | glob:completed, ls:completed, file_read:completed, file_edit:completed, file_read:completed, grep:failed, bash:completed/exit=0 |
| recheck | recovery-01-1 | 19 | 工具契约/执行（待区分） | 全部通过 | 恢复契约无效或失败次数不为一次 | bash:completed/exit=0, file_read:completed, file_read:completed, file_read:completed, bash:completed/exit=0, file_edit:completed, bash:completed/exit=0 |
| recheck | recovery-01-2 | 20 | 工具契约/执行（待区分） | 全部通过 | 恢复契约无效或失败次数不为一次 | ls:completed, file_read:completed, file_read:completed, file_read:completed, bash:completed/exit=0, file_write:completed, bash:completed/exit=0, bash:completed/exit=0, bash:completed/exit=0, bash:completed/exit=0 |
| recheck | recovery-01-3 | 21 | 工具契约/执行（待区分） | 全部通过 | 缺少真实退出码或恢复次序不符 | ls:completed, bash:completed/exit=0, file_read:completed, file_read:completed, file_read:completed, bash:completed/exit=0, file_write:completed, bash:completed/exit=0, bash:failed, bash:completed/exit=0, bash:completed/exit=0, bash:completed/exit=0 |
| recheck | recovery-02-1 | 22 | 工具契约/执行（待区分） | 全部通过 | 恢复契约无效或失败次数不为一次 | glob:completed, ls:completed, file_read:completed, file_read:completed, bash:completed/exit=0, bash:completed/exit=0, bash:completed/exit=0 |
| recheck | recovery-02-2 | 23 | 工具契约/执行（待区分） | 全部通过 | 恢复契约无效或失败次数不为一次 | bash:completed/exit=0, ls:completed, file_read:completed, file_read:completed, bash:completed/exit=0, file_write:completed, bash:completed/exit=0, apply_patch:completed, bash:completed/exit=0 |
| recheck | recovery-02-3 | 24 | 工具契约/执行（待区分） | 全部通过 | 恢复契约无效或失败次数不为一次 | bash:completed/exit=0, ls:completed, file_read:completed, file_read:completed, bash:completed/exit=0, file_write:completed, bash:completed/exit=0, bash:completed/exit=0, bash:completed/exit=0, file_read:completed |
| recheck | recovery-03-1 | 25 | 工具契约/执行（待区分） | 全部通过 | 恢复契约无效或失败次数不为一次 | bash:completed/exit=0, ls:completed, file_read:completed, file_read:completed, file_read:completed, bash:completed/exit=0, file_write:completed, bash:completed/exit=0, bash:completed/exit=0 |
| recheck | recovery-03-2 | 26 | 工具契约/执行（待区分） | 全部通过 | 恢复契约无效或失败次数不为一次 | ls:completed, bash:completed/exit=0, file_read:completed, file_read:completed, file_read:completed, bash:completed/exit=0, bash:completed/exit=0, bash:completed/exit=0 |
| recheck | recovery-03-3 | 27 | 工具契约/执行（待区分） | 全部通过 | 恢复契约无效或失败次数不为一次 | ls:completed, file_read:completed, file_read:completed, file_read:completed, bash:completed/exit=0, file_edit:completed, bash:completed/exit=0, file_read:completed |
| recheck | state-02-2 | 32 | 上下文/执行结果（待区分） | expectedFiles | 缺少可信 oracle 或产物验收失败 | glob:completed, ls:completed, file_read:completed, memory_store:completed, ls:completed, file_read:completed, file_read:completed, file_write:completed, file_read:completed, bash:completed/exit=0 |
| recheck | finish-01-1 | 37 | 工具契约/执行（待区分） | 全部通过 | 未声明的工具失败不得豁免 | ls:completed, file_write:completed, bash:failed, bash:completed/exit=0 |
| recheck | finish-01-3 | 39 | 工具契约/执行（待区分） | 全部通过 | 未声明的工具失败不得豁免 | file_write:completed, bash:failed, bash:completed/exit=0, bash:completed/exit=0, bash:completed/exit=0 |
| recheck | finish-03-3 | 45 | 上下文/执行结果（待区分） | expectedFiles | 缺少可信 oracle 或产物验收失败 | ls:completed, file_read:completed, file_read:completed, file_write:completed, file_read:completed, bash:completed/exit=0, bash:completed/exit=0 |
## 替代解释与下一次最小实验

替代解释 A：模型将 think 标签直接放在普通 text，或评测端使用了本就定义为展示用途的
chat.delta；原记录没有上游原始 reasoning/text 通道，不能排除。替代解释 B：
多工具步骤的普通说明也会进入聚合文本；移除 reasoning 仍非 JSON 的那1条说明
单独隔离 reasoning 未必解决所有输出格式违约。源码/二进制不对应是归因的不确定因素。

下一次仅做一个确定性通道重现实验，先绑定实际 Gateway 的构建源码，在最低合并点
注入 `ReasoningDelta("plan")`、`Delta(有效JSON)`、正常 Done，逐层捕获类型化输入、
processor转发、Gateway SSE和Provider输出，沿用冻结 JSON oracle。
若类型化 reasoning 从未进入普通最终答案而仍失败，则推翻 H，检查展示/最终输出契约；
若注入正确 text 被合并后拒绝，则支持 H。还需以普通 TextDelta 包含标签作对照，
区分模型误用和运行时合并；不能通过评测层剥标签、改提示词或放宽 oracle 实现修复。
这只是下次实验规格，本任务未执行、未新增基础设施或生产测试。

## 本次验证

- `python3 evals/promptfoo/results/task23/diagnose.py`：退出0；读取90条正式尝试，
  逐条追踪45条失败；唯一离线反事实实验18条，原始匹配0/18，移除think后匹配17/18，
  工具错误0，新增模型尝试0。脚本、摘要、输入文件 SHA-256 在 `results/task23/`。
- `evals/promptfoo/score.sh test`：退出0，配置有效；结果见 `results/task23/score-test.log`。
- `node evals/promptfoo/freeze-accuracy.mjs verify`：退出0，冻结字节未改。
- RustCodeGraph `status`、`node --file … --offset … --limit …`、
  `callers run_llm_step_with_retry --limit 8`、
  `callers do_stream_request_aisdk_with_model --limit 5`：退出0；定位上述入口与边界。
- 纯文档取证，不调用Cargo；不存在生产行为失败到通过的日志。
  最终的有效尝试数仍为90，失败45；诊断完成不等于能力改善。
