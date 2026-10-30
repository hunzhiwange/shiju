# 任务 72: 准备-protein-assembly

批次：【批次 41】；依赖批次：1、2、3、4、5。同批无文件依赖；Git 提交按任务编号串行，禁止删除其他会话的索引锁。

状态：已阻塞

目的：为“融合蛋白序列设计”准备原题完整环境与四道独立诊断题。

来源任务：Terminal-Bench2.1/protein-assembly

预计会话范围：只准备一个父题的四个窄诊断题和确定性校准；不运行付费模型。长镜像拉取允许记录进程后续接，不能把未校准标成完成。

## 文件

- 新建：`evals/terminal-bench/suite/cases/protein-assembly/case.json`、`evals/terminal-bench/suite/cases/protein-assembly/check.py`、`evals/terminal-bench/suite/cases/protein-assembly/tests/test_cases.py`、`evals/terminal-bench/suite/cases/protein-assembly/fixtures/`、`evals/terminal-bench/suite/cases/protein-assembly/oracle/`、`evals/terminal-bench/suite/cases/protein-assembly/environment/Dockerfile`、`evals/terminal-bench/suite/cases/protein-assembly/README.md`
- 只读完整上游包：`evals/promptfoo/results/qwen38-suite/source/tasks/protein-assembly/`
- 忽略产物：`evals/promptfoo/results/qwen38-suite/preflight/protein-assembly/`、`evals/promptfoo/results/qwen38-suite/diagnostics/protein-assembly/`

## 上下文

原题 protein-assembly；类别 scientific-computing；CPU=1、内存=4096MiB、存储=10240MiB、GPU=0；Agent=1800秒、verifier=1800秒。预建镜像 alexgshaw/protein-assembly:20260403。原题树SHA=848a3c18e3d82ba9485ec5edb42b7bcce0c5831e；题面SHA256=1bb2af1d85e9a823a8ae7273a87df8a3f67e2c93b06cddf34f0051a8b003f7c5；配置SHA256=9b91f32925035fba7b5a1db16c7d4ed650a117af145d3d3f8e514cb4b1e970ab。读完整 instruction.md、输入、solution、tests 后设计，禁止改原题。四子题：m1 解析合成序列与光谱约束；m2 选择符合条件的结构域；m3 检查连接和窗口组成约束；m4 验证完整编码序列交付。

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

先使用技能 ./skills/create-agent-evals/SKILL.md；每个子题定义合成输入、清楚完整题面、输出路径/API/服务行为与独立确定性判定。新增 evals/terminal-bench/suite/cases/protein-assembly/tests/test_cases.py：正解通过，至少3个有意义错误变体被拒绝；运行真实程序/产物，不验证关键词、自述或仅Agent自测。错误fixture和漏检checker先失败，再最小修复。

验证命令：

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/protein-assembly/tests -p test_cases.py
```

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/protein-assembly/case.json --run-fixtures
```

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.run preflight --catalog evals/terminal-bench/suite/catalog.json --runtime evals/promptfoo/results/qwen38-suite/runtime.json --parent-id protein-assembly --output evals/promptfoo/results/qwen38-suite/preflight/protein-assembly
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

## 2026-10-28 主机准备检查点

主调度确认依赖批次1–5满足；按照共享资源调度要求暂停全部新Docker活动，等待窗口后续接。未标记ready，未提交或删除任务文件，plan.md未修改。

已读取全部11个上游文件并逐文件核对catalog SHA-256。已创建四道独立合成题、黑盒checker、gold、每题3个错误变体、tests及README。初始宽松checker真实红灯17项断言失败；修复后指定unittest命令退出0，3个测试包含20组gold/nop/负例控制，m1/m2/m3正解分别运行6/8/13个输入，另覆盖m4同义密码子、连接肽边界和非法交付。

证据目录：`evals/promptfoo/results/qwen38-suite/diagnostics/protein-assembly/` 的red.log、green.log、host-checkpoint.json、clippy.log及clippy-metadata.txt。仅主机证据，不是容器校准。Clippy命令 `cargo clippy --all-targets --all-features -- -D warnings` 退出0；实际动态槽位1，CARGO_TARGET_DIR与CARGO_BUILD_BUILD_DIR均为 `/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1`，JOBS=1，自有锁已释放、缓存保留。没有Rust变更，无全量测试/fmtcheck/wasm/Makefile。

待恢复：获得主调度Docker窗口后核实通用单leaf镜像及真实amd64 Agent闭包、资源空间收据，再跑指定diagnostics validate --run-fixtures、标准SuiteDockerEnvironment原题oracle/nop和四子题fresh Harbor校准。历史runtime.json缺historical_root，使用忽略目录内验证runtime-v2 execution config。没有任何Docker命令、付费模型调用、官方评分/模型/预算变更或公共代码修改。真实容器证据齐备前不完成、不提交、不删编号文件。

补充：`evals/promptfoo/score.sh test` 退出0（Configuration is valid）；日志promptfoo-validation.log。仅既有配置静态验证，没有模型调用。

阻塞原因：等待主会话分配新的Docker窗口，69父阶段优先。主机20组控制与Clippy已通过，但原父题及容器校准证据尚未齐备；收到新窗口后续接。


## 2026-10-31 突破调度阻塞与真实容器复核

状态：**已阻塞（阻塞已更新为上游参考解失败，已解除旧 Docker 窗口等待）**。诊断资产已完成本轮确定性校准；完整父题未通过，不标 ready，不删除任务文件、不修改 plan.md。

- 修复本题 checker 的真实容器导入冲突：从 `/app` 启动时，候选 `select.py` 遮蔽 stdlib `select`，导致 checker 基础设施 exit 2。仅在本题 checker 导入依赖前移除候选工作目录搜索路径；候选程序仍以 `-I` 执行。新增 fresh-process `json.py` 遮蔽回归（macOS 的 select 是内建模块），先红 1 项失败，再修复转绿。没有修改公共 Suite/Harbor、被测 Agent 或原题。
- 指定 unittest 命令退出 0：4 个测试，含 20 组 gold/nop/语义负例及格式、同义密码子、连接肽边界、导入隔离覆盖。指定 diagnostics CLI `validate --run-fixtures` 退出 0：20 个 fresh、离线、受限容器，4 gold 通过、4 nop 与 12 负例拒绝。`evals/promptfoo/score.sh test` 退出 0，仅静态配置校验，无模型请求。
- 最终独立窗口 `preflight/protein-assembly-children-windowed-20261031/children-calibration.json`：4 子题各一次 fresh 标准 Harbor oracle/nop，共 8 Trials，全部 oracle=1、nop=0；含每次磁盘准入、公共 backend 资源收据、镜像构建日志及实际容器 HostConfig 采样。最终窗口显式检查其他父题数量并串行执行；早先共享资源竞争与磁盘不足的中断证据原样保留，不作为最终窗口替代物。
- 原镜像 `alexgshaw/protein-assembly:20260403` digest 为 `sha256:602c467ae6b661a73441b960d9fa7130f068f4c6041d37ef8632611733d83f4a`，config digest `sha256:2a47b0bd852b71effd01caaded10f3897b25712c8b87b0921a5f67079338ac5f`。压缩层总计 44,235,754 字节，解压 tar 总计 125,246,464 字节；下载内容 SHA、diff-ID、导入配置/层链全部核验。镜像内三份输入与上游源逐一 SHA 一致；全部 11 个官方文件保持 catalog 哈希。宿主/VM 空间按较小值准入，初次导入保留三份 C、512MiB 解压硬上限、原 S/B/A 和 5GiB 余量；缓存复用与实际下载 44,235,754 字节分开记录。中途空间收紧曾拒绝启动，恢复后重新准入。
- 原题真实标准 Harbor oracle 已执行至未改动的官方 verifier，reward=0（预期应为 1）；另补跑独立原题 nop，reward=0。原题实测 1 CPU、4GiB 内存且 MemorySwap=Memory，保留 storage=10240MiB 与 agent/verifier 各 1800 秒。参考解的 `gblock.py` 在 DNAChisel `EnforceTranslation` 失败，未生成 `/app/gblock.txt`。容器内网络已成功完成抗体靶标、PubChem、PDB、FPBase 与 verifier 依赖获取。
- 已确定真实根因：5WJ2 canonical 序列第 86 个零基位置与 2H5Q 第 70 个零基位置各为 `X`；当前 PDB REST 两份响应均无参考解依赖的 modified_monomer features。按官方参考解去初始 M、拼接域与连接肽后，两处 X 恰好映射至核苷酸 `[324,327)`、`[1611,1614)`，与 DNAChisel 错误逐一一致。这是原题 Agent/参考解阶段失败，不能用 verifier-only 依赖修复解决。没有重跑/替换原题参考解、伪造 API、硬编码官方序列、放宽评分或用诊断成功冒充父题通过。
- 新鲜 Clippy `cargo clippy --all-targets --all-features -- -D warnings` 退出 0；动态槽位 **3**，`CARGO_TARGET_DIR` 与 `CARGO_BUILD_BUILD_DIR` 均为 `/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-3`，`CARGO_BUILD_JOBS=1`。有效行为验证为前述 4 个 unittest、20 容器控制、8 子题 Trials 与 2 父题 Trials；Clippy 本身有效测试数为 0。自有槽位 3 锁已释放，缓存保留。先前槽位 2 的运行退出 101（临时 rmeta 目录不存在，随后自有 owner.json 也消失），仅保留原始日志；没有删除当时已重建的他人槽位锁或缓存。未修改 Rust，未执行全量 cargo test、fmt check、wasm 或 Makefile。

主要证据均在忽略目录 `evals/promptfoo/results/qwen38-suite/diagnostics/protein-assembly/resume-20261030/`：`import-shadow-red.log`、`import-shadow-green.log`、`validate-final.log`、`runtime-probe-final.log`、`parent-image-inputs-verified.json`、`parent-oracle-failure.json`、`current-pdb-api-diagnosis.json`、`parent-nop.log`、`children-windowed.log`、`children-windowed-actual-inspects.json`、`clippy-final.log` 与 `clippy-final-metadata.json`。父题失败预检完整收据在 `preflight/protein-assembly-20261031-cached/`；最终子题证据在 `preflight/protein-assembly-children-windowed-20261031/`。原始调度/容量失败记录保留，成功的 diagnostics validation.json 不等于 preparation/ready receipt；本题根目录没有伪造 preparation.json。

恢复入口：由上游恢复与冻结参考解相容的 PDB 数据契约，或先明确更新上游冻结源与 catalog，再修复参考解的修饰残基解析；不能在本任务内改只读原包或悄悄接受哈希漂移。恢复后使用忽略目录 `resume-20261030/runtime.json` 与新输出目录重跑标准完整 preflight，要求原题 oracle=1/nop=0 后才能解除阻塞。模型仍固定 token-plan/qwen3.8-flash、32000 输出、200000 输入窗口；本轮没有付费调用。

提交检查清理了 9 个错误 fixture 的多余末尾空行，随后重新运行 4 个 unittest 与 20 个容器控制并刷新 validation.json。`child-materialization-reverified.json` 证明最终字节重新物化的四任务与已通过的 8 次 Harbor 任务逐字相同；`final-checkpoint.json` 绑定最终收据及依赖哈希。
