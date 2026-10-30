# 任务 84: 准备-rstan-to-pystan

批次：【批次 48】；依赖批次：1、2、3、4、5。同批无文件依赖；Git 提交按任务编号串行，禁止删除其他会话的索引锁。

状态：已阻塞（HTTPS变体已实现；Launchpad响应截断/超时，尚未完成校准）

## 本轮验收结果与恢复入口（2026-10-30，最新）

- 已落实用户批准的两URI HTTPS独立变体，镜像 `vw-rstan-https-apt@sha256:0da5996e1c9f25c72dfe7c3091fe285130108f4339ab28fb6eaf9962ca92d0fd`。原镜像全部9层和运行配置语义保持；新增层唯一普通文件为ubuntu.sources，只有两个URI增加`s`；另有3个空Rosetta目录元数据，已单列记录。原114包清单不变，没有预装Python、依赖、答案或参考产物。两个InRelease及原失败deb均完整HTTPS读取成功。
- 新入口、schema及before-Agent证明已正式接入：仅此parent/id可用，固定基础镜像、配方、sources字节及完整原包清单；重哈希不能授权额外安装、复制、环境变量、源或包集合变更。6项先失败的新增回归通过，连同旧协议/变体/资源/契约共43项实际回归通过。Clippy在动态槽位3退出0，target/build均为 `/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-3`、JOBS1，自己的锁已释放，缓存保留；Rust未修改、未运行全量Rust测试/格式化。当前执行身份 `588690e423e955cab7b2c335d302678cae81598b2ec421946065c88f964ce16c`。
- 通过现有SuiteDockerEnvironment的真实before-Agent证明后，标准兼容变体oracle已实际运行4CPU/8GiB/无swap，APT成功安装此前失败的software-properties-common，越过原阻塞。随后原命令add-apt-repository在Launchpad读取响应时失败：`http.client.IncompleteRead(15 bytes read)`。官方verifier确实运行6项（1通过、5缺产物失败），raw reward=0完整保留；派生 `https-apt-approved-20261030/calibration-classification.json` 为invalid/unscored、infrastructure、capability=null。未进入Stan采样；标准preflight在首个oracle失败后停止，没有新nop及8子题控制，不能宣称完整变体准备完成，也不能把旧94dec身份的8控制填充到本轮。
- 进一步受限只读探针完整解析了Launchpad WADL（2706949B），但读取团队metadata仍超时。没有链路恢复证据，因此不盲重试，也不扩展两URI授权范围去绕过PPA、预装Python或修改原安装流程。原题HTTP失败、HTTPS变体PPA失败均分别保留，不计模型能力0。兼容成绩始终单列且counts_as_original_official=false。
- 当前所有本题作业已结束：旧原父PID88322和HTTPS恢复PID73127均退出，存储观察器也正常结束（33次采样，无超额或停止动作）。活动指针 `active-https-variant.json` 只用于找到最新失败目录；状态 `https-apt-approved-20261030/state.json` 为failed。**不要直接重启旧脚本（目录已存在），不要删除历史目录或生成就绪收据。** 有新恢复证据后使用该目录的合法runtime.json和全新preflight输出路径，按原资源单次校准变体父题及8子题，再verify_preparation验收全部10控制；完成前保留本任务。
- 汇总进度日志曾被我的重复启动命令误覆盖；重复作业在existing-directory检查即退出。全部分阶段transport/build/OCI/协议/Clippy和真实校准原始日志完整，错误文件 `https-apt-duplicate-start-error.log` 与 `log-integrity-note.json` 明确记录，没有丢失官方评分数据；恢复日志为 `resume-https-apt-approved.log`。3份旧/tmp归档的gzip无损保存及去重清理记录仍在。

## 最新结果与授权变体（2026-10-30）

- 原队列PID88322确已启动原父oracle，实测双边容量通过、main保持4CPU/8GiB/无swap。当前已结束并清理自有容器；证据 `resume-20261030-155d1125/parent-controls.json`。原始HTTP APT包下载再次连接失败（198.18.0.99:80），software-properties-common未安装，随后add-apt-repository缺失；未进入Stan采样。官方verifier确实运行6项：1通过、5缺产物失败，raw reward=0。派生 `calibration-classification.json` 明确invalid/unscored、infrastructure、capability=null；原始summary保留，未继续把失败oracle冒充合格校准或盲重试原题。
- 用户已明确授权独立 `rstan-to-pystan-compat-https-apt` 变体：仅把Ubuntu archive/security两个源URI从http改https，保留Signed-By、包集合、原题文件、4CPU/8GiB/10GiB及1800/1800秒；不预装依赖；先验证HTTPS再构建/校准，成绩单列且不计原题官方成功。两行差异、原/目标源完整字节和授权范围在 `proposals/https-apt/`。
- 从未启动的原镜像只读快照取出114个已安装包的固定清单；原镜像已有ca-certificates 20240203。已准备case/variant的Dockerfile、原/目标sources、base-packages；尚无可宣称完成的变体contract/image。六项协议测试先证明当前入口拒绝此变体；隔离候选协议连同既有CMake/Stan/资源/契约测试43项通过。当前共享variants.py/schema.json未改，以免改变正在运行的MCMC校准身份；候选补丁有原SHA前置检查。
- **当前活动PID66933，exec session67884**；入口 `prepare-https-apt-approved.py`，指针 `active-https-variant.json`，状态与日志在 `https-apt-approved-20261030/state.json`、`prepare-https-apt-approved.log`。当前等待其它4CPU/8GiB校准结束才能验证HTTPS。旧PID57676已在任何探针/构建开始前被替换，保留 `https-apt-approved-20261030-wait-initial/` 说明。禁止重新启动原父队列或重复HTTPS作业。
- 当前流水线顺序：①受限原镜像完整读取两个HTTPS InRelease及此前失败的deb，失败则不构建、不改协议；②仅两URI的受限classic构建，完整OCI/config/层哈希回读，确认只追加sources文件且包/配置语义保持；③空闲窗口中核对原SHA后接入正式variants/schema，43项实测协议回归和动态槽位Clippy；④合法私有runtime经标准preflight重新执行变体父题oracle/nop及8子题控制；⑤核验完整10控制收据和父题6项真实verifier。任何阶段失败即保留证据、停止，不自动扩边界或计模型分；通过后仍待本会话审查交接、提交及按完成条件处理任务文件。

## 最新续接（2026-10-30，活动入口以此为准）

- 旧PID92694已退出：排队期间遇到未设限的外部容器，严格资源检查返回 `missing or excessive suite Docker resource limits`，没有运行父题。未放宽或忽略该检查。
- 本轮重新核验8项子题控制和完整inputs，执行身份仍为 `94dec6f0c90690b5c0ea348c8142a22b0cec1d62fa174df762f6dc93cb7653df`。任务47已释放预约，但其它校准仍在占用共享构建锁/资源。
- 磁盘曾降至约16.5GiB，低于原父准入。本题三个完整/tmp归档已无损压缩，逐个解压流SHA与原tar一致，节省1587814374B；记录在 `archive-compression-continue.json`。两份重复public依赖目录逐文件与保留副本比对后清理，所有唯一来源、原始校准和最终OCI仍保留，记录在 `redundant-public-context-cleanup.json`。未清理其他任务缓存。归档现在使用 `probe-tmp.tar.gz`，历史原tar SHA仍可通过解压流复核。
- 排队器的宿主门槛已对齐启动前的完整 `3C+Ucap512MiB+S10+B2+A1+安全5GiB`，避免在18GiB至完整门槛之间提前退出。PID79738已在未启动父题时终止；其 `resume-20261030-6a63d8ec/state.json` 记录原因。
- **当前活动PID88322，exec session13139**；最新指针仍为 `active-resume.json`，目录 `resume-20261030-155d1125/`，日志 `queue-parent-resume-third.log`。当前为 `waiting_for_exclusive_parent_capacity`，没有本轮父题成绩。后续先检查此PID和state，禁止重复启动；取得窗口后仍按原资源、1800/1800秒和标准harbor_trial运行oracle/nop。仅恢复控制脚本与证据/说明变化，未修改被测实现或评测资产，未重复已通过测试，不提交仅状态更新。

## 本次继续（2026-10-30）

- 重新核验当前preparation inputs、20 fixture收据及8个标准子题控制：执行身份、当前物化字节、原物化包、原始评分及日志树均匹配，可严格复用。未重跑已通过的子题，未改共享实现。
- 任务47的2CPU/4GiB预约仍由活跃的reserve-resources、真实x86虚拟机和原题oracle持有，已确认不是遗留占用，未清理或绕过。父题仍需原4CPU/8GiB独占窗口。
- 已启动真实排队进程 PID `92694`（exec session `49605`），入口 `evals/promptfoo/results/qwen38-suite/diagnostics/rstan-to-pystan/queue-parent-resume.py`。活动指针 `active-resume.json`；本轮目录 `resume-20261030-284f352f/`，初态 `waiting_for_exclusive_parent_capacity`。最多等待2小时；仅资源完全空闲、执行身份及双边容量重新通过后，以标准harbor_trial顺序运行原父oracle/nop，各保持1800秒及原资源。不会重试失败控制、发起付费请求或自动生成就绪收据。
- **续接前先检查此PID、活动指针和state.json，禁止重复启动队列。** 父题结束后审查本轮parent-controls原始评分与资源、正式验证完整preparation，再提交并按完成条件处理本文件；排队或失败不能算完成。本轮仅增加恢复记录，不提交仅状态更新。

## 本次授权恢复与交接（2026-10-30）

- 用户已明确授权仅本题四子题使用 2GiB，保持 1CPU、无 swap 和原超时。真实最小正态模型编译并产生10个样本，峰值1845981184B；独立3观测/2特征的合成GP也编译并采样成功，峰值2147483648B、`oom_kill=0`。512MiB历史OOM全部保留。未增加父题预算、模型预算、超时或付费尝试。
- 四个独立合成资产已实现：m1 CSV与设计矩阵；m2 RStan采样参数映射及真实Stan GP密度校验；m3参数维度、正值和显式seed检查；m4后验均值与预测CSV。每题正解/nop/三个语义负例。额外目标列的checker漏检先以回归失败复现，再新增黑盒探针修复；不接受自述、关键词或模拟采样器。
- 最终公开工具镜像 `vw-rstan-pystan-diagnostics@sha256:ebaf0987df9c994c99dbe2b0980cb2f2bacc4ac34bc343cd227d683ba6de0ed6`：Python3.12.12/PyStan3.10.0/httpstan4.13.0/clang14.0.6。固定来源的解释器、23个wheel和9个Debian包全部回读SHA；镜像manifest/config/逐层完整SHA及diff_id、amd64与RootFS验证通过。仅公开依赖进入镜像，无原题输入、参考解、checker、gold或编译模型缓存。冻结ELF `--help`、真实运行时LD_LIBRARY_PATH下的clang与PyStan导入均通过；clang库已注册系统加载器。构建使用统一受限classic命令0.5CPU/2GiB/无网络；未覆盖正式Trial的platform/FROM/Compose。
- 统一诊断运行器仅对 `rstan-to-pystan` 显式使用2048MiB，fixture无swap，标准Harbor由现有资源策略固定无swap；其他子题仍为512MiB。两个预算回归先失败后通过，12项协议回归全部通过。共享锁竞争曾使首个子题和父题在Agent启动前以BlockingIOError失败，原证据保留；最小底层修复为可取消的异步锁等待，原Harbor超时和4CPU/8GiB准入均保留。12项构建/资源测试通过，含事件循环可运行、等待可取消、异常后释放及容量拒绝。
- 最终镜像：5项本题测试通过（含20个真实隔离fixture控制）；独立 `suite.diagnostics validate --run-fixtures` 的20项全部通过；8个fresh标准 `suite.scoring.harbor_trial` / `SuiteDockerEnvironment` 控制全部匹配 oracle=1/nop=0，各main实际1CPU/2147483648B、MemorySwap同值，源字节不变。当前执行身份 `94dec6f0c90690b5c0ea348c8142a22b0cec1d62fa174df762f6dc93cb7653df`。共享实现哈希改变，其他题旧validation/preparation收据须在后续freeze前刷新；本轮未改其他题资产或预算。
- 三条原任务命令已实际执行：前两条退出0；原preflight的根 `runtime.json` 是历史证据报告，缺 `historical_root`，退出1。未篡改历史报告；另建忽略目录的合法 `runtime-20261030.json`，保持generation2冻结运行时和父题1800秒。修复锁后的正式preflight已实际到达容量准入，以 `suite Docker capacity exceeds 4 CPU / 8 GiB` 在Agent启动前退出1；其他任务仍占资源，无法满足原父独占窗口。没有新官方verifier测试或新能力评分，reward/capability均为null；原始apt失败及raw reward=0仍按invalid/unscored保留，不能用端点恢复200替代父题oracle通过。
- 最终Clippy动态领取槽位1：`CARGO_TARGET_DIR`、`CARGO_BUILD_BUILD_DIR` 均为 `/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1`，JOBS=1，`cargo clippy --all-targets --all-features -- -D warnings` 退出0，有效行为测试数0；自己的owner/锁已释放，缓存保留。此前本轮首阶段槽位2同命令也退出0。未修改Rust、未运行fmt、全量cargo test、wasm或Makefile。`evals/promptfoo/score.sh test` 退出0。
- 新证据入口 `evals/promptfoo/results/qwen38-suite/diagnostics/rstan-to-pystan/checkpoint-20261030.json`；各阶段原始日志、依赖/镜像锁、TDD、资源、正式控制、容量与Cargo收据在同目录。两个2GiB探针完整/tmp归档经tar完整读取和SHA验证后按精确ID删除；本轮失败builder及8组自有Trial容器已精确清理，未删除其他会话的锁、进程或缓存。
- 当前“已完成，待回归”仅表示本轮实现及诊断验收完成，剩余正式父题验证被无关共享资源阻断；**完整题尚未校准，不就绪，不生成case preparation/readiness，不删除本任务**。恢复时先取得可容纳原父4CPU/8GiB的空闲独占窗口，使用下方合法私有runtime和新输出目录执行原父oracle/nop及完整preflight；可按严格哈希审计复用本轮8个子题控制。不得沿用旧容量快照，不得调大总资源或用无效父题记录填分。

## 本次恢复证据（2026-10-30）

- 已执行上一轮恢复入口的官方 clang 探针，未提高预算：固定原诊断镜像 `sha256:32c8a19cdeab820f8ea710fcdacaef0893848898158f0d3ccc890a75ab365adf`，离线运行 Python 3.12.12 / PyStan 3.10.0 / httpstan 4.13.0，以 Debian clang 14.0.6 编译同一个最小正态模型。真实编译退出 1，clang 进程退出 -9，Docker `OOMKilled=true`；cgroup `memory.peak=536870912`、`oom=1`、`oom_kill=1`、`memory.swap.max=0`、`cpu.max=100000 100000`。尚未进入采样；没有样本、预编译答案、源码补丁或模拟采样器。clang 也不能在已测的默认编译路径下突破 512MiB 限制。
- 工具链通过原镜像 Debian bookworm 官方源的 apt 签名与包体校验取得，九个真实 deb，clang 固定 `1:14.0.6-12`；各包 SHA256 再与 apt 元数据逐一比对。首次获取在 `cap-drop ALL` 下因 apt 降权失败，第二次因旧 archives/partial 的 DAC 权限失败；第三次只改获取命令使用 root sandbox user 和独立 `/tmp/archives`，保留 ALL capabilities drop，未关闭签名或包体校验。三次原始日志均保留。
- 新容量准入实测 host/VM，并保存 `admission.json`，按 `3Ccap+Ucap+S1+B2+A1+安全5GiB`，Ccap=512MiB、Ucap=2GiB；仅下载约42MB官方包，无原父层重拉、全局清理或共享 builder 修改。原父 oracle 的 apt 下载失败仍是独立未解决阻塞，不能由本次 Debian 工具链成功获取推断 Ubuntu 父题已通过。
- 新证据目录：`evals/promptfoo/results/qwen38-suite/diagnostics/rstan-to-pystan/clang-20261030/`，包括 deb-lock、官方 apt lists/元数据、SHA256SUMS、完整命令、原始日志、容器 inspect、cgroup 和清理收据。探针完整 `/tmp` 归档 664243712B，经 tar 完整读取与 SHA256 校验（`e196c4103329a46e434656eed3384560553f807e34d8d83bfb269f2a75f584d1`）后，四个本轮已停止容器按记录的精确 ID 删除；未触碰其他会话的容器、锁、源码或缓存。
- 本轮没有修改业务/评测代码，没有运行 Cargo、20 fixture、8 子题标准控制、父题重校准或三条正式验证命令；这些仍无新增通过证据。保持已阻塞，不生成 readiness/preparation，不删除任务文件，不提交仅阻塞记录更新。
- 恢复入口：已向用户请求明确授权本题诊断内存从512MiB调至2GiB，保持1CPU、无swap与原超时，先验证真实编译；2GiB是否足够尚未实测，未获授权不得执行或改共享诊断预算。若授权，需只对本题显式接入预算并验证其他题仍保持512MiB，再制作四子题、完成原定全部验收。原父安装路径的基础设施失败需另行处理，不得修改官方题面、oracle或checker后冒充原题成功。

## 本次执行证据（2026-10-28）

- 阻塞为诊断真实工具资源限制，不是父题磁盘不足：官方 PyStan 3.10.0 / httpstan 4.13.0 / Python 3.12.12 已离线安装成功；`stan.build('parameters {real x;} model {x ~ normal(0,1);}', random_seed=1)` 的真实 gcc 编译在 1CPU / 512MiB / 无 swap 下失败，`cc1plus` 被杀，Docker `State.OOMKilled=true`，退出码 1。没有生成样本、没有模拟采样器，也没有扩大子题预算。尚未制作可宣称完成的四子题资产或生成 readiness/preparation 收据。
- 缓存工具镜像 Python 3.13 的首次路径也已真实排查：pysimdjson<7 无 cp313 wheel，官方 6.0.2 源码在 CPython 3.13 的 `_PyLong_AsByteArray` / `_PyGen_SetStopIterationValue` API 上编译失败。随后改用官方固定 Python 3.12 和 23 个真实 wheel；宿主完整 SHA、压缩/解压体积和安装配方均保留。没有修改原父 Agent 环境。
- 依赖批次 1–5：catalog、contract、runtime、agent 共 48 项聚焦测试通过；上游全部 13 文件与冻结 catalog SHA256 匹配。执行身份 `ce05045daeb0b6f9a28b5cd179f2f4363cfbbaf143a099565e27218a494a4bd1` 与授权一致。`evals/promptfoo/score.sh test` 配置验证通过，无模型请求。
- 初次 host 12573515776B / VM 93889794048B 未满足原父最低 18GiB，未下载原父层。之后外部清理使宿主约 99GiB，重新实测双边空间并按 `3C+Ucap+S10+B2+A1+安全5GiB` 准入；原父镜像 `sha256:b23d48839408524326095c15ada4145c772b87646cf299022ff1f58c97baba91`（linux/amd64），C=50931626B，完整流 SHA / diff_id 验证后的 exact U=146356736B，导入后配置与 RootFS 完全匹配。原父冻结 ELF 无密钥离线 `--help` 实际通过。
- 原父标准 `suite.scoring.harbor_trial` / `SuiteDockerEnvironment` nop 已完成，原始 verifier 6 测试（1 通过、5 缺产物失败），reward=0；oracle 已结束：原始 apt 下载失败，随后 `add-apt-repository` 缺失；官方 verifier 同样 6 测试（1 通过、5 缺产物失败），raw reward=0。该 oracle 校准标记 invalid/unscored、能力成绩 null、基础设施失败，不计模型能力 0；原始 Harbor summary/reward 保留，派生归类在 `checkpoint.json`。没有重试或预修父 Agent。资源保持 4CPU / 8192MiB / 10240MiB，Agent/verifier 各 1800 秒；无私有 Trial、platform/FROM/Compose 覆盖或全局配置修改。
- Cargo 首阶段动态领取槽位 1，绝对 `CARGO_TARGET_DIR` 和 `CARGO_BUILD_BUILD_DIR` 均为 `/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1`，`CARGO_BUILD_JOBS=1`，执行 `cargo clippy --all-targets --all-features -- -D warnings`。外部删除整个 target（含活跃锁和缓存）导致临时目录消失，退出 101，原始失败日志未覆盖；本会话未删除他人锁。已重新动态领取槽位 1 重建验证，最终退出 0；有效行为测试数 0（Clippy 是静态检查，不代替上述 48 项及评分验收）。自己的 owner/锁正常释放，缓存保留。未修改 Rust，未运行 fmt、全量测试、wasm 或 Makefile。
- 原始日志、安装失败、真实依赖/哈希、容量/OCI、父题评分与 Cargo 收据均保留于忽略目录 `evals/promptfoo/results/qwen38-suite/diagnostics/rstan-to-pystan/`。诊断 20 fixture、8 标准控制和三条任务验证命令尚无有效验收证据，不能以零测试、父题 nop 或安装成功替代；任务文件保留，不提交仅状态更新。两个停止探针容器已在完整 `/tmp` 归档 SHA256 校验后按精确 ID 删除，父题 Trial 容器已结束并清理；原镜像、唯一 OCI/层、真实依赖、原始失败/评分及 Cargo 缓存保留，独占窗口已释放。
- 恢复入口：先解决真实 Stan 扩展在既定 512MiB 下的编译可行性（保留本次 OOM；不得自动提高预算），本轮没有新增 clang 工具链，也未试图修改 PyStan 源码、预置答案或调高预算；若后续采用官方 clang，须先固定来源/SHA、重新准入并在同预算做一次真实探针。再制作/校准四个独立合成子题，完成 TDD、20 fixture、8 标准控制、官方父题 oracle/nop 与正式 preflight；重新动态领取 Cargo 槽位，使用新的双边容量实测，不沿用旧快照。

目的：为“统计模型接口转换”准备原题完整环境与四道独立诊断题。

来源任务：Terminal-Bench2.1/rstan-to-pystan

预计会话范围：只准备一个父题的四个窄诊断题和确定性校准；不运行付费模型。长镜像拉取允许记录进程后续接，不能把未校准标成完成。

## 文件

- 新建：`evals/terminal-bench/suite/cases/rstan-to-pystan/case.json`、`evals/terminal-bench/suite/cases/rstan-to-pystan/check.py`、`evals/terminal-bench/suite/cases/rstan-to-pystan/tests/test_cases.py`、`evals/terminal-bench/suite/cases/rstan-to-pystan/fixtures/`、`evals/terminal-bench/suite/cases/rstan-to-pystan/oracle/`、`evals/terminal-bench/suite/cases/rstan-to-pystan/environment/Dockerfile`、`evals/terminal-bench/suite/cases/rstan-to-pystan/README.md`
- 只读完整上游包：`evals/promptfoo/results/qwen38-suite/source/tasks/rstan-to-pystan/`
- 忽略产物：`evals/promptfoo/results/qwen38-suite/preflight/rstan-to-pystan/`、`evals/promptfoo/results/qwen38-suite/diagnostics/rstan-to-pystan/`

## 上下文

原题 rstan-to-pystan；类别 data-science；CPU=4、内存=8192MiB、存储=10240MiB、GPU=0；Agent=1800秒、verifier=1800秒。预建镜像 alexgshaw/rstan-to-pystan:20251031。原题树SHA=c0f640ad3dffd723bedb0298bce4c5503f89160f；题面SHA256=7b6dd84eb47579469556915a5eab6d303217bba438424eaee4db52375d5add2a；配置SHA256=ed0cd4382cb353e73cb575aa7a20b624e35f09ddd8769c332f40f8986980e13f。读完整 instruction.md、输入、solution、tests 后设计，禁止改原题。四子题：m1 解析合成训练与测试数据；m2 转换模型与采样接口；m3 校验参数维度与随机条件；m4 比较后验和预测产物。

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

先使用技能 ./skills/create-agent-evals/SKILL.md；每个子题定义合成输入、清楚完整题面、输出路径/API/服务行为与独立确定性判定。新增 evals/terminal-bench/suite/cases/rstan-to-pystan/tests/test_cases.py：正解通过，至少3个有意义错误变体被拒绝；运行真实程序/产物，不验证关键词、自述或仅Agent自测。错误fixture和漏检checker先失败，再最小修复。

验证命令：

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/rstan-to-pystan/tests -p test_cases.py
```

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/rstan-to-pystan/case.json --run-fixtures
```

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.run preflight --catalog evals/terminal-bench/suite/catalog.json --runtime evals/promptfoo/results/qwen38-suite/diagnostics/rstan-to-pystan/runtime-20261030.json --parent-id rstan-to-pystan --output evals/promptfoo/results/qwen38-suite/preflight/rstan-to-pystan-resume-20261030
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
