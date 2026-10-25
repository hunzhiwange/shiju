# 首轮候选：reasoning 与最终答案通道契约

状态：证据不足，当前没有获准执行的生产修复任务。不是“仅模型波动”的结论；下一步是一个确定性通道实验，结果成立后最多生成一个窄缺陷任务。可将本文件与 optimization-cycle.md 作为 create-task-plans 输入，不自行创建会话或修复。

## 已满足依赖与边界

批次23提交9e93544ae和 accuracy-diagnosis.md 已交付。`results/task23/audit.json`覆盖两轮90次、45条失败；facts三题18次均严格JSON失败，工具错误0；仅离线去think后17/18匹配，不能作为修复通过证据。原始Gateway build ID为5ffdbb231a45abb15e4f98c792c041aa1715579c3300fb2ad47f62ce94444e03，与当前源码未绑定，缺少上游reasoning/text原始分类。因此尚不能决定应修改processor、Gateway还是评测消费契约。

范围仅reasoning是否混入最终答案；recovery、文件执行、模型额外解释、评分加载错误均不混入。JSON oracle保持严格，不做标签清理。

## 下一次唯一实验与真实入口

先绑定实际构建源码及features，沿任务23符号路径复核：

- `crates/vw-agent/src/session/processor/llm_runner.rs`：spawn_llm_stream_thread、collect_llm_step_from_stream；runner.rs的run为调用方。
- `crates/vw-agent/src/session/llm/aisdk/driver.rs`：dispatch_stream_request；stream.rs的TextDelta/ReasoningDelta源头保持独立。
- 独立测试入口 `crates/vw-agent/src/session/processor/llm_runner_tests.rs`，相邻测试collect_stream_wraps_reasoning_before_text，不能仅改它对展示标签的预期就宣称修复。
- 消费入口 `evals/promptfoo/vibewindow-provider.mjs` 的aggregateEvents；验收入口 `evals/promptfoo/artifact-oracle.mjs` 的exactJson。

以同一次确定性实验包含两组对照：A注入ReasoningDelta("plan")、Delta(有效JSON)、正常Done；B把含think标签的文本作为普通Delta注入，再Done。逐层记录类型化输入、processor转发、实际Gateway SSE、aggregateEvents输出和冻结oracle结果。A应只将JSON作为最终答案，reasoning展示仍独立可用；B不能被标签剥离成合格JSON。若原契约只定义展示流，先明确最终答案的现有权威来源，禁止猜新事件协议。缺少真实SSE重现入口时记录实验设施依赖，另行设计，不改生产代码凑实验。

若A已经正确分离，则推翻运行时合并假设，保留无修复结论；若A在同源二进制中混入reasoning，且最终答案合同明确，则支持最低合并层候选；B仍失败为必要对照。若需要跨模块新协议或多个独立设计决策，停止并按单行为拆分，不能直接生成多模块修复。

## 条件成立后的单一TDD任务约束

候选crate为vw-agent，features先采用 `--all-features`，实际构建features必须在实验前固定并记录。拟新增独立行为测试名 `collect_stream_preserves_reasoning_without_polluting_final_answer`，放在上述llm_runner_tests.rs，覆盖reasoning保留、最终JSON不污染、正常完成与普通文本对照；须调用实际生产路径，不能复制实现。若该层不能提供最终答案权威输出，不能伪造断言，应停止并修正规格责任层。

候选红/绿精确命令（新增前不会命中，当前不执行）：

```bash
cargo test -p vw-agent --lib --all-features session::processor::llm_runner_tests::collect_stream_preserves_reasoning_without_polluting_final_answer -- --exact
```

必须先核实实际模块完整名且有效测试数为1；零测试不算证据。红灯先证明A真实混入，最小实现后同命令绿灯；相邻回归过滤命令：

```bash
cargo test -p vw-agent --lib --all-features session::processor::llm_runner_tests::
```

生产修改仅限经实验确认的最低责任层；不提前指定新trait、配置、事件类型或上层绕过。以上命令是条件规格，不是已验证的修复入口。新增Rust任务须完整附 optimization-cycle.md 的10条共享槽位规则并按其单轮顺序执行；当前没有失败到通过日志，未运行Cargo或模型请求。

## 进入优化的附加依赖

独立before/after及保留集通用执行入口尚未具备，详见 optimization-cycle.md。安全隔离与后台回收仍未验证，不能承诺保留集安全验收。先补齐有证据的执行契约，再决定是否规划一个窄修复；不得把这些设施缺口与reasoning修复并为一个任务。
