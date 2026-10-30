# 任务 71: 准备-portfolio-optimization

批次：【批次 41】；依赖批次：1、2、3、4、5。同批无文件依赖；Git 提交按任务编号串行，禁止删除其他会话的索引锁。

状态：已完成，待回归验证

目的：为“投资组合计算加速”准备原题完整环境与四道独立诊断题。

来源任务：Terminal-Bench2.1/portfolio-optimization

预计会话范围：只准备一个父题的四个窄诊断题和确定性校准；不运行付费模型。长镜像拉取允许记录进程后续接，不能把未校准标成完成。

## 文件

- 新建：`evals/terminal-bench/suite/cases/portfolio-optimization/case.json`、`evals/terminal-bench/suite/cases/portfolio-optimization/check.py`、`evals/terminal-bench/suite/cases/portfolio-optimization/tests/test_cases.py`、`evals/terminal-bench/suite/cases/portfolio-optimization/fixtures/`、`evals/terminal-bench/suite/cases/portfolio-optimization/oracle/`、`evals/terminal-bench/suite/cases/portfolio-optimization/environment/Dockerfile`、`evals/terminal-bench/suite/cases/portfolio-optimization/README.md`
- 只读完整上游包：`evals/promptfoo/results/qwen38-suite/source/tasks/portfolio-optimization/`
- 忽略产物：`evals/promptfoo/results/qwen38-suite/preflight/portfolio-optimization/`、`evals/promptfoo/results/qwen38-suite/diagnostics/portfolio-optimization/`

## 上下文

原题 portfolio-optimization；类别 optimization；CPU=1、内存=4096MiB、存储=10240MiB、GPU=0；Agent=3600秒、verifier=3600秒。预建镜像 alexgshaw/portfolio-optimization:20251031。原题树SHA=9fd52796e147d886d6c25c5c4fb36cfb28c7d397；题面SHA256=ffaadcbf9f576b5d5e087d2a06a4b525dd52c6b84d43eb9d85107f0a9aebcb5d；配置SHA256=4918687e4e261a9852e39e5a554156733c071c17dc89c5f544f15f2ef744ce0a。读完整 instruction.md、输入、solution、tests 后设计，禁止改原题。四子题：m1 解析矩阵与权重布局；m2 实现小型风险和收益内核；m3 验证精度和内存边界；m4 测量并核验扩展入口。

不得修改旧评测、默认 Promptfoo 配置或 accuracy-manifest.mjs 的20例基线。不得用旧五题或旧 Qwen 两题成绩填充本轮。保持 token-plan/qwen3.8-flash、单次输出32000、实验输入窗口200000；不自动换模型、加预算、重复付费尝试。完整题保留官方题面、数据、参考解、评分、资源和各自超时。每例只运行一轮，整题、诊断、准入、校准与基础设施失败分栏；终态正常不等于官方评分通过。密钥只通过临时0600配置注入，禁止进入命令参数、日志、镜像或提交。

## Cargo 共享槽位规则

1. 同一仓库的所有计划和任务共用仓库根目录下的 `target/rust-slot-1` 至 `target/rust-slot-10`，不得为任务、会话或计划另建构建目录，也不得按任务编号取模固定分配。
2. 每次构建/验证阶段开始前，创建仓库根目录下的 `target/rust-slot-locks`；依次用原子命令 `mkdir target/rust-slot-locks/slot-N.lock` 尝试领取槽位，`N` 为 1–10。仅 `mkdir` 成功者获得该槽位；这些相对路径命令从仓库根目录执行。
3. 领取成功后，在自己的锁目录中记录计划、任务、会话标识和持有进程 PID。同一槽位同时只允许一个持有者；已占用时尝试下一个，仅全部 10 个槽位都占用时等待并重试。
4. 将 `CARGO_TARGET_DIR` 设置为领取槽位的绝对路径，例如领取槽位 3 后，从仓库根目录执行 `export CARGO_TARGET_DIR="$PWD/target/rust-slot-3"`。示例编号不代表固定分配。
5. 所有 Cargo 构建、检查、测试以及调用 Cargo 的脚本和子进程均继承该设置，不得通过 `--target-dir` 或其他配置绕过槽位；生成的二进制也从该槽位定位。
6. 该阶段的 Cargo、脚本以及使用槽位产物的子进程全部结束后，删除自己写入的占用记录，并用 `rmdir` 释放自己的锁目录；保留槽位编译缓存供后续任务复用。
7. 正常结束、失败和中断均须清理自己的锁；在后台进程或其他使用槽位产物的进程仍运行时不得提前释放。规划清理步骤时覆盖这三种退出路径。
8. 遗留锁须先核实持有会话及相关进程已结束；无法确认则保持占用，不得删除其他活跃任务的锁、进程或缓存。
9. 生成或更新计划时，统一移除与这些规则冲突的单一 Rust 共享构建空间、逐任务独立目录和 Cargo 缓存全局独占窗口要求；仍须遵守源码文件占用、真实业务依赖和其他共享验证资源的互斥，不能仅因缓存分槽就把任务判定为可并行。
10. 最终验证记录包含实际槽位、绝对路径 `CARGO_TARGET_DIR`、确切命令、退出码和有效测试数量；Rust 代码修改后先运行 `cargo fmt --all`，再运行相应验证。编译成功或零测试不能替代行为验收证据。

仓库另设 build-dir：领取槽位后同时设置 `export CARGO_BUILD_BUILD_DIR="$CARGO_TARGET_DIR"`、`export CARGO_BUILD_JOBS=1`，防止 `.cargo/config.toml` 写入共享 target。纯 Python 改动无需格式化 Rust；若确有 Rust 修改先运行 `cargo fmt --all`。代码任务结束执行 `cargo clippy --all-targets --all-features -- -D warnings`；既有无关失败只记录证据，不修无关模块。禁止全量 cargo test、fmt --check、wasm 检查及 Makefile 命令。

## 测试与验证计划

先使用技能 ./skills/create-agent-evals/SKILL.md；每个子题定义合成输入、清楚完整题面、输出路径/API/服务行为与独立确定性判定。新增 evals/terminal-bench/suite/cases/portfolio-optimization/tests/test_cases.py：正解通过，至少3个有意义错误变体被拒绝；运行真实程序/产物，不验证关键词、自述或仅Agent自测。错误fixture和漏检checker先失败，再最小修复。

验证命令：

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/portfolio-optimization/tests -p test_cases.py
```

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/portfolio-optimization/case.json --run-fixtures
```

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.run preflight --catalog evals/terminal-bench/suite/catalog.json --runtime evals/promptfoo/results/qwen38-suite/runtime.json --parent-id portfolio-optimization --output evals/promptfoo/results/qwen38-suite/preflight/portfolio-optimization
```

## 步骤

1. 读取本题固定完整上游材料；索引仅导航，不能代替完整题面。
2. 逐一写m1..m4的合成独立题面、输入和黑盒判定；每题能单独理解，不能用父题参考解前缀或旧模型轨迹。
3. 写gold和至少3种错误实现，先证明checker缺陷的失败再修复；合成素材可提交，上游隐藏材料只留忽略目录。
4. 按task2策略受限启动原镜像，验证oracle/nop；四子题fresh环境各自校准。必要依赖修复限verifier-only且重新校准，不改官方checker，服务题不得伪造artifact回放。
5. 记录镜像、哈希、时间与校准收据；同批只占自己目录、最多2题合计4CPU/8192MiB；8GiB独占。共享builder修复必须串行。
6. 执行聚焦测试和Clippy门槛，按任务序号串行提交自己资产。

## 完成证据

完整原题oracle通过、nop失败；4子题各有oracle/nop与3负例；记录镜像digest、依赖/fixture/checker哈希、隔离与资源收据。不能校准则阻塞本题，不宣称已就绪。

仅有编译通过、零测试或历史结果不足以完成。有代码、测试、配置或文档实际变更时使用 `$git-commit`，只提交本任务产物；完成后报告新证据并删除本编号文件。无关原因阻断验证时记为 `已完成，待回归` 并保留文件；确实无法突破时记为 `已阻塞`，只在本文件记录原因、检查与恢复入口。`plan.md` 只读。


## 2026-10-28 host 检查点（未就绪、未提交）

- 主调度确认依赖批次 1–5 已满足。已完整阅读固定上游 15 个文件，并逐文件对照 catalog SHA256，全部一致；原题和 plan.md 未修改。
- 已使用 do-task-plans、create-agent-evals 技能；只在本题目录新增 m1 布局、m2 数值内核、m3 精度/形状/非有限边界、m4 真 CPython C 扩展题面、合成输入、独立 checker、gold 和各 3 个语义负例。
- 初始宽松 checker 的真实红灯：`red.log`，退出 1，18 个失败（含缺失真实计时收据）；修复后四 gold=0、四 nop=1、十二负例=1。不是 Agent 自测、文本关键词或预写计时结果。m4 独立编译 native.c，要求真实 ExtensionFileLoader/native callable，checker 实测三次调用并核验与独立 Python baseline 的加速。
- 聚焦命令：`PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/portfolio-optimization/tests -p test_cases.py`，退出 0；1 个 unittest 方法覆盖 20 个实际进程控制。末轮 `host-tests.log`：2.423 秒。`host-receipt.json` 另存全部 20 个实际退出码、编译错误、真实 timing、声明文件哈希和父题 15 文件哈希；m4 gold baseline=0.002715042035561055 秒/native=0.00021720799850299954 秒，12.499733224711619x，仅 host 证据，不替代 Linux/Harbor 校准。
- 标准 materialize 静态检查通过：四个 environment 只含 Dockerfile 和 fixtures/example.json，无隐藏材料。case load/schema/hash 检查通过，未创建 fixture validation.json 或 preparation.json。
- `evals/promptfoo/score.sh test` 退出 0，Configuration is valid；保留 Promptfoo 版本提示和 telemetry shutdown timeout 日志，不更新旧基线或默认配置。
- `cargo clippy --all-targets --all-features -- -D warnings` 退出 0。动态领取共享槽位 **1**；`CARGO_TARGET_DIR=/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1`，`CARGO_BUILD_BUILD_DIR` 同值，`CARGO_BUILD_JOBS=1`。Clippy 行为测试数 0，不能代替上述 20 控制；自有 owner/锁已释放、缓存保留。只有既有 block 0.1.6 future-compatibility 提示；无 Rust 源码改动，未运行全量 test/fmt check/wasm/Makefile。
- read-only Docker inspect 确认 task52 cached generic tool 的真实 Docker v2 leaf `32c8a19cdeab820f8ea710fcdacaef0893848898158f0d3ccc890a75ab365adf` 为 linux/amd64，Config/RootFS 与既有通用工具证明一致，config digest `a6c16930afacbb61ef2e9f1e5f994d6c560b787fa4c5f849b5c15b3f017d111c`。只复用元数据，未拉取/构建/运行容器。未来需 fresh clean-filesystem、gcc/Python headers 和冻结 amd64 Agent 的动态库闭包检查；历史 image-probe 不作本题 fresh 通过。
- 证据目录：`evals/promptfoo/results/qwen38-suite/diagnostics/portfolio-optimization/`，含 red.log、host-tests.log、host-receipt.json、promptfoo-test.log、clippy.log、clippy-receipt.txt、cached-image-inspect.json、cached-tool-review.json。

当时阻塞原因（后续诊断窗口已获准）：主调度指定 68 独占重阶段，71 暂停一切 Docker builder/tool image/fixture/standard Harbor/父 body 下载与校准，待主调度窗口。此为明确共享执行资源约束，不是能力得分或已完成，当前不提交、不删除本文件。host 无 NumPy（ModuleNotFoundError），本题独立合成 CPython C API 诊断不依赖它；父题仍需原 Python 3.13/NumPy 2.3.2 完整环境，不在 host 伪造官方结果。

恢复入口：窗口获准后，先重新核验 min(host/VM) 空间，按原 S/B/A/5GiB 与所有压缩 C 副本及完整 archive U（未知非零，inspect Size/ISIZE 不算完整 U，或明确正硬停止 cap）准入；parent S=10GiB 与 diag S=1GiB 分栏，cached reuse extra transfer=0 独列。保持原父资源/3600 秒/评分/模型预算。通过公共 `suite.scoring.harbor_trial -> SuiteDockerEnvironment`，RUN 前限额、up --no-build、不用私有 Trial/compose 或 DOCKER_DEFAULT_PLATFORM。先补 clean Linux/Agent 依赖检查及 `suite.diagnostics validate --case .../case.json --run-fixtures` 20 控制，再以 ignored executionconfig 引用 verified runtime-v2 做原父 oracle/nop 和四 fresh 子题 oracle/nop；不要使用缺 historical_root 的历史 runtime.json report。无 fresh 父/子证据前不得标 ready/提交/删文件。无付费调用。


## 2026-10-28 Linux 诊断窗口检查点（父题容量仍阻塞）

状态：已阻塞

- 主调度已释放 70 窗口并授权 71 独占容器/镜像诊断阶段。旧的等待调度原因已解除；当前唯一阻塞是原父完整峰值容量不足。未提交、未删除任务文件。
- 诊断启动前 3 次真实 host/VM 采样最小可用 11,792,486,400 字节；诊断 S1+B2+A1+安全5GiB=9,663,676,416 字节，加 metadata 134,217,728 字节，共 9,797,894,144 字节，准入通过。完整已缓存纯工具 leaf 精确复用，新增传输 C 副本和 U 副本均 0；未将未知 archive U 当作 0 或用 inspect Size/ISIZE 作为上界，无新增工具 build/pull/import。原始样本见 diagnostic-capacity-admission.json。
- 真实缓存工具名称 `vw-log-summary-diagnostics:task52-single`，case 保持 true leaf `sha256:32c8a19cdeab820f8ea710fcdacaef0893848898158f0d3ccc890a75ab365adf`，Docker v2 manifest，config digest `sha256:a6c16930afacbb61ef2e9f1e5f994d6c560b787fa4c5f849b5c15b3f017d111c`，Config/RootFS 对照一致。实际容器 x86_64、Python3.13.7/gcc12.2.0/真实 Python headers；/app 空，/tests、/solution、/oracle 不存在。四合成题明确无需 NumPy，未以 2.3.1 或其他共享版本替代原父 2.3.2。无 DOCKER_DEFAULT_PLATFORM、私有平台/ref/CMD/Trial/Compose 覆盖。
- `PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/portfolio-optimization/case.json --run-fixtures` 退出 0，20/20 真 Linux 控制通过：4 gold=0、4 nop=1、12 语义负例=1。case/validation.json 绑定全部声明文件和公共 checker 实现哈希；linux-validation.log 保留原始输出。
- ignored `calibrate-children.py` 仅调用标准公共 `suite.scoring.harbor_trial -> SuiteDockerEnvironment`，退出 0。四个 fresh 子题各 oracle=1/nop=0，共 8/8 有效 first-score，无异常/超时/评分回放。当前执行身份 `ce05045daeb0b6f9a28b5cd179f2f4363cfbbaf143a099565e27218a494a4bd1` 起止一致；官方源、模型、预算、Harbor 均未修改。runtime.json 是本题 ignored executionconfig，引用 verified runtime-v2，保留父 agentTimeoutSec=3600。
- 8 个主容器创建即 1 CPU/512MiB，8 个 sidecar 创建即 0.25 CPU/64MiB；公共 builder 与 admission 原始日志齐全，up 不隐式 build。真实运行镜像为 amd64，RootFS 均继承精确 cached leaf。每份 source/materialized task、first-score/scoring 文件哈希及资源 inspect 都由 children-calibration-production-leaf.json 绑定，未自建 Trial 包装。
- m4 标准 oracle 的 checker 在独立进程真实编译 CPython 扩展并计时：baseline=0.007286666979780421 秒、native=0.0005118749977555126 秒、14.235246909365085x；原始 verifier/test-stdout.txt 保留。不是 Agent 自述耗时。
- `probe-agent.py` 退出 0：冻结 x86_64 ELF SHA256 `847823c5cd99a6828cddfc2e3e5dbb67c5ddcc9f8693a13d539d8a8fa47c9dae` 与 runtime-v2 校验一致，所有声明 sidecar 哈希一致。真实 linux/amd64 工具容器，network=none/read-only/0.25CPU/256MiB，按只读 ELF 所有者运行，ldd 与无密钥/无模型配置 --help 成功，进程 exit=0、无 OOM；未装 Agent 依赖。
- finalize-linux-checkpoint.py 退出 0，复核 20+8 控制、所有任务/scoring 哈希、资源限额、冻结 ELF 身份与真实计时。仅清理本题已停止容器及空网络/卷；保留全部唯一 OCI、缓存、材料与原始证据。最终 owned containers/networks/volumes 均为空；付费调用 0。
- 本轮无业务/checker/fixture 实现修改，只新增真实校准收据与文档；复用本任务已通过的 Clippy：动态槽 1，绝对 `CARGO_TARGET_DIR=CARGO_BUILD_BUILD_DIR=/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1`，JOBS=1，退出 0，Clippy 行为测试数 0，自有锁已释放。未运行禁止的全量验证。

当前父题阻塞数字：清理后再次 3 次真实 host/VM 采样，min(host,VM)=12,366,946,304 字节；父题原 S10+B2+A1+安全5GiB+128MiB metadata 底线为 19,461,570,560 字节，未计镜像 C/U 所有副本已差 7,094,624,256 字节。父题 C、完整 U、正 Ucap 尚未知，不填 0；基础底线已失败，未下载任何父 layer body、未进口原父、未运行父 oracle/nop。原父 Agent ABI 与官方 verifier 的 Python3.13/NumPy2.3.2/uv 及网络/bootstrap 边界均待验证；未预修 Agent 或替换官方评分。当前不生成 preparation.json、不宣称整题 ready。

恢复：维护者释放至少上述缺口并预留真实 C/U 所有副本/完整峰值后，重新三次 min(host,VM) 准入（如采用正硬停止 cap，须原始全 stream SHA/diff_id/U 验证）；再按原 S10/1CPU/4GiB/3600 秒运行标准原父 oracle/nop，必要 verifier-only 修复须保留官方 checker 并重新校准。不能只凭诊断通过完成/提交/删除。本次最终证据：`evals/promptfoo/results/qwen38-suite/diagnostics/portfolio-optimization/checkpoint-linux-calibrated-parent-blocked.json`。


## 2026-10-31 续跑检查点（原环境阻塞已解除，待 Clippy 回归）

- 已完成本题准备与确定性校准；唯一剩余门槛是下述无关 Rust 编译错误。保留本任务文件，未修改 plan.md、原题、旧评测、模型或预算，无付费调用。
- 原父真实 Docker v2 amd64 leaf `sha256:93461db3cec697c31585941a3b9387c1398495aa1ff0aee2ac752c7230b50ef8`，config `sha256:b0ef263fdd85ca609faafead645c86c8a03f534178b3368e3eedc0a273b3fed9`。12 层完整压缩 SHA256 与解压 diff_id 全部通过；C=196450772、完整 U=592377344 字节，非 inspect Size/ISIZE 推断。以正 2GiB Ucap、2C、S10+B2+A1+安全5GiB+128MiB metadata 及三次 min(host,VM) 准入后流式 OCI 导入，无额外 archive 文件；Config/RootFS/实际架构一致。
- 首轮容量充足后，被其他并行工作消耗；父 nop 在创建容器前取消，子题后续触发独立 S1+B2+A1+安全5GiB+128MiB=9797894144 字节守卫。未把基础设施中止记作能力失败。空间恢复后，三次重新采样 min(host,VM)=56239910912 字节，缓存复用新增 C/U 均为 0。另一次公共 4CPU/8GiB 准入拒绝完整保留；之后等待容量，未放宽限制。
- 全部沿公共 `suite.scoring.harbor_trial -> SuiteDockerEnvironment` 执行。原父 oracle 官方 reward=1，6/6 官方测试通过（154.29 秒，包含 5000/6000/8000 资产）；nop 官方 reward=0，1 基线通过、5 实现失败（27.17 秒）。4 子题各 fresh oracle=1/nop=0，共 10/10 有效 first-score、无评分恢复或结果回放。先核验已通过 oracle 的原始评分和文件哈希，未重复执行该正解。
- 主容器创建即 1CPU，父 4096MiB、子 512MiB，swap 与 memory 一致；子题 sidecar 创建即 0.25CPU/64MiB。镜像实际均 amd64；父 RootFS 精确一致，子 RootFS 继承声明的 cached leaf。RUN 前受限、up --no-build；父 S10 与子 S1 独立保留。原父 3600 秒、官方 checker 与完整 15 文件源哈希均未改。
- host 聚焦 unittest 退出 0：1 个方法覆盖 20 个真实进程；`suite.diagnostics validate --case .../case.json --run-fixtures` 退出 0：20/20 Linux 控制（4 gold=0、4 nop=1、12 语义负例=1）。现行 validation.json 绑定公共实现与声明文件。m4 新标准 oracle 实测 baseline=0.008099916998617118、native=0.00052829199921689 秒，加速 15.332272702641673x。
- `calibrate-admitted.py` 退出 0，公共 verify_preparation 对 ignored 和 case 两份完整 preparation.json 均通过。现行执行身份 `588690e423e955cab7b2c335d302678cae81598b2ec421946065c88f964ce16c` 起止一致；旧 ce05045... 子题证据仅保留为历史，未冒充现行结果。完整本机 preparation.json 含绝对路径，不进入资产提交。
- 原父 Python3.13.7/NumPy2.3.2/setuptools78.1.1/gcc/headers 实测通过；冻结 ELF `847823c5cd99a6828cddfc2e3e5dbb67c5ddcc9f8693a13d539d8a8fa47c9dae` 及全部声明 sidecar 在原父中离线只读、ELF 所有者 --help 通过。Python HTTP uv 安装器探测曾 403，但原官方 curl 流程成功安装 uv0.9.5 并执行全部测试；未启用 verifier bootstrap，未安装 Agent 依赖，未改官方测试。
- `evals/promptfoo/score.sh test` 退出 0。无 Rust 源码修改，未运行全量 cargo test、fmt check、wasm 或 Makefile。
- 必需 `cargo clippy --all-targets --all-features -- -D warnings` 首轮槽 1 退出 101：共享槽的 serde rmeta 消失，原锁也在运行期间消失；现有新锁不属于本会话，未动它。重新动态领取槽 **4**，`CARGO_TARGET_DIR=CARGO_BUILD_BUILD_DIR=/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-4`，`CARGO_BUILD_JOBS=1`，遇到既有并行改动 `crates/vw-agent/src/session/processor/runner_tests.rs:2416` 的 E0425：`super::llm_runner::TestLlmMessages` 不存在。Clippy 行为测试数 0，不替代 30 项校准；依仓库规则不修无关 Rust 问题。
- 当前证据目录：`evals/promptfoo/results/qwen38-suite/diagnostics/portfolio-optimization/resume-20261030/`。保留原始容量中断、共享准入拒绝、Clippy 两轮日志、全部 first-score/官方 stdout、层校验、资源 inspect、runtime/fixture/checker 身份；最终核验见 `final-checkpoint.json`。所有本题容器/网络/卷均按归属检查；不清理业务、共享缓存或其他会话资源。

恢复入口：由相关会话修复上述 Rust 测试类型引用后，重新按共享槽位规则执行完整 Clippy 门槛；通过后报告新证据并删除本编号文件。若公共执行身份改变，须重新 inputs 校验并补受影响的 Harbor 校准，不能修改历史评分或替换旧 hash。当前允许认定本题环境与 4 子题校准已准备完成，不能声称本次 Clippy 已通过。
