# 任务 63: 准备-nginx-request-logging

批次：【批次 37】；依赖批次：1、2、3、4、5。同批无文件依赖；Git 提交按任务编号串行，禁止删除其他会话的索引锁。

状态：已阻塞

目的：为“请求日志配置”准备原题完整环境与四道独立诊断题。

来源任务：Terminal-Bench2.1/nginx-request-logging

预计会话范围：只准备一个父题的四个窄诊断题和确定性校准；不运行付费模型。长镜像拉取允许记录进程后续接，不能把未校准标成完成。

## 文件

- 新建：`evals/terminal-bench/suite/cases/nginx-request-logging/case.json`、`evals/terminal-bench/suite/cases/nginx-request-logging/check.py`、`evals/terminal-bench/suite/cases/nginx-request-logging/tests/test_cases.py`、`evals/terminal-bench/suite/cases/nginx-request-logging/fixtures/`、`evals/terminal-bench/suite/cases/nginx-request-logging/oracle/`、`evals/terminal-bench/suite/cases/nginx-request-logging/environment/Dockerfile`、`evals/terminal-bench/suite/cases/nginx-request-logging/README.md`
- 只读完整上游包：`evals/promptfoo/results/qwen38-suite/source/tasks/nginx-request-logging/`
- 忽略产物：`evals/promptfoo/results/qwen38-suite/preflight/nginx-request-logging/`、`evals/promptfoo/results/qwen38-suite/diagnostics/nginx-request-logging/`

## 上下文

原题 nginx-request-logging；类别 system-administration；CPU=1、内存=2048MiB、存储=10240MiB、GPU=0；Agent=900秒、verifier=900秒。预建镜像 alexgshaw/nginx-request-logging:20251031。原题树SHA=868b31c1ccf43ed998364891ff26a4cd54c423b0；题面SHA256=26a7ac98aced6107f147206790ada77bbf8ed3c25fb360cf323d5fc6889edf99；配置SHA256=b7a78d00be13656f75069197100b7b8c4fd268decf6f78929c853b3222a2729d。读完整 instruction.md、输入、solution、tests 后设计，禁止改原题。四子题：m1 配置本地静态服务；m2 记录指定请求字段；m3 处理路径和日志边界；m4 发送请求验证日志。

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

先使用技能 ./skills/create-agent-evals/SKILL.md；每个子题定义合成输入、清楚完整题面、输出路径/API/服务行为与独立确定性判定。新增 evals/terminal-bench/suite/cases/nginx-request-logging/tests/test_cases.py：正解通过，至少3个有意义错误变体被拒绝；运行真实程序/产物，不验证关键词、自述或仅Agent自测。错误fixture和漏检checker先失败，再最小修复。

验证命令：

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/nginx-request-logging/tests -p test_cases.py
```

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/nginx-request-logging/case.json --run-fixtures
```

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.run preflight --catalog evals/terminal-bench/suite/catalog.json --runtime evals/promptfoo/results/qwen38-suite/runtime.json --parent-id nginx-request-logging --output evals/promptfoo/results/qwen38-suite/preflight/nginx-request-logging
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

- 主调度确认依赖批次1–5满足。已读本任务、只读plan.md、执行计划/创建评测技能及完整原题instruction、README、task.toml、Dockerfile、solution、tests；未改上游、公共文件或他题文件。
- 本题目录保留未暂存草稿：四道独立合成题（静态服务、真实请求字段、404/日志边界、请求验证）、黑盒真实Nginx checker、四gold与12语义负例、20项测试。未声称TDD红绿闭环完成。case.image目前仍为缓存Python工具base，不能当作最终含Nginx镜像；README明确恢复时须换成真实构建digest。
- 主调度暂停新Docker builder/工具镜像/标准Harbor/父下载及校准，等待61审计入口资源限制与串行窗口；本会话未启动任何构建、下载、父容器或标准Harbor。受限离线缓存探针退出1，shutil.which('nginx')为None。checker将缺Nginx显式退出2，避免把基础设施错误计作nop/负例通过。无付费调用，无评分/模型/预算变更。
- 指定unittest命令退出1：20测试、20基础设施错误、有效行为控制0；指定diagnostics validate --run-fixtures退出1，首个gold因缺Nginx失败；单独无fixture格式校验退出0，但不是校准证据。指定父preflight退出1：公共runtime.json缺historical_root（KeyError），未改公共runtime。日志位于evals/promptfoo/results/qwen38-suite/diagnostics/nginx-request-logging/{unittest,validate,preflight}-blocked.log。
- evals/promptfoo/score.sh test退出0；cargo clippy --all-targets --all-features -- -D warnings退出0。实际动态槽位1，CARGO_TARGET_DIR=/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1，CARGO_BUILD_BUILD_DIR相同，CARGO_BUILD_JOBS=1；有效Rust行为测试0（Clippy不运行测试）。自有锁已释放，缓存保留；无Rust变更，无需fmt。block 0.1.6既有未来兼容提示未修改。
- 阻塞恢复：等待主调度核查并授权受限串行构建窗口；构建只含工具的Nginx镜像并记录真实单manifest digest，更新本题case.image；先补checker缺陷红测/修复，再跑20项fresh fixture和8项标准Harbor控制。按task2容量规则核验min(host,VM)、全部副本与正数硬Ucap，准入后原父官方fresh oracle/nop；使用ignored私有execution runtime引用既有runtime-v2，不改官方评分或公共report。补足镜像/依赖/fixture/checker哈希和资源隔离收据后才可完成。
- 当前无本题容器、子进程或Cargo锁。按阻塞分支只更新本编号文件并保留未暂存资产，不提交、不删任务文件；plan.md保持只读。


## 公共入口修复后的工具前置审查（2026-10-28）

- 已读suite/BUILD_RESOURCES.md，公共修复964419691使用suite.docker:SuiteDockerEnvironment，在RUN前预设CPU/memory并禁止up隐式重建；没有复制私有compose包装或修改Harbor/Trial/FROM/platform。67优先标准控制，本题仍未启动builder/父下载/校准。
- 当前缓存只读审查：task52-single digest32c8a19c…为真实amd64 Debian12，uname x86_64，Python /usr/local/bin/python3；libc6=2.36-9+deb12u13、libpcre2-8-0=10.42-1、libssl3=3.0.17-1~deb12u2、zlib1g=1:1.2.13.dfsg-1已存在，nginx缺失且apt列表为空。工具base inspect.Size=330438807为缓存内容大小，非已知解压U。task23 digest681f…为arm64，不能代替实际amd64 Agent工具环境。旧业务镜像nginx1.13.4虽amd64但缺Python且依赖旧OpenSSL/libpcre，未复制其内容、不操作业务容器。
- 最小工具方案保持现有固定task52 FROM，仅apt --no-install-recommends安装Debian nginx及必要包，阻止安装期服务启动、删除apt列表，实测nginx -V/ldd/Python；工具源不带上游/题目/评分材料。确切包版本、下载总量及最终digest须构建窗口中测定，当前不能伪报已知。实际Nginx需测试自有临时pid/error/access/body/proxy等路径能在只读rootfs下运行。
- 诊断既有独立profile来自suite.diagnostics.materialize：1CPU/512MiB/storage1024MiB。容量提案保留B=2GiB/A=1GiB/余量5GiB，不改父S=10GiB；三份压缩C硬cap512MiB + 完整U正硬cap2GiB + S1GiB+B2GiB+A1GiB+5GiB =12.5GiB（13421772800字节）。此为待实施的停止预算，不是已测量上界或已通过构建准入。缓存base实际完整U及nginx包总量仍待核验；所有副本/归档均须计入cap，超限停止并重新准入，不能只用docker inspect Size或VM空闲代替。
- 只读探针均在network none、0.25CPU/128MiB、cap-drop ALL、no-new-privileges、read-only下短暂运行并自动删除，无工具构建/新下载。本次host/VM及提案见tools-capacity-proposal.json；cache-inspect.jsonl、base-closure.txt、old-nginx-audit.txt保留实际审查。等待67结束串行窗口，再做真实依赖闭包与TDD红绿；此前20错误及公共runtime缺字段日志全部保留，不声称就绪。


## 串行工具窗口执行结果（2026-10-28；此前阻塞日志保留）

- 按主调度恢复指令已完成独立工具路径；此前“缺Nginx/暂停builder/待窗口”仅为历史检查点，现已解除。任务状态仍已阻塞，唯一实质剩余是原父完整容量与校准，不能以子题证据冒充整题就绪。plan.md、上游/他题/公共文件未改，无付费调用。
- 当前base为原固定amd64 task52-single；只读APT metadata容器预设1CPU/512MiB及相同swap、rootfs只读，列表/缓存/tmp分别硬限64/64/16MiB。官方已签名metadata解析8个包，共1943416字节，nginx=1.22.1-9+deb12u10；按metadata SHA256逐包HTTPS读取并执行精确size硬截止，未换父镜像或平台。environment/dependencies.json记录包名/版本/官方URL/大小/哈希，archives只存ignored目录。
- 全部底座层经streaming docker save逐个解压、比对diff_id，按重复RootFS引用计U=910168064；不将inspect.Size当U。新增deb data/control tar在解压前逐块累计硬截止，base+deb预检916353024字节。工具实际构建命令见tools-build-resources.json：DOCKER_BUILDKIT=0 docker build --cpu-period=100000 --cpu-quota=100000 --memory=512m --memory-swap=512m --network=none -t vw-nginx-diagnostics:task63 tool-context；退出0。实际RUN abcdf3241fd9捕获CpuPeriod=CpuQuota=100000，Memory=MemorySwap=536870912。未先无界后update，未改安装Harbor、Trial、私有compose或公共后端。离线COPY已验证包，policy-rc.d exit101阻止安装期服务启动；仅工具材料进入镜像。
- 实际true单OCI manifest sha256:ef27f9fbbcb46c21f223adc5915ab60c676351bdafe3896814155568f0497b3a，config sha256:a47817ba8a50e0641719b0063c58f5869463be3943f07accaab2c60723902155，amd64；最终compressed layers C=335161124、U含全部重复RootFS引用=920099840字节。全部diff_ids/config/manifest匹配，均未超Ccap512MiB/Ucap2GiB。streaming保存/导出未落完整归档；APT元数据、新包、build context及缓存副本均在3C+B/A预算内；独立S1GiB、B2GiB、A1GiB、安全5GiB保留，准入12.5GiB，构建前host约18680561664、VM92024901632。见base-layer-sizes、dependency-U、tools-identity及capacity提案与原始日志。
- 真实只读离线probe通过nginx -V、ldd、Python3.13.7、uname x86_64，/app为空且无/solution或/tests；现有冻结Agent与runtime sidecars只读挂载，/runtime/shiju --help退出0，没有启动付费请求。旧业务nginx镜像仅只读审查，不复用其库，不操作业务容器。
- 首轮真实红测发现nginx默认/var/lib/nginx/body在只读rootfs失败，gold及合成题面补齐ROOT下五种temp path。另一次checker状态码漏检mutation令m2/wrong-status被误收，unittest退出1（20测试1失败）；恢复完整HTTP字段比较后指定unittest退出0，20有效控制。指定diagnostics validate --run-fixtures退出0，四题各gold/nop/3负例共20 fresh离线控制；validation.json记录hash。之前缺nginx20错误、过窄OCI类型断言和真实rootfs红测均单列保留，未算通过。
- 新公共suite.scoring.harbor_trial→SuiteDockerEnvironment标准控制退出0：8个fresh Trial，m1..m4各oracle=1/nop=0；全部first score有效，无recovery/回放、没有私有platform/Trial/compose包装。原materialize的FROM(ref@digest)/Dockerfile/资源字节保持不变；逐项实际main=1CPU/512MiB、sidecar=0.25CPU/64MiB、amd64层链均断言通过；execution identity末尾再次核对无漂移。receipt children-calibration-production-backend.json，逐项production-*-resources.json；源完整inputs核验见source-checkpoint.json。
- 代码完成后最终Clippy退出0，动态slot1，CARGO_TARGET_DIR=/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1；CARGO_BUILD_BUILD_DIR同绝对路径、CARGO_BUILD_JOBS=1。命令cargo clippy --all-targets --all-features -- -D warnings；有效Rust测试0，不替代20+8行为证据。自有锁已释放、缓存保留；既有block未来兼容提示未改。evals/promptfoo/score.sh test退出0。未改Rust、未跑fmt/全量test/wasm/Makefile。
- 父容量严格单列：官方S10GiB+B2GiB+A1GiB+安全5GiB必要预留已达19327352832字节，当前host18660679680，仅固定下界就缺666673152，且尚须正C/U，不能准入。这里没有将未知C/U按0计入；用必要下界即可拒绝。未新下载父层或启动父oracle/nop，历史公共report缺historical_root日志未覆盖；私有runtime仅用于已通过的子题/inputs，未借此绕父容量。
- 当前已退出本题controller/子进程，删除自有容器，保留工具镜像/包/证据缓存；无自有Cargo锁。**本题串行工具构建/标准校准窗口已释放**。required-checks.json明确20fixture+8标准控制及剩余父检查。按阻塞分支保留未暂存独立资产，只更新本编号任务文件，不提交就绪、不删除编号文件。
- 恢复入口：先按min(host,VM)重新准入父S10GiB/全部C副本/正硬Ucap/B2/A1/安全5，核验官方原manifest/config/layers及真实依赖，再通过原父官方fresh oracle/nop与完整preflight/preparation；不足不得扩大预算、减少原profile或清理非本题资产。原定模型/输出32000/输入200000/900秒父超时及官方评分未改。


## 当前CE独立诊断复验（2026-10-28；仅diag，等父题窗口）

- 按主调度要求与61最多两活动，61独占父题窗口；本阶段没有父镜像层下载/导入、父oracle/nop、付费模型或公共代码/题面/评分/预算修改。当前精确状态为**已阻塞（等父题窗口）**；旧容量快照/旧f759标准8只作历史证据，不冒充当前CE或当前父准入。
- 当前CE=ce05045daeb0b6f9a28b5cd179f2f4363cfbbaf143a099565e27218a494a4bd1。工具manifest ef27f9f…、amd64与完整RootFS层链和原tools-identity完全匹配，复用已stream逐层验证的exactU=920099840及C=335161124证据；没有用inspect.Size推断U，也没有将未知新增量记0。case/checker/diagnostics实现/image/policy全哈希未变，核验validation.json后复用原20fixture；reuse-check.json记录全部对比，未改case或checker。
- 每项新标准控制前重测min(host,VM)，再扣61未来增长26GiB；既有独立S1+B2+A1+安全5=9GiB，新增构建/副本另明确正预算2GiB，完整diag峰值11GiB。工具和基底已缓存且identity匹配，不重复下载或复制完整层；标准materialize只COPY既有小fixture，未新增RUN/下载/大归档。八次实际准入全部覆盖11GiB，最低扣除61后可用值见stage-final.json；实际运行资源逐项归档，保留业务容器。
- 固定冻结ELF连现有sidecars只读挂载，在实际Nginx amd64工具环境offline/read-only --help再次退出0，未含密钥、未发模型请求；runtime-closure-probe.log保存原始uname/nginx-V/ldd/Python/help证据。没有新的controller私有platform/FROM/Trial/Compose/Builder覆盖。
- Fresh8正式suite.scoring.harbor_trial→SuiteDockerEnvironment退出0，m1..m4 oracle=1、nop=0，全部first score有效，无恢复/回放；实际main=1CPU/512MiB、sidecar=0.25CPU/64MiB及amd64层链断言通过。新目录diag-current-ce05045/保存新materialize/task hashes、逐项资源、控制日志和children-calibration-production-backend.json；末尾CE再次核验一致。旧f759所有证据原样保留。
- 本阶段无实际代码变更，依指令没有额外Cargo/Clippy，不动Cargo缓存或任何他人锁；完整父任务收尾前仍须按动态slot/JOBS1执行Clippy。未改plan.md、上游、评分、原父Agent环境或超时；仅本任务文件更新与ignored诊断证据。保留此前未暂存独立资产，不宣称全题就绪、不提交/删编号任务。
- 八控制controller已退出、本题临时容器已全部清理，**本阶段无活动资源**；stage-final.json记录精确阶段状态和容量。下一步只等待主调度父题窗口及重新完整父准入；本会话不占父窗口、不抢61未来增长预留。


## 公共工具互斥窗口收尾（2026-10-28）

- 收到暂停新工作的调度时，当前CE ce05045轮8项正式控制此前已经自然完整结束，均有有效first；本轮剩余未启动项0，未重复任何已完成项。全部旧证据保留，公共身份若改变，旧CE控制不得作为新身份证明。
- 精确状态：已阻塞（等公共工具稳定/父窗口）。此阶段仅核对资源与写本任务检查点，没有启动新控制、构建、父题或Cargo，不改原预算、源评分或Agent环境，不触碰公共互斥修改。
- 本题进程0、容器0、Cargo锁0；public-mutex-checkpoint.json保存核对结果。等待7稳定后由原会话恢复，父题窗口仍未占用。


## b35499当前身份独立diag完成（2026-10-28）

- 精确状态：**已阻塞**。原因：当前身份独立诊断已完成，等待另授父题窗口；旧空间阻塞与旧CE仅为历史。仅执行1–90范围内本题diag，未执行91以后或付费子集。62独占父窗口，本阶段没有父层下载/导入/父控制、公共代码或原题/参考解/评分/Agent依赖/预算/超时修改。
- CE=b35499b54cd173f716f856c17ecb9baf971ef1326c5b4ebc4656adf9d95fb5df。case所有声明/checker/diagnostics实现/image/policy哈希全同，原20fixture经完整核验复用；实际缓存工具SHA/amd64/RootFS与原stream exactU920099840及C335161124匹配，不把inspect.Size当U或未知作0。固定runtime-v2 ELF与sidecars在真实Linux/x86_64 Nginx工具环境无密钥offline/read-only --help退出0，原始log保留。
- 每项重测min(host,VM)扣62未来28GiB；初次扣后20557930496字节，后续最低值见stage-final.json，均覆盖完整diag峰值11GiB（S1+B2+A1+安全5=9GiB，新增构建/副本正预算2GiB）。仅复用已验证缓存、标准小fixture COPY，没有新大归档或新工具构建，未借VM空间代替host、未减少原profile或清理他题/业务/Cargo/唯一OCI/评分证据。
- 当前身份有效fresh8正式suite.scoring.harbor_trial/SuiteDockerEnvironment全部通过，四题oracle=1/nop=0且均有效first，无recovery/评分回放。实际main1CPU/512MiB、sidecar0.25CPU/64MiB与amd64层链断言通过；原materialize FROM/ref/字节不改，无私有platform/Trial/Compose/Builder替代，收尾inputs身份仍一致。
- 另保留1次m2 nop启动前争用公共锁的BlockingIOError，unscored。核验锁已空闲后在resume-after-shared-lock新Trial路径续跑5项；原3项有效first未重复。总9次Trial含8有效+1基础设施失败，失败原raw/first与旧CE全部保留、不改hash冒充新证明、不删或绕过共享锁。两份children-calibration-production-backend.json、各controls/resource/first、reuse-check、ELF probe与stage-final均在diag-current-b35499/。62旧capacity-probe限制未伪造恢复。
- 本阶段无评测代码/资产变更，未额外运行Cargo；完整任务完成前仍须按原契约动态slot/绝对target与builddir/JOBS1/Clippy及聚焦验证、串行提交自身资产。当前仅任务状态与ignored证据，保持编号文件、不提交就绪；plan.md只读。
- 收尾进程清点首次遇到系统ps输出非UTF8，改用errors=replace只读解码后完成核验，不涉及评测或评分变化。**当前本题进程0、容器0、锁0，独立diag窗口已释放**。下一步只等待主调度父窗口后重测完整父S10GiB/C/U/全部副本/B/A/安全余量，完成官方父oracle/nop与准备验收；本阶段不称全题就绪。
