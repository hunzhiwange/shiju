# 任务 56: 准备-mcmc-sampling-stan

批次：【批次 32】；依赖批次：1、2、3、4、5。同批无文件依赖；Git 提交按任务编号串行，禁止删除其他会话的索引锁。

状态：已阻塞

目的：为“层次模型采样”准备原题完整环境与四道独立诊断题。

来源任务：Terminal-Bench2.1/mcmc-sampling-stan

预计会话范围：只准备一个父题的四个窄诊断题和确定性校准；不运行付费模型。长镜像拉取允许记录进程后续接，不能把未校准标成完成。

## 文件

- 新建：`evals/terminal-bench/suite/cases/mcmc-sampling-stan/case.json`、`evals/terminal-bench/suite/cases/mcmc-sampling-stan/check.py`、`evals/terminal-bench/suite/cases/mcmc-sampling-stan/tests/test_cases.py`、`evals/terminal-bench/suite/cases/mcmc-sampling-stan/fixtures/`、`evals/terminal-bench/suite/cases/mcmc-sampling-stan/oracle/`、`evals/terminal-bench/suite/cases/mcmc-sampling-stan/environment/Dockerfile`、`evals/terminal-bench/suite/cases/mcmc-sampling-stan/README.md`
- 只读完整上游包：`evals/promptfoo/results/qwen38-suite/source/tasks/mcmc-sampling-stan/`
- 忽略产物：`evals/promptfoo/results/qwen38-suite/preflight/mcmc-sampling-stan/`、`evals/promptfoo/results/qwen38-suite/diagnostics/mcmc-sampling-stan/`

## 上下文

原题 mcmc-sampling-stan；类别 data-science；CPU=4、内存=8192MiB、存储=10240MiB、GPU=0；Agent=1800秒、verifier=1800秒。预建镜像 alexgshaw/mcmc-sampling-stan:20251031。原题树SHA=af1d2e3b571ff9f3e8601c7c674601597870297a；题面SHA256=5554ddc6d7b069e965eda4936cd02fce45ed54f51cf38045e6c51aeb39bed943；配置SHA256=c1c0351670499e15b3818f7394894e2789f74fc2a61ea76eccab826c1a951a94。读完整 instruction.md、输入、solution、tests 后设计，禁止改原题。四子题：m1 解析小型层次数据；m2 构造模型和采样入口；m3 固定种子并检查收敛；m4 输出可检验的后验统计。

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

先使用技能 ./skills/create-agent-evals/SKILL.md；每个子题定义合成输入、清楚完整题面、输出路径/API/服务行为与独立确定性判定。新增 evals/terminal-bench/suite/cases/mcmc-sampling-stan/tests/test_cases.py：正解通过，至少3个有意义错误变体被拒绝；运行真实程序/产物，不验证关键词、自述或仅Agent自测。错误fixture和漏检checker先失败，再最小修复。

验证命令：

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/mcmc-sampling-stan/tests -p test_cases.py
```

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/mcmc-sampling-stan/case.json --run-fixtures
```

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.run preflight --catalog evals/terminal-bench/suite/catalog.json --runtime evals/promptfoo/results/qwen38-suite/runtime.json --parent-id mcmc-sampling-stan --output evals/promptfoo/results/qwen38-suite/preflight/mcmc-sampling-stan
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


## 本次检查与阻塞证据（2026-10-28）

已按执行/评测技能阅读只读plan及上游完整instruction、task、README、solution、tests、Dockerfile与全部data.csv。依赖1–5主调度确认完成，本地source/contract/agent/runtime-v2既有收据存在，不作为本题新证据。无付费请求，无上游/公共代码/其他case变更。

容量阻塞：Registry probe exit0，原镜像manifest sha256:073fc36a3347b4f6b43bd6e98339542c41ed531c771af8bee40ba10f00cfabaf，config sha256:c7ba1f1f9d2df36db0338e4d1e429692aaf653ef6faa13dc4694a7c457593d6e，linux/amd64，C=237648324 bytes；本地原镜像不存在（inspect exit1）。layer-tail-probe exit0，仅逐层Range读8字节并核对206/Content-Range，ISIZE和597928960 bytes仅作U下界，不作精确U。capacity probe exit0，host19811373056、VM91480817664 bytes，以min准入。即使只计1C+U下界+S10GiB+B2GiB+A1GiB+5GiB，也至少20162930116 bytes，缺351557060 bytes。未知U不能0，正Ucap=2GiB且需监控的2C下载预算21950133128、3C导入预算22187781452 bytes均不准入。未下载层体、删共享镜像/缓存或启动完整题；未伪造准备收据。

按主调度补充先保留轻量checkpoint：case.json、check.py、tests、fixtures/oracle、environment/Dockerfile、README及validation.json均仅本题。m1完整CSV验证；m2层次Beta/Binomial log joint与受控真实Metropolis转移；m3四链固定种子离散后验Metropolis和unsplit Rhat；m4去warmup后均值与线性分位数。明确这些Python窄核不证明Stan编译/RStan安装/完整连续后验采样，完整父题必须真实RStan校准。四题独立合成输入，gold+各3个有意义负例，公私材料隔离，无父题参考解前缀。

新验证（忽略目录下对应日志）：
- 指定unittest命令先因case缺失exit1（red.log），最小资产后exit0，20项（green.log）。
- checker-red.py用仅首探针的弱checker实际漏掉m2 always-accept，exit1；完整checker聚焦m2回归见checker-green.log，保持实际采样转移判定，不靠关键词。
- 指定 `python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/mcmc-sampling-stan/case.json --run-fixtures` exit0，20项：4gold通过、4nop拒绝、12负例拒绝；validation.json固定checker/fixture/image/policy哈希。
- `evals/promptfoo/score.sh test` exit0，配置有效；日志保留既有telemetry shutdown timeout，不当模型成绩。
- `cargo clippy --all-targets --all-features -- -D warnings` exit0，行为测试0；原子领取槽位1，CARGO_TARGET_DIR=/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1，CARGO_BUILD_BUILD_DIR同值，CARGO_BUILD_JOBS=1。自有锁已释放，缓存保留。无Rust变更，无全量测试/fmt/wasm/Makefile。
- 指定历史preflight命令exit1：给定runtime.json是历史report而非execution config，缺historical_root；已查task52私有runtime恢复格式，无需改公共runtime。该已知配置差异不是最终容量阻塞的替代原因；恢复时使用本题ignored execution配置引用冻结runtime-v2。
- fresh标准子题控制及BuildKit首次失败保留在children-calibration-standard*收据；不改生产materialize的原始FROM/ref、不设全局平台、不改Trial。最终结果见下方检查点。

恢复入口：`evals/promptfoo/results/qwen38-suite/diagnostics/mcmc-sampling-stan/`，registry.json/parent-manifest.json/parent-config.json/layer-tail-probe.json/capacity-probe.json及对应probe脚本；create-assets.py、checker-red.py、calibrate-children-standard-legacy.py、clippy-receipt.json与日志。先重新实测host/VM并满足正Ucap和所有副本峰值；超cap停止重新准入。再固定原digest，依官方4CPU/8192MiB独占、10GiB与1800/1800秒做依赖探测和真实RStan父oracle/nop，仅允许verifier-only依赖修复，不改官方checker。最后绑定既有有效轻量证据、完整生产preflight与当前身份。当前保持已阻塞并保留本文件和未提交轻量资产，不因已有fixture/Clippy通过改为已完成或删除文件。


### 最终轻量检查点

- `calibrate-children-standard-legacy.py`（DOCKER_BUILDKIT=0，仅本调用）exit0，标准materialize原FROM/ref未变，8个fresh控制全部符合预期：m1..m4各oracle=1、nop=0；`children-calibration-standard-legacy.json`绑定每例任务哈希、首评分、证据文件和资源inspect。主容器1CPU/512MiB、sidecar0.25CPU/64MiB，amd64及RootFS链核对成功；全部本题容器退出/清理，无活动下载/Cargo/锁，独占准备窗口已释放。首次默认BuildKit本地ref解析失败原样保留于children-calibration-standard.json，不混作有效成绩。
- 原冻结runtime-v2的shiju（SHA256 847823c5cd99a6828cddfc2e3e5dbb67c5ddcc9f8693a13d539d8a8fa47c9dae）在同digest镜像中、无网络/只读root/只读runtime挂载/1CPU512MiB成功执行`--help` exit0，frozen-agent-help.log证明执行兼容；未调用模型，不证明模型任务成功。
- 源文件树与catalog逐文件SHA和executable mode完全一致，official-identity.json；checker-green.log为5项m2回归exit0。checkpoint.json绑定新验证、容量、镜像、控制和Clippy收据哈希。
- 完整原题oracle/nop未运行，parent preparation.json未创建，保持已阻塞。轻量资产未提交，留待容量恢复后绑定完整题校准与最终身份再按git-commit技能提交本题；本编号文件不删除。plan.md未修改。


## 恢复执行（2026-10-28）

主调度授予4CPU/8192MiB独占窗口；当前执行身份ce05045daeb0b6f9a28b5cd179f2f4363cfbbaf143a099565e27218a494a4bd1。容量重测host68158255104、VM87177605120 bytes，正Ucap2GiB与3C导入预算22187781452 bytes准入，旧容量阻塞已解除。诊断固定镜像冻结x86 ELF离线--help exit0；原checker-only20夹具字节/实现/镜像哈希未变核验复用。新children-calibration-current.json的8fresh正式SuiteDockerEnvironment控制exit0（4oracle=1、4nop=0），无控制器私有platform/FROM/Trial/Compose/Builder覆盖，正式helper自身资源设置生效。原始失败/旧私有legacy收据保留，不作为本次正式控制。原父镜像正在按同digest分段下载与stream SHA/diff_id/exactU验证。


### 恢复执行结果：真实原始oracle依赖阻塞（当前终态）

容量已解除，不再以旧容量不足阻塞。`fetch-parent.py`、`import-parent.py`各exit0：逐层stream SHA256/diff_id全匹配，精确U=597928960 bytes，C=237648324，Ucap=2GiB硬停止；3C+Ucap+S10+B2+A1+安全5的22187781452 bytes准入。原manifest/config/amd64/RootFS/Config逐项回读一致，所有唯一原始层分片、缓存及导入镜像保留，无全局Docker更改。父与诊断镜像冻结x86 ELF离线--help各exit0；只读依赖probe确认R4.3.3且原镜像无RStan。原CRAN archive/rstan_2.32.7端点404并非本次阻塞，原脚本成功找到src/contrib/rstan_2.32.7.tar.gz；根因以下面的实际安装错误为准。

正式`calibrate-children-current.py` exit0，8fresh SuiteDockerEnvironment controls（4oracle=1、4nop=0），不读旧私有DOCKER_BUILDKIT控制，不私设platform/FROM/Trial/Compose/Builder；正式helper资源覆盖生效，当前执行身份ce05045daeb0b6f9a28b5cd179f2f4363cfbbaf143a099565e27218a494a4bd1。旧checker-only20fixture的所有声明文件、checker实现和image哈希未变，按主调度授权核验复用；无新paid调用，无改题/扩大预算。

`calibrate-parent-current.py`控制器exit0但ready=false，不是题目完成：原始上游oracle实际exit1。oracle.txt:7500–7504、8263–8267记录当前RcppParallel 6.2.1要求cmake>=3.5，而冻结原镜像和上游solve.sh的apt依赖未包含cmake；随后StanHeaders/RStan安装失败，oracle.txt:8835为library(rstan)不存在。原脚本在写model/analysis和实际采样前终止。不能通过预修Agent环境、改原脚本或改依赖评分绕过，verifier-only修复不能补回未生成的Agent模型/分析产物；不做相同条件空循环。本题因此保持已阻塞，不能标为待回归完成。

父oracle和nop各执行完整6项官方检查，均2passed/4failed、无skip，原始reward各0；nop失败符合对照预期。oracle的reward0来自安装失败，不记模型能力0：parents-calibration-current.json保留raw_summary，另summary为infrastructure/unscored/reward=null、capability_score=null、calibration_status=invalid。原始评分、完整oracle日志、exit-code.txt=1、两份ctrf与first.json不修改。未生成父preparation.json。`audit-current.py` exit0，blocked-gate-current.json核验原始6+6测试、8正式子控制、20fixture哈希及所有材料隔离；47份R依赖源码归档43182366 bytes保存在parent-source-cache并SHA绑定，供授权后恢复使用。

存储归属纠正：主容器SizeRw归S10GiB，额外保存源码归档归B2GiB，其余收据归A1GiB，安全5GiB保持。原始监控错把SizeRw标B，但本次180个oracle样本峰值仅757465088 bytes、19个nop样本峰值228630528 bytes，冗余低阈值未触发，不是本题失败原因。storage-audit-current.json按实际S和实际(B+A)重新核验每个样本的remaining S10+B2+A1+安全5；B归档实际分配43274240 bytes，A上界<48MB，无预算超限。不得将旧监控的2GiB保护直接作为未来S上限；不修改安装路径、原脚本或扩大预算。

父重阶段和诊断结束后串行运行 `cargo clippy --all-targets --all-features -- -D warnings`：exit101，0行为测试，既有无关E0433（crates/vw-cli/src/cli/tui/runtime/gateway_tests.rs:119的super::capabilities不存在），本题没有Rust改动，不修无关模块。动态原子槽位1，CARGO_TARGET_DIR=/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1，CARGO_BUILD_BUILD_DIR同值、CARGO_BUILD_JOBS=1。clippy-current.log/receipt-current.json记录确切命令与退出码，自有锁清理，缓存保留。无全量cargo test/fmt检查/wasm/Makefile。

恢复需先审定独立的原始环境兼容方案（当前授权禁止预装cmake到Agent环境），不自动扩大模型/评分/预算，也不伪造通过；之后当前clippy无关失败需维护者修复并重验。旧运行时report缺historical_root已通过本题ignored runtime.json正确引用runtime-v2解决，不修改公共runtime文件。所有恢复证据位于本题ignored diagnostics目录，当前blocked-gate-current.json/storage-audit-current.json/parents-calibration-current.json/children-calibration-current.json为入口。本题资产未提交、编号文件保留。plan.md及其他case/业务代码均未修改；本题容器、下载与Cargo进程均结束、自有锁释放，4CPU/8192MiB独占窗口已释放。


### 未授权兼容方案草案（仅只读元数据，未执行）

按主调度补充，只读检查原环境软件源，未改Agent环境、未安装/构建/启用。`probe-cmake-metadata.py`、`verify-cmake-metadata.py`、`prepare-cmake-proposal.py`与`check-cmake-constraints.py`各exit0。原Ubuntu keyring验证noble InRelease并核对Packages.xz SHA。已知cmake3.28.3-1build7期望包SHA256=4b0a7f8c0daf27b26b46997d994ae5d1ee7a3d11dfcda9f7627bb1462c162295，cmake-data同版本SHA=20a3b644211ce82f35c24f1f5052199adeb0ac159978a8740fd6c0959611557f；来源archive.ubuntu.com/ubuntu noble/main。必要缺失依赖闭包另含libarchive13t64、libjsoncpp25、librhash0、libuv1t64、libxml2，固定版本/来源/期望SHA均在deb-lock.json，原已安装包无升级。原镜像dpkg只读比较全部版本约束通过。包体未下载/回读，构建/解压体积和原oracle在该变体上的结果均未知，不宣称可运行；noble基线锁不是updates/security最高优先级候选的声明。

本题ignored `cmake-compatibility-proposal/approval.md`、Dockerfile.draft、SHA256SUMS、deb-lock.json、dependency-version-check.json为具体审批材料，cmake-proposal-receipt.json绑定其SHA。原父digest不变，配方仅补CMake/数据/必要库，不升级既有包、不改R/RStan/题面/数据/原脚本/评分；该变体未获授权，未沿用13授权，不加入runtime/catalog。approval.md列明授权后所有新准入、正Ucap及所有副本预算、正式受限构建/层链和依赖回读、冻结ELF、原1800/1800秒真实RStan oracle/nop、独立分栏、诊断身份复核与Clippy重校准清单。没有额外原条件重试或付费调用。元数据探测容器已退出，无本题进程/锁，独占窗口释放；本编号保持已阻塞。


## 七包兼容变体获批后恢复（2026-10-28）

用户对56的直接答复call_04c53ccbe5ca463280bf6a9dad84b289第0问授权，仅补锁定7包。正式协议与执行身份b35499b54cd173f716f856c17ecb9baf971ef1326c5b4ebc4656adf9d95fb5df已核对，不复用旧CE正式控制。最新容量host71242178560、VM80675000320，包含所有实际C/U副本、正Ucap及S10+B2+A1+安全5的保守预算28980430998 bytes准入。宿主/容器直连HTTPS超时原样保留；同一锁定URL经Docker既有代理下载全部7包并SHA回读，无来源/版本变化。dpkg-deb流式解包数据tar合计52746240 bytes，未达到512MiB硬上限。正式SuiteDockerEnvironment._limited_build首RUN创建即4CPU/8192MiB，未设控制器platform/FROM/Trial/Compose/Builder覆盖，仅安装锁定7包，原base层链保留。原生OCI单AMD64 manifest sha256:a6c991f2a0f6ba19abdccbd006bef08b40b9c380549d59aa29113c8a9df95559，config sha256:379d89d5964c77d223c7f15b50efdd945470757c245019f98c6ba19e684be8cf，逐层压缩SHA和解压diff_id全流验证，C272027750/U666092032 bytes；原父5层压缩SHA/大小及RootFS前缀保持。变体5文件落在本题variant/，正式resolve通过；47份旧R源码未注入、未预装R/RStan、原题/原失败不变。正在运行新身份fresh8诊断控制，之后串行真实父变体oracle/nop与Cargo；未创建任何子集manifest或付费调用。


### 获批七包变体执行终态：新的Stan版本/语法阻塞

七包授权已落实，CMake缺失旧边界已解除；不再将其或容量作为当前阻塞。用户仅授权固定7包，未授权R依赖版本/环境变量或原模型改写，故当前真实新失败不能自行修复。没有处理91及之后任务、任务99、任何付费调用或实际子集manifest。

当前正式SDK接法与证据：
- variant/五文件完整，`resolve`与正式`inputs`均通过。closure SHA=5fb755bed7fb032a15d63c0eb495a19b2253e345db2181c621190cd5d416cebb，7包实际包体SHA/版本/架构逐一核验。原native OCI archive、所有7包体/上下文、旧唯一OCI层和47份R源码均保留；47份R源码没有注入Agent环境。
- 正式受限helper构建从首RUN创建即4CPU/8192MiB，观察到3个构建容器、无资源违规。单AMD64 manifest=a6c991f2a0f6ba19abdccbd006bef08b40b9c380549d59aa29113c8a9df95559，config=379d89d5964c77d223c7f15b50efdd945470757c245019f98c6ba19e684be8cf，原五层压缩SHA/大小与RootFS前缀保留；每层完整流验证C272027750/U666092032，未以ISIZE/inspectSize充上界。
- 父变体及固定诊断镜像在无密钥、只读、离线条件下执行冻结ELF847823c5... --help均exit0。20个旧checker-only夹具的声明/实现/image/policy哈希全相同，fixture-reuse-audit.json核验复用；当前b35499b54cd173f716f856c17ecb9baf971ef1326c5b4ebc4656adf9d95fb5df身份fresh8正式诊断控制全部通过（4oracle=1、4nop=0），无控制器platform/FROM/Trial/Compose/Builder覆盖。
- fresh父变体oracle/nop都通过正式SuiteDockerEnvironment固定的before-Agent证明：cmake3.28.3、7包版本架构及原基线库存compact SHA970281431fa99f0555b17a42a16c30991ce92bcad1ba4a409bcfc0dd27d10862完全匹配。没有私有校准hook；原已安装包不升级，R/RStan不预装，原题、原参考解、原评分及4CPU/8192MiB/10GiB/1800+1800秒不变。

实际新失败：原脚本成功安装RcppParallel与RStan2.32.7，但安装流程又选择StanHeaders2.39.1（oracle.txt:7844）。加载输出明确`rstan version 2.32.7 (Stan version 2.39.0)`（:23281/:23309）。原analysis.R调用stan_model时，stanc拒绝原模型`int<lower=0> n[num_obs];`旧数组语法（:23319–23335），要求array[num_obs]声明，原oracle实际exit1，未产生有效采样/后验统计。不得在七包授权内改原模型/solve.sh、固定R/Stan依赖或预热R库，也不在同条件重复oracle。

官方验证完整运行：变体oracle 6项中3passed/3failed（RStan安装检查通过，模型/脚本/采样三项失败）；nop 6项中2passed/4failed、reward0符合对照预期。两个原始first.json/raw reward均0保留；变体gold按infra invalid/unscored/reward=null、capability=null记录，不能计模型0或变体就绪。旧原父CMake失败/旧评分/旧CE收据全部原样保留，不冒充当前原题通过。

`audit-approved-blocker.py` exit0：核验fresh8、两份正式before-Agent证明、6+6官方测试、所有raw/材料哈希和真实资源收据；`inputs`通过；`verify_preparation`对实际失败控制组成的ignored rejected-readiness-candidate.json按预期拒绝（gold未通过）。本题case下没有preparation.json，没有正式父就绪收据。blocked-acceptance.json为新边界/排除证据入口，不是通过收据。

资源监控正确分类S10/B2/A1及安全5：oracle高水位S884809728/B43274240/A12132352 bytes（358样本），nop S228646912/B43274240/A12304384 bytes（24样本），所有min(host,VM)覆盖剩余预算，无违反。已留存C/U资产另行实计且不与S/B/A混淆，未信用其他进程释放。

当前必跑Clippy：`cargo clippy --all-targets --all-features -- -D warnings` 命令exit0，0行为测试，动态槽位1，CARGO_TARGET_DIR=/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1、CARGO_BUILD_BUILD_DIR同值、JOBS1，耗时12m37s；自有锁已释放、缓存保留。**不能无条件视为当前最新Rust工作树回归通过**：前后输入摘要不等价，Cargo.toml/Cargo.lock、crates/vw-desktop/build.rs改变并新增vendor/block下Rust/Cargo文件；clippy-input-drift.json和完整before/after快照保留9项变化。没有修复/回滚这些外部变更、没有删缓存/别人锁，也没有用旧pass顶替当前证据。原有block0.1.6 future-compat notice原样保留。未执行全量cargo test/fmt检查/wasm/Makefile。

`evals/promptfoo/score.sh test` 当前exit0，配置有效，不是模型评测；没有修改默认Promptfoo配置、accuracy20例或公共执行代码。全部变更仅本题目录与编号文件。

收尾/恢复入口：本题ignored authorized-cmake/blocked-acceptance.json、parents-calibration.json、children-calibration.json、formal-build/、variant-provenance.json、clippy-receipt.json、clippy-input-drift.json及所有raw日志；新variant/五个正式文件可审阅，未提交。任务56保持已阻塞并保留编号文件，不能改为已完成/待回归完成。后续需要另审Stan依赖/原始语法的兼容边界，并以新独立证据目录重校准，不能改本次raw或突破7包。当前所有本题容器、下载、Node与Cargo进程结束，自有锁释放；保留唯一OCI/真实7包依赖/47源码及Cargo缓存，4CPU/8192MiB独占窗口已释放，供主会话恢复90以内其他原会话。


## 继续恢复：已授权保留原解依赖版本（2026-10-30，进行中）

用户直接答复 call_a26b4c1e616a45608ac89d36389692da 第0问“授权该精确方案，继续校准”。审批材料与4项离线真实remotes策略函数检查在本题ignored stan-upgrade-proposal/；仅在已验证七包digest基础上新增 ENV R_REMOTES_UPGRADE=never，允许新独立变体正式接线和启动前证明，不预装R库、不改原solve.sh/模型/官方6检查/资源/超时、不调用付费模型。原日志:7609已装StanHeaders2.32.10、:7658又升级2.39.1；候选抑制自动升级但不是完整依赖锁，不能预判编译/采样成功。

本次证据独立保存在authorized-remotes-never/。正式变体ID=mcmc-sampling-stan-compat-remotes-never；旧七包variant及raw不覆盖。新增严格配方/继承契约/环境事实检查及schema路由；23项协议检查、20项本题unittest通过，score.sh test exit0。Clippy动态槽位1 exit0，前后Rust输入一致，自有锁释放。正在等待其它题目释放完整4CPU/8192MiB窗口，尚未构建或执行本次父控制，保持阻塞状态，不生成就绪收据。

### 新变体已构建、当前资源排队检查点

正式资源受限helper构建新metadata-only镜像成功（无RUN/COPY、base无ONBUILD，因此使用剩余2CPU/4GiB；父控制仍4CPU/8192MiB独占）。原全部七包层和RootFS逐字节保持，native OCI manifest=dd7e3d1d90ed3b7b9e0510f5c2390bdf5fac692e4d306436f5847f4230cf4470，config=f31b29de166b99fe20325bc25ce6f590a9dd1412974f650eb5d287061afd8cd7；Config唯一新增R_REMOTES_UPGRADE=never，C272028078/U666092032完整流校验。实际仅新增272042496字节压缩导出档和有正上限的metadata/context；不解包或重新导入已存在层。所有S10/B2/A1/安全5预算保留；最新host103411400704、VM70583431168，未信用未来释放或删除缓存。

共享diagnostics.py已由其它任务改动，仅wrapper实现SHA从a8a9e489...变为7c98dc25...，case声明、image、checker/fixture/policy均未改。旧validation快照留存；指定validate --run-fixtures重新真实运行20项（4gold、4nop、12负例）全过，刷新本题validation.json。23项协议测试与40项准备/评分/报告集成测试均通过。

首个正式子题m1 oracle在Agent启动前因共享资源锁BlockingIOError失败，原始children-calibration.json及child-calibration/保留为infrastructure/unscored/reward=null，无实际Agent执行、非能力0。当前新一轮控制输出隔离到children-calibration-current.json/child-calibration-current/，队列先等待完整空闲及锁可用，再做fresh8，然后原始父oracle/nop；父控制从未启动，不重复旧Stan失败的oracle。queue-owner.json/queue-state.json记录运行PID和阶段；任务47与86等仍占用窗口。queue-current.py为恢复入口，若续接先核实进程状态和已有收据，禁止重复启动。尚无preparation.json，不能标就绪。

### 本次收尾：资源阻塞，未标就绪

资源准入与独占窗口累计等待超过20分钟，任务47/86等的资源预留与其它控制仍未结束；本任务不能安全撤销其它会话资源。本次自有空等队列已停止，queue-owner.json标记stopped_resource_blocked_checkpoint；当前fresh8和父控制均未启动。恢复前先看blocked-checkpoint.json、queue-owner.json、queue-state.json，重新确认进程/控制输出不存在、host/VM预算和SDK/fixture身份，再执行queue-current.py；不得盲目重复已有控制。旧首个m1在启动前锁冲突的基础设施记录保留，不能算能力分。

本题必跑Clippy首次slot1 exit0且前后Rust输入一致，元数据保存clippy-earlier-pass-receipt.json。额外追加的最新工作树复验领取冷缓存slot3，主动仅终止自有cargo-clippy进程组（exit -15，0行为测试），其前后Rust输入亦一致；clippy.log/receipt与refresh-cancel-reason明确保存取消事实，不冒充通过。CARGO_TARGET_DIR=/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-3，CARGO_BUILD_BUILD_DIR同值、JOBS1；自有锁释放、缓存保留。未改任何Rust文件、未处理无关变更。

独立blocked-checkpoint.json核验inputs、fresh20、镜像/OCI SHA、冻结ELF、47源码归档、首个子题未运行Agent、当前父题0次和无preparation。无本题容器、下载、Node、Cargo或排队进程，无自有锁；未触碰其它会话暂存/进程/锁，保留镜像及所有原始失败收据。任务保持已阻塞，编号文件保留，plan.md未改；下一步是获得完整4CPU/8192MiB独占窗口后继续fresh8与原父oracle/nop校准。
