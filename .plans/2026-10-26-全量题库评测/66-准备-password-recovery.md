# 任务 66: 准备-password-recovery

批次：【批次 38】；依赖批次：1、2、3、4、5。同批无文件依赖；Git 提交按任务编号串行，禁止删除其他会话的索引锁。

状态：已阻塞

目的：为“删除文件取证”准备原题完整环境与四道独立诊断题。

来源任务：Terminal-Bench2.1/password-recovery

预计会话范围：只准备一个父题的四个窄诊断题和确定性校准；不运行付费模型。长镜像拉取允许记录进程后续接，不能把未校准标成完成。

## 文件

- 新建：`evals/terminal-bench/suite/cases/password-recovery/case.json`、`evals/terminal-bench/suite/cases/password-recovery/check.py`、`evals/terminal-bench/suite/cases/password-recovery/tests/test_cases.py`、`evals/terminal-bench/suite/cases/password-recovery/fixtures/`、`evals/terminal-bench/suite/cases/password-recovery/oracle/`、`evals/terminal-bench/suite/cases/password-recovery/environment/Dockerfile`、`evals/terminal-bench/suite/cases/password-recovery/README.md`
- 只读完整上游包：`evals/promptfoo/results/qwen38-suite/source/tasks/password-recovery/`
- 忽略产物：`evals/promptfoo/results/qwen38-suite/preflight/password-recovery/`、`evals/promptfoo/results/qwen38-suite/diagnostics/password-recovery/`

## 上下文

原题 password-recovery；类别 security；CPU=1、内存=2048MiB、存储=10240MiB、GPU=0；Agent=900秒、verifier=900秒。预建镜像 alexgshaw/password-recovery:20251031。原题树SHA=1c9b07ed999d3839f6cd9eb33ccf1c2167738607；题面SHA256=bc4f501148805bbccdb94a5c5b745f1ce9564f04ef0fe36bc37c3ca20b7810a5；配置SHA256=cfd0362d201b957796f9bf1c02cc75646250cb2bd107ec88c768bfcfd091b24b。读完整 instruction.md、输入、solution、tests 后设计，禁止改原题。四子题：m1 识别合成文件删除痕迹；m2 搜索可恢复的数据片段；m3 过滤错误候选与诱饵；m4 保存虚构目标口令。

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

先使用技能 ./skills/create-agent-evals/SKILL.md；每个子题定义合成输入、清楚完整题面、输出路径/API/服务行为与独立确定性判定。新增 evals/terminal-bench/suite/cases/password-recovery/tests/test_cases.py：正解通过，至少3个有意义错误变体被拒绝；运行真实程序/产物，不验证关键词、自述或仅Agent自测。错误fixture和漏检checker先失败，再最小修复。

验证命令：

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/password-recovery/tests -p test_cases.py
```

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/password-recovery/case.json --run-fixtures
```

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.run preflight --catalog evals/terminal-bench/suite/catalog.json --runtime evals/promptfoo/results/qwen38-suite/runtime.json --parent-id password-recovery --output evals/promptfoo/results/qwen38-suite/preflight/password-recovery
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

## 2026-10-28 执行检查点

- 主调度确认依赖批次 1–5 已满足；完整阅读本题全部 8 个上游文件，未修改原题、公共代码、模型或预算。
- 已准备本题 case/checker、四份独立契约、4 gold、12 错误程序及 host 黑盒测试。先红：21 项中 m3 normalize-case 被漏放（退出1）；补充独立小写诱饵后指定 unittest 命令 21 项通过（退出0）。每个 gold 运行4个请求，nop 和3负例均被拒绝。
- 已运行 `PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/password-recovery/case.json`（退出0；不带 run-fixtures，仅结构及哈希验证，checks为空）。主机收据位于忽略目录 diagnostics/password-recovery/host-validation.json。
- `evals/promptfoo/score.sh test` 退出0（Configuration is valid）。
- 动态领取实际槽位1；`CARGO_TARGET_DIR=/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1`，`CARGO_BUILD_BUILD_DIR` 同路径，`CARGO_BUILD_JOBS=1`；运行 `cargo clippy --all-targets --all-features -- -D warnings` 退出0（无行为测试数量；仅既有 block 0.1.6 future-incompatibility 提示）。进程结束后删除本会话owner并rmdir释放锁，缓存保留。没有 Rust 修改，不运行格式化。
- 阻塞：主调度通知公共生产/prepare/评分恢复 builder 资源修复独占窗口，明确暂停一切新Docker build、工具image、标准Harbor、父镜像下载及父校准；当前 host 约19.2GB，不符合新增父镜像下载准入。未执行 validate --run-fixtures 或指定 preflight，未取得父题oracle/nop、四子题fresh容器校准、平台/资源隔离收据，不能标完成。
- 当前 digest 引用缓存 Python base，amd64 可用性尚未核实，不代表最终工具镜像。无付费调用，无新Docker构建或镜像下载。公共代码hash尚会变，不将host单测冒充容器校准。
- 恢复入口：主调度通知公共入口稳定并分配受限窗口后，按README确认amd64、磁盘正硬Ucap准入和全部压缩副本+18GiB/metadata；重绑公共代码身份，执行 validate --run-fixtures 与原题 preflight，补全镜像/依赖/fixture/checker哈希及隔离资源收据。只保留本题资产，不提交或清理其他任务文件；本任务保持已阻塞，未提交，待完整校准后按编号串行提交。

## 2026-10-28 容器诊断恢复检查点（取代上段共享入口暂停原因）

- 公共资源修复964419691稳定，已阅读suite/BUILD_RESOURCES.md；使用原公共 suite.scoring.harbor_trial / SuiteDockerEnvironment，无私有Trial/compose/platform覆盖。
- case.image实际验证为Linux amd64单Docker v2 manifest，Python3.13.7/x86_64；空/app，无/tests、/oracle、/solution，无新工具镜像或父镜像下载。镜像digest=32c8a19cdeab820f8ea710fcdacaef0893848898158f0d3ccc890a75ab365adf，FROM/ref未更改。
- 使用 /tmp/task66-fixtures.py 调用 diagnostics.validate(case,run_fixtures=True)，退出0：20个fresh容器（4gold/4nop/12负例）全部符合预期，捕获20容器实际1CPU/512MiB/128PID、无网络、只读root。case目录validation.json及忽略目录container-resume/validation.json绑定所有声明哈希。没有覆盖原host-validation.json。
- `PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python evals/promptfoo/results/qwen38-suite/diagnostics/password-recovery/container-resume/calibrate-children.py` 退出0：8个标准fresh Harbor控制，4oracle=1、4nop=0，首次评分且无recovery。每次实际main=1CPU/512MiB，sidecar=0.25CPU/64MiB；原materialize文件不变，main镜像amd64且沿用缓存base层。
- children-calibration.json保存开始/结束身份确认、task/scoring树哈希、Harbor/冻结Agent/原题源码以及8次真实磁盘与活动资源准入。execution身份 f759534dd79bbe107a90b14ddc75b553f285607cd7e9c3fd5ca10d841c43f8e5 前后相同。诊断按S1/B2/A1/5GiB，base U=330438807、context硬上限1MiB，无新compressedcopies。
- 主机指定unittest再次21项通过（退出0）。再次动态领取槽位1执行cargo clippy --all-targets --all-features -- -D warnings退出0；绝对target/builddir均为/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1，JOBS1；自有锁已释放，缓存保留，仅既有block future提示。
- 唯一未突破项为父题容量：host=18705506304，VM=92692164608，真实min=18705506304 bytes；必要S10+B2+A1+5GiB=19327352832 bytes已超出，尚不含全部压缩副本、正硬Ucap和metadata。C/U未知并未按0；parent-capacity-rejection.json保留拒绝收据。未父下载/未原题preflight，缺父oracle通过/nop失败，不能标完成或生成完整preparation.json。
- 原始资源、图像、探测、构建、评分及身份收据都在 evals/promptfoo/results/qwen38-suite/diagnostics/password-recovery/container-resume/；父容量恢复后重新完整准入并执行任务原preflight。无付费调用，原评分/资源/超时/模型预算未改。保持已阻塞及编号文件，独立资产未提交，等待父证据后按原任务要求提交。

## 2026-10-28 base 容量 metadata 更正（仅说明维护）

- 前段“base U=330438807”及旧raw字段 cached_image_U / cached_base_U 实际取自 Docker inspect.Size，属于inspect报告大小，不能称完整层解压归档U；旧raw/评分/哈希均保留原样。
- 只读引用63既有 base-layer-sizes.json：唯一层归档U=910167040 bytes；按同一RootFS中重复引用计完整层归档U=910168064 bytes。66仅核对已有metadata及RootFS diff_id序列、逐项U求和，没有自行重新解压或重测fullU。
- 本题 diagnostic-admission.json 与 children-calibration.json 的base均为单manifest sha256:32c8a19cdeab820f8ea710fcdacaef0893848898158f0d3ccc890a75ab365adf，RootFS序列与63分层证据完全相同。本轮复用已经缓存的base，无新增该base compressedcopy或新layer import，因此该缓存base复用的extra C/U=0；并非完整archive U=0，也非本轮合成COPY/context/运行可写层零占用，后者仍按原S/B/A/context预算计。
- 新增忽略收据：evals/promptfoo/results/qwen38-suite/diagnostics/password-recovery/container-resume/base-size-metadata-correction.json，引用63分层证据及本题旧准入/控制/父拒绝收据并记录各原始文件SHA256。仅更正metadata说明，不改case源、既有raw JSON/hash或历史评分；20容器fixture与8标准控制不因本次文字更正失效。
- 本次未重跑测试、容器校准、Clippy或磁盘准入。父容量阻塞及状态“已阻塞”保留，未父下载；待容量恢复后重新做完整父准入和preflight。
