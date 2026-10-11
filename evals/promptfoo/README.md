# VibeWindow Promptfoo 评测

该目录把 VibeWindow `/v1/chat/stream` SSE 协议适配为 Promptfoo Provider，用于重复运行 Agent 任务、检查最终回答、工具轨迹、token 和耗时。

## 环境要求

- VibeWindow gateway 已启动，默认地址为 `http://127.0.0.1:42617`。
- Node.js 22.22.0 或更高版本。
- 从仓库根目录运行命令。

首次运行会由 `npx` 下载固定版本 Promptfoo，耗时可能较长，但不会修改 Rust workspace 或生成仓库级 Node 依赖。

## 运行

统一使用评分脚本。先检查本地 JavaScript 语法和 Promptfoo 配置：

```bash
evals/promptfoo/score.sh test
```

运行完整评测；日常运行默认每项执行一次：

```bash
evals/promptfoo/score.sh run
```

像单元测试一样按测试组运行：

```bash
evals/promptfoo/score.sh run --suite selection
```

单独运行 TypeScript 编辑器 coding 用例：

```bash
evals/promptfoo/score.sh run --suite coding
```

运行会话控制首次选中技能的权限回归用例：

```bash
evals/promptfoo/score.sh run --suite coding-skill-selection
```

运行 Thread 命令顺序轨迹用例：

```bash
evals/promptfoo/score.sh run --suite thread-commands1
```

该用例在同一父任务中分轮覆盖 `create_thread`、`list_threads`、
`read_thread`、`wait_threads`、`send_message_to_thread`、`fork_thread`、
`set_thread_title`、`set_thread_pinned`、`set_thread_archived` 和 `handoff_thread`。
评分依据 SSE 中的真实工具事件与持久化结果；`handoff_thread` 必须对原始
`thread_id` 明确报告不支持，不将最终回答中的命令名称当作成功证据。

运行时切换模型，无需修改 YAML：

```bash
evals/promptfoo/score.sh run --suite coding --model provider/model
```

脚本固定 Promptfoo 版本、配置、禁用缓存并将并发设为 1，避免各次评分参数漂移。
`run`、`before` 和 `after` 统一使用 `promptfooconfig.yaml`；可用 `--suite`、`--batch`、
`--metadata KEY=VALUE`、`--provider`、
`--pattern` 和 `--failing` 缩小范围，用 `--model` 切换模型；多个 metadata 筛选使用 AND 逻辑。

Provider 会将 `max_output_tokens` 设为 `0`，沿用模型自身上限，避免 DeepSeek V4 Flash 的长工具调用
轨迹被网关通用 32000 token 限制截断。
Provider 在收到 step、工具及终态 SSE 事件时向 stderr 输出进度；达到 24 个 Agent 迭代或 32 次
工具调用后，通过流请求 ID 调用 Gateway stop 接口。可通过用例变量或 Provider 配置的
`maxIterations`、`maxToolCalls` 调整，用例变量优先。

评测矩阵从 `promptfooconfig.yaml` 的 `providers` 读取；每个 Provider 对同一组任务运行，
Provider `label` 是前后结果匹配时的模型键，比较期间不要改名。Agent 工作目录配置为 `.`，
相对路径始终以 VibeWindow 仓库根目录为基准，评测对象就是自身代码。删除模型配置或单独
复用 Provider 时，默认模型为 `token-plan/deepseek-v4-flash-0731`。

评测失败时 Promptfoo 返回退出码 `100`，工具或配置错误返回退出码 `1`。查看本地报告：

```bash
npx --yes promptfoo@0.122.0 view
```

## 保存并比较优化前后结果

结果 JSON 可能包含源码片段和完整工具 metadata，只保存在已忽略的 `results/` 目录，
不要提交或共享。以下命令构成固定操作闭环。

1. 在未修改产品代码、Gateway 已运行时保存 before：

   ```bash
   evals/promptfoo/score.sh before
   ```

2. 修改产品代码并完成聚焦验证，然后终止旧 Gateway，在独立终端用评测端口重新启动：

   ```bash
   cargo run -p vw-cli --bin shiju -- gateway --host 127.0.0.1 --port 42617
   ```

3. 使用完全相同的配置和参数保存 after，再生成 Markdown 差异报告：

   ```bash
   evals/promptfoo/score.sh after
   evals/promptfoo/score.sh compare
   ```

`before` 和 `after` 默认每项执行三次。缩小矩阵时，两次必须传入完全相同的筛选参数，
例如都使用 `--suite selection --batch 4`。

对比脚本以 Provider `label` 和测试 `description` 组成稳定键，输出模型平均总分、逐任务
总分、逐命名 metric 的 before/after/delta，以及 after 的失败任务原因。任一侧缺少记录、
出现重复记录或 metric 集不一致时会明确失败，避免比较不同矩阵。

最后运行 viewer，下钻失败任务的回答、断言和真实工具轨迹：

```bash
npx --yes promptfoo@0.122.0 view
```

### 按 metric 定位问题

| 现象 | 优先检查 |
| --- | --- |
| `expected_facts` 低 | 改进结果表达，确保回答从工具输出提取要求的事实，不以提及工具名代替事实。 |

## 覆盖运行参数

Provider 支持以下环境变量：

| 环境变量 | 默认值 | 用途 |
| --- | --- | --- |
| `VIBEWINDOW_EVAL_ENDPOINT` | `http://127.0.0.1:42617/v1/chat/stream` | 流式接口地址 |
| `VIBEWINDOW_EVAL_DIRECTORY` | `.`（VibeWindow 仓库根目录） | Agent 工作目录；相对路径基于仓库根目录 |
| `VIBEWINDOW_EVAL_MODEL` | `token-plan/deepseek-v4-flash-0731` | 被测模型 |
| `VIBEWINDOW_EVAL_AGENT` | `main` | 被测 Agent |
| `VIBEWINDOW_EVAL_TIMEOUT_MS` | `900000` | 单请求超时（15 分钟） |
| `VIBEWINDOW_PLATFORM_ID` | `10000` | 租户平台 ID |
| `VIBEWINDOW_COMPANY_ID` | `100000000` | 租户公司 ID |

例如：

```bash
VIBEWINDOW_EVAL_MODEL=provider/model \
npx --yes promptfoo@0.122.0 eval -c evals/promptfoo/promptfooconfig.yaml --no-cache -j 1
```

环境变量适合单模型临时排查；保存同场矩阵基线时应使用配置中的显式 Provider，避免所有
Provider 被同一个环境变量覆盖。

单个测试也可以通过 `vars.model` 或 `vars.agent` 覆盖对应值，优先级高于环境变量。

## 添加用例

直接在 `tests/` 下新增 YAML 或追加任务，主配置会通过 `tests/*.yaml`
自动加载。无需修改 `assertions.mjs` 或新建 Promptfoo config。为新增用例设置
稳定的 `metadata.batch`，即可只执行本批。RustCodeGraph 任务提供
稳定的 `expectedTerms`，评分器会自动选择对应规则：

```yaml
- description: 示例
  metadata:
    suite: rustcodegraph-selection
    batch: 4
  vars:
    task: 读取目标符号并回答，不要修改代码。
    expectedTerms: '["TargetSymbol", "target_file.rs"]'
    maxToolCalls: 6
```

只运行该批新增测试：

```bash
evals/promptfoo/score.sh run --batch 4
```

通用评分入口会根据 `metadata.suite` 和用例变量自动选择评测规则，
并在 Promptfoo 中保留各项 metric。RustCodeGraph 用例通过外部 CLI 技能导航；
CLI 不可用或未索引时允许普通搜索回退，不要求专用工具调用。用例检查：

- 流正常到达 `chat.done` 且回答非空。
- 没有失败、拒绝或取消的工具调用。
- 工具调用不超过用例的 `maxToolCalls`。
- 只读任务轨迹中没有明显写操作。
- 回答包含全部 `expectedTerms`。

coding 用例通过 `metadata.suite: coding` 启用通用写入轨迹评分，并在 YAML 中用
`writeRootPattern`（第一捕获组为写入根目录）、`requiredArtifactPattern`、
`excludedArtifactPattern` 和 `requireSuccessfulTest` 声明具体约束。未知写入目标、允许目录外写入、
缺少指定产物或没有成功的作用域内测试都会被确定性拒绝。
修复过程中失败但随后成功的测试尝试不影响评分；其他失败工具仍会被拒绝。

`skill-selection` 评分档案用 `requiredSuccessfulSkills` 声明必须成功调用的技能。
它会同时拒绝未调用、调用失败和其他失败工具，用于隔离验证会话选中技能的权限语义。

`metadata` 中保留 context 路径、step、工具调用、usage、模型和 Agent，便于在 Promptfoo viewer 中分析失败原因。
