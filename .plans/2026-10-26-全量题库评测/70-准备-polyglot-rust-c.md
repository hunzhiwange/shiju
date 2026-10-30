# 任务 70: 准备-polyglot-rust-c

批次：【批次 40】；依赖批次：1、2、3、4、5。同批无文件依赖；Git 提交按任务编号串行，禁止删除其他会话的索引锁。

状态：已阻塞

阻塞说明：单题串行容量现已满足；当前任务47占用窗口，两题联合准入仍不足。旧Linux20控制有效，调度器身份变化，父题及8项正式控制待新窗口校准。

目的：为“双编译器公共源码”准备原题完整环境与四道独立诊断题。

来源任务：Terminal-Bench2.1/polyglot-rust-c

预计会话范围：只准备一个父题的四个窄诊断题和确定性校准；不运行付费模型。长镜像拉取允许记录进程后续接，不能把未校准标成完成。

## 文件

- 新建：`evals/terminal-bench/suite/cases/polyglot-rust-c/case.json`、`evals/terminal-bench/suite/cases/polyglot-rust-c/check.py`、`evals/terminal-bench/suite/cases/polyglot-rust-c/tests/test_cases.py`、`evals/terminal-bench/suite/cases/polyglot-rust-c/fixtures/`、`evals/terminal-bench/suite/cases/polyglot-rust-c/oracle/`、`evals/terminal-bench/suite/cases/polyglot-rust-c/environment/Dockerfile`、`evals/terminal-bench/suite/cases/polyglot-rust-c/README.md`
- 只读完整上游包：`evals/promptfoo/results/qwen38-suite/source/tasks/polyglot-rust-c/`
- 忽略产物：`evals/promptfoo/results/qwen38-suite/preflight/polyglot-rust-c/`、`evals/promptfoo/results/qwen38-suite/diagnostics/polyglot-rust-c/`

## 上下文

原题 polyglot-rust-c；类别 software-engineering；CPU=1、内存=2048MiB、存储=10240MiB、GPU=0；Agent=900秒、verifier=900秒。预建镜像 alexgshaw/polyglot-rust-c:20251031。原题树SHA=a80cca48c040300496622d1a237b0283b5e40699；题面SHA256=b0fbcd99a97fc886e4e198b6189b977409dada47cae0a87ac7dc185b440b2355；配置SHA256=3c41e83437068c62a3fdff74641899146e34907aefd9e47346213f47caeb82de。读完整 instruction.md、输入、solution、tests 后设计，禁止改原题。四子题：m1 找到两种编译器的公共语法；m2 实现共享小型算法；m3 覆盖类型与参数边界；m4 编译比较两个入口。

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

先使用技能 ./skills/create-agent-evals/SKILL.md；每个子题定义合成输入、清楚完整题面、输出路径/API/服务行为与独立确定性判定。新增 evals/terminal-bench/suite/cases/polyglot-rust-c/tests/test_cases.py：正解通过，至少3个有意义错误变体被拒绝；运行真实程序/产物，不验证关键词、自述或仅Agent自测。错误fixture和漏检checker先失败，再最小修复。

验证命令：

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/polyglot-rust-c/tests -p test_cases.py
```

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/polyglot-rust-c/case.json --run-fixtures
```

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.run preflight --catalog evals/terminal-bench/suite/catalog.json --runtime evals/promptfoo/results/qwen38-suite/runtime.json --parent-id polyglot-rust-c --output evals/promptfoo/results/qwen38-suite/preflight/polyglot-rust-c
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

## 当前检查点（2026-10-28）

依赖批次1–5已由主调度确认满足。完整上游8文件SHA-256与catalog一致。已新增四道独立Rust/C++题面、真实双编译黑盒checker、4 gold/4 nop/12语义负例；指定宿主unittest退出0（20控制），stdout漏检变体导致11断言失败后恢复通过。score.sh test退出0；Clippy退出0，动态槽位1，CARGO_TARGET_DIR=CARGO_BUILD_BUILD_DIR=/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1，JOBS=1，自有锁已释放且缓存保留。

主调度要求暂停所有Docker build/fixture/Harbor/父下载及校准直至主会话新窗口通知；当前未启动上述动作。官方通用Rust候选单amd64 manifest bf7d87666c4da6eace19e06d21bc4859c6e2a5c97a21ac273b0e082112753cf0，C=541938170；父单amd64 manifest 3ba84f0e138a4b65e58c53f920a0fc852571e6402901c3bdbb876e9dab6e6c5e，C=271241474。仅取registry manifest/config元数据，层body下载0；U/Ucap未知，不能当0或声称容量准入。实际Linux工具、ELF兼容、单manifest config/RootFS与隐藏材料核查待窗口；原父与诊断容器控制均0，未标就绪、未提交。证据位于ignored diagnostics/polyglot-rust-c/。

### 阻塞与恢复入口

本机未找到已缓存的独立通用Rust工具镜像；官方rust候选仅manifest/config核验，未做层body下载、导入或容器工具实测。其完整tar U与可用于当前磁盘预算的硬Ucap尚未得到；不能使用inspect Size替代、不能假定镜像Python/rustc/ELF兼容已通过。主调度明确要求等待主会话新窗口通知，故未运行 `suite.diagnostics validate --case evals/terminal-bench/suite/cases/polyglot-rust-c/case.json --run-fixtures` 或原任务的 `suite.run preflight ... --parent-id polyglot-rust-c`；这两项无退出码、有效容器测试数均0。未获得原父oracle/nop或4子题fresh校准收据，不得ready或删除编号文件。

恢复时先取得窗口，对工具候选/父镜像分别做真实min(host,VM)、C、硬停止cap/fulltar U、allcopies、原S/B/A及5GiB保留联合准入；再在允许窗口内以preset限额准备真实通用工具环境并验证Python3/rustc/g++/冻结Agent、真单manifest config和RootFS及无隐藏材料，必要时仅调整本case候选工具镜像。运行指定20fixture容器校准、8个公共harbor_trial标准oracle/nop、原父完整preflight及oracle/nop后再决定完成。当前case schema检查退出0（4子题、30声明项）；host20控制与Clippy均exit0，不替代这些缺失检查。无付费调用，无公共代码/Harbor/原题/plan.md修改；实际资产未提交，待证据完整后按编号串行git-commit。

### 调度说明更正（2026-10-28）

当前68已安全准入并进行父下载/校准，69处于inactive状态、待68之后调度。70维持Docker暂停，直到主会话通知新窗口；不以69尚未开始的父重阶段作为自行等待条件。本次仅规范状态机与调度说明，宿主机20个真实Rust/C++双编译控制、Clippy槽位1证据及全部候选工具metadata保持不变；未运行Docker、新验证或Clippy，未覆盖旧raw/hash，未提交。

### Linux诊断恢复结果（2026-10-28，取代先前工具/窗口阻塞说明）

主调度授予本题唯一容器/镜像窗口，禁止原父下载进口。三次新采样最小12766584832字节，诊断S1+B2+A1+安全5GiB+metadata+4C+3Ucap要求11765066928字节，准入通过。基座复用已缓存纯工具真单amd64 leaf32c8a19c，无新base导入。官方rustc/rust-std 1.75.0 x86_64发行包，合计C=89140012，下载SHA-256与完整解压流校验，实际tar U=472209408 < 硬停止cap512MiB；来源、SHA、许可与安装配方保存于本case environment/及ignored linux-tooling/。未使用Cargo安装、虚假编译器、Agent/上游stage或平台覆盖。

受限classic构建RUN实测CpuPeriod=CpuQuota=100000，Memory=MemorySwap=536870912。产物真单amd64 OCI manifest=20a0f6d71e716c237e1ccc84c243548d75eac2ded6395004217ff55da77124d9，无重新包装；全部layer compressed SHA/diff_id流式核验，整图fulltar U=1471391744（包括复用基座及重复空层），声明manifest/config/RootFS架构一致，inspect仅补零默认字段。初始metadata辅助脚本因空层/inspect默认字段断言失败的日志保留，最终完整流复核另存；没有用inspect Size代替U或覆盖旧raw/hash。

Linux Python3.13.7、g++12.2.0、rustc1.75.0真实编译执行及冻结x86_64 ELF --help退出0。指定 `suite.diagnostics validate --case evals/terminal-bench/suite/cases/polyglot-rust-c/case.json --run-fixtures` 退出0，有效20控制（4gold/4nop/12语义负例）。公共 `suite.scoring.harbor_trial` -> SuiteDockerEnvironment 8个正式控制全部首评分oracle=1/nop=0，执行身份ce05045daeb0b6f9a28b5cd179f2f4363cfbbaf143a099565e27218a494a4bd1，main1CPU512MiB/sidecar0.25CPU64MiB，题字节与身份前后一致。没有私有Trial/Compose/platform包装、付费模型或评分/预算变更。最新指定host unittest退出0（20控制）；最新Clippy退出0，动态槽1，CARGO_TARGET_DIR=CARGO_BUILD_BUILD_DIR=/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1，JOBS=1，锁释放且缓存保留。无全量cargo test/fmtcheck/wasm/Makefile。

最终三次实测min(host,VM)=11813294080字节；父S10+B2+A1+安全5GiB+128MiB metadata最小19461570560，尚缺7648276480字节，且未计父镜像副本。父C=271241474，U/Ucap仍未知，父body下载/进口/控制均0；原父preflight因明确容量不足未运行，无退出码，不冒充通过。当前精确已阻塞，仅父容量缺口；保留编号文件和未提交资产。自有容器/网络/卷均清空，保留唯一工具镜像与发行包/原始证据/依赖。恢复入口：主调度新窗口+新父容量准入后运行原任务指定parent preflight/oracle/nop；诊断已绑定当前身份的20+8证据可先核对复用，不无故重跑。最终ignored收据：checkpoint-linux-calibrated-parent-blocked.json、parent-capacity-linux-final.json、children-calibration-production-leaf.json、linux-tooling/full-stream-verified.json。


### 继续突破结果（2026-10-30，取代此前“仅父容量缺口”结论）

本轮按用户继续指令重新取样，未清理、停止或改变其他任务/业务资源。先后观察到 gcode-to-text、password-recovery、path-tracing 等准备容器；最终仍有 `vw-task47-x86-reservation`（ID `3503ca2fe97b527ad9dd2ed8649a34d67cb81891036a792ad94203b3e78fa52b`，2CPU/4294967296字节内存）及对应活跃Lima/QEMU作业。自有准入等待进程已停止，无遗留下载、构建或自动续跑进程；其他资源未改动。

本次采用不落盘OCI归档/完整分片的流式导入配方，显式计入宿主blob、daemon incoming/store三份C，以及daemon解压暂存/最终snapshot两份Ucap，未扣共享层。父C仍为271241474，正数硬停止Ucap=1073741824（未经完整流核验不得当实际U）；每个父题保持S10+B2+A1，联合安全余量5GiB，每题metadata128MiB。单题要求22422778630字节；两父题要求36515640070字节。最终三次min(host,VM)=25288339456字节（VM71611011072），单题串行已可准入；与任务47并行仍缺11227300614字节。因此原先“本机连单题底线也不足”的阻塞已经解除，剩余为可用串行窗口/联合容量约束；不得通过忽略任务47官方存储、停止其作业或降低原题资源绕过。

原父manifest/config重新从Registry只读核验：manifest `3ba84f0e138a4b65e58c53f920a0fc852571e6402901c3bdbb876e9dab6e6c5e`、config `42e1c101034156ab6a0cbefc63ee654ab7fccb668e1580a090abba89244f6a93`，linux/amd64及RootFS声明一致。令牌只在内存；父层body下载/导入/容器控制均0，实际U未知。原父preflight未启动、退出码null、有效父控制0，未冒充校准成功。

指定宿主unittest首次退出1，4个gold失败；定位为rustup仅配置仓库stable覆盖、未配置临时目录默认工具链。直接复现得到 `rustup could not choose a version of rustc`，不是checker或fixture语义退化。未改全局rustup或checker；仅将已安装真实编译器目录加到PATH后，原指定命令退出0，1个unittest方法包含20个真实控制（4gold/4nop/12语义负例）。完整命令前缀：`PATH="/Users/xiangmin/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH" PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/polyglot-rust-c/tests -p test_cases.py`。当前rustc1.99.1/Apple clang21，不能替代父题编译器或Linux校准。

最新 `cargo clippy --all-targets --all-features -- -D warnings` 退出0，动态槽位1；`CARGO_TARGET_DIR=CARGO_BUILD_BUILD_DIR=/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1`，JOBS=1，正常完成后仅释放自有锁、保留缓存。lint有效行为测试数0，行为证据来自上述20控制；无Rust源码修改、无全量cargo test/fmtcheck/wasm/Makefile、无付费调用。

公共 `suite.preparation.inputs` 校验退出0；冻结上游、runtime、Harbor、诊断校准收据等与旧身份相比仅 `scheduler_sha256` 改变，本轮inputs采样时为 `5c0763881dd3de0cf83385c6f67a62ddb6fba2ebad143053602ea65113b507d5`。所以旧Linux20fixture字节证据可复核保留，但旧8项正式控制不能直接充作当前调度器的完成收据。恢复时先重新实测当前空间、占用与CPU/内存；取得串行窗口后按本次硬cap配方校验完整层SHA/diff_id/fulltar U并流式导入原digest，再受限探测真实工具/网络/隐藏材料隔离，运行原计划 `suite.run preflight ... --parent-id polyglot-rust-c`，以当前公共入口重新取得父oracle/nop和8个子题正式控制。不私改共享调度器、不合成preparation.json、不重跑付费模型。

新证据均在ignored `diagnostics/polyglot-rust-c/resume-20261030/`：`metadata-receipt.json`、`parent-manifest.json`、`parent-config.json`、`current-inputs.json`、`host-tests.log`、`clippy-receipt.txt`、`clippy.log`、`capacity-final.json`、`other-reservation-readonly.json`、`import-*.log`及可续接 `import-parent.py`。其中早期双题保守预留被拒绝和等待记录保留，未覆盖旧证据。当前继续已阻塞，保留编号文件及尚未就绪的诊断资产；本次仅更新本编号阻塞记录，不提交未校准资产。
