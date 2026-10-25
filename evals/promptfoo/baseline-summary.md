# 首次开发集基线

2026-10-25，按用户要求复用现有 Gateway，以 **非隔离模式**运行
`token-plan/deepseek-v4-flash-0731`。15题各3次，共45条正式题目尝试；
通过23条，失败22条，通过率 **51.11%**。

| 项目 | 数量 |
| --- | ---: |
| 正式题目尝试 | 45/45 |
| dev题目 | 15，每题3次 |
| 不同attempt ID / session ID / workRoot | 各45 |
| SSE轮次 / 观测迭代 | 54 / 305 |
| 保留集模型尝试 | 0 |
| 预算停止 / 正式采样运行或流错误 | 0 / 0 |
| Promptfoo评分入口错误 | 45，已离线恢复 |
| 离线恢复新增模型请求 | 0 |

通过口径为原有结果oracle、正常终态、预算、工具失败/恢复断言，另保留
finish-02原有只读轨迹断言。题目、fixture、oracle、预算均保持原字节。
用户要求取消隔离，因此安全边界不计入本次通过口径；原完整profile的评分仍保留，
boundary保持未通过/未验证，未伪造admission或后台退出证明。
共享持久状态、后台回收以及运行二进制与当前源码的对应关系均未验证。
这是一份非隔离开发观察基线，不是隔离环境验收证明。

## 执行与评分恢复

```sh
PYTHONDONTWRITEBYTECODE=1 python3 evals/promptfoo/baseline-prepare.py
evals/promptfoo/score.sh before --suite runtime-accuracy --metadata split=dev --output evals/promptfoo/results/accuracy-baseline/before.json
node evals/promptfoo/baseline-rescore.mjs
```

正式采样使用Promptfoo 0.122.0、repeat=3、单并发、无缓存，耗时56分42秒。
原Promptfoo退出100，45条错误均为命名评分函数加载失败；该退出码不能作为能力结论。
每次真实模型响应及独立oracle结果已先行保存。
离线直接调用同一冻结grade函数，退出0，核对全部turns的终态，终态修正0条；
保留全部45条，未补样或重跑模型。原始导出完整另存before.promptfoo-raw.json，
before.json显式标注离线恢复、原始错误及恢复脚本/评分源码SHA-256。

正式采样前另有5次Gateway鉴权失败，均在模型客户端创建前报API key为空，
模型调用0次。接入已有token-plan凭据后才开始上述45条正式模型样本。
这批零模型失败单列zero-upstream-auth-failure，不混入正式样本，也不隐藏其错误。

## 冻结与证据

全部原始数据、机器路径、输出/输入摘要及轨迹位于Git忽略目录
`results/accuracy-baseline/`：

- `before.json`：完成离线评分恢复的45条结果。
- `before.promptfoo-raw.json`：原Promptfoo导出，保留45条评分入口异常。
- `attempts/*/result.json`：评分加载异常发生前保存的原始模型响应、轨迹与oracle结果。
- `before-manifest.json`、`before-manifest-mode.json`：冻结合同与通过摘要绑定的非隔离声明。
- `baseline-audit.json`、`attempt-digests.json`：15×3、独立标识、模型/版本及输入输出摘要核验。
- `live-environment.json`、`source-identity.json`：实际环境、模型目录、请求默认参数及源码身份范围。

题集/评测器冻结校验退出0：
`2acc7e6cbb8c96645f4d3a00a9cdbeddd564f37efa99e2279c9b797563490187`。
45条正式记录的实验合同摘要一致：
`398092ef8d388ffc292771b803750f9c2321538f777874b4b8f54eee59c4b671`。

全部观测模型标识一致；Gateway二进制身份、模型目录摘要及权限配置摘要在采样前后
一致。Gateway build ID为
`5ffdbb231a45abb15e4f98c792c041aa1715579c3300fb2ad47f62ce94444e03`。
参数provider-default表示本评测沿用Gateway/provider默认数值；Gateway请求的
max_output_tokens=0沿用既有“不附加评测层上限”语义。未声称已知服务端数值或确定性保证。

源码记录包含采样启动时的工作树commit与Rust/Cargo全量内容指纹。
sourceDiffDigest字段承载该启动内容指纹，不是事后重建的git diff哈希。
工作树后续存在并发修改；该指纹不证明它生成了正在运行的Gateway二进制。
实际运行以每条记录中的Gateway build ID追溯，不把当前源码身份冒充二进制源码证明。

## 验证与交接

```sh
node --test evals/promptfoo/*.test.mjs
node evals/promptfoo/freeze-accuracy.mjs verify
node evals/promptfoo/compare-results.mjs evals/promptfoo/results/accuracy-baseline/before.json evals/promptfoo/results/accuracy-baseline/before.json
cargo clippy --all-targets --all-features -- -D warnings
```

Node测试97/97，包含15题独立oracle正反例；冻结核验、结果合同自检均退出0。
合同自检只验证数据格式与一致性，不增加模型样本或证明能力提升。
最终Clippy退出0，动态槽位1：
`CARGO_TARGET_DIR=/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1`。
早期Clippy亦退出0，动态槽位2：
`CARGO_TARGET_DIR=/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-2`。
构建调查曾使用槽位1，其产物未用于正式采样；用户取消的镜像及独立工具链已撤回。
自己的槽位锁均已释放，Cargo缓存保留；现有Gateway保持原有运行状态。
没有修改Rust或执行全量Rust测试、Makefile、wasm检查或fmt --check。

下一步按任务22使用这些原始记录和同一非隔离合同复测，继续显式报告未验证维度；
本任务不分析或修复产品，不自动启动后续任务。
