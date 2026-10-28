# 任务 31: 准备-dna-insert

批次：【批次 17】；依赖批次：1、2、3、4、5。同批无文件依赖；Git 提交按任务编号串行，禁止删除其他会话的索引锁。

状态：已阻塞

目的：为“环状序列插入”准备原题完整环境与四道独立诊断题。

来源任务：Terminal-Bench2.1/dna-insert

预计会话范围：只准备一个父题的四个窄诊断题和确定性校准；不运行付费模型。长镜像拉取允许记录进程后续接，不能把未校准标成完成。

## 文件

- 新建：`evals/terminal-bench/suite/cases/dna-insert/case.json`、`evals/terminal-bench/suite/cases/dna-insert/check.py`、`evals/terminal-bench/suite/cases/dna-insert/tests/test_cases.py`、`evals/terminal-bench/suite/cases/dna-insert/fixtures/`、`evals/terminal-bench/suite/cases/dna-insert/oracle/`、`evals/terminal-bench/suite/cases/dna-insert/environment/Dockerfile`、`evals/terminal-bench/suite/cases/dna-insert/README.md`
- 只读完整上游包：`evals/promptfoo/results/qwen38-suite/source/tasks/dna-insert/`
- 忽略产物：`evals/promptfoo/results/qwen38-suite/preflight/dna-insert/`、`evals/promptfoo/results/qwen38-suite/diagnostics/dna-insert/`

## 上下文

原题 dna-insert；类别 scientific-computing；CPU=1、内存=4096MiB、存储=10240MiB、GPU=0；Agent=1800秒、verifier=1800秒。预建镜像 alexgshaw/dna-insert:20251031。原题树SHA=a88b1748a1857e70022d95dcbb39ca0c5bd813ae；题面SHA256=49fce0ee8b73d7ccdf4b8476c6bb87cf362689fd1d77b21a0d8737d16019a473；配置SHA256=1a9efd5c826d64d31c4874b395f1f9c488b2f26581b0c752fc4619ff177f25eb。读完整 instruction.md、输入、solution、tests 后设计，禁止改原题。四子题：m1 定位环状序列差异；m2 设计小型引物候选；m3 检查方向和互补约束；m4 验证目标序列重建。

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

先使用技能 ./skills/create-agent-evals/SKILL.md；每个子题定义合成输入、清楚完整题面、输出路径/API/服务行为与独立确定性判定。新增 evals/terminal-bench/suite/cases/dna-insert/tests/test_cases.py：正解通过，至少3个有意义错误变体被拒绝；运行真实程序/产物，不验证关键词、自述或仅Agent自测。错误fixture和漏检checker先失败，再最小修复。

验证命令：

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/dna-insert/tests -p test_cases.py
```

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/dna-insert/case.json --run-fixtures
```

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.run preflight --catalog evals/terminal-bench/suite/catalog.json --runtime evals/promptfoo/results/qwen38-suite/runtime.json --parent-id dna-insert --output evals/promptfoo/results/qwen38-suite/preflight/dna-insert
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

## 执行中证据（2026-10-27）

- 已读取执行计划/创建评测技能、plan.md、完整原题题面/序列/参考解/评分与配置；核对批次1–5的冻结source/catalog、容量、实验契约、v2运行时及Agent适配层收据。plan.md未修改。
- 新增本题四个独立合成子题：m1环状插入定位，m2小型引物枚举（明确使用合成Wallace公式，不代替官方oligotm），m3方向/反向互补，m4目标重建及字节SHA256。12个有意义错误产物；宽松checker先失败27个拒绝断言（25测试，退出1）。独立复算发现并修复m1合成坐标歧义；最终26测试通过（退出0），含四gold独立计算。
- 指定`suite.diagnostics validate --run-fixtures`退出0：20个fresh离线容器控制通过；四子题8次fresh Harbor oracle/nop均为1/0。validation.json绑定全部声明字节。`evals/promptfoo/score.sh test`退出0。
- `cargo clippy --all-targets --all-features -- -D warnings`退出0，动态领取槽位2，CARGO_TARGET_DIR及CARGO_BUILD_BUILD_DIR均为`/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-2`，CARGO_BUILD_JOBS=1；自有锁已释放，缓存保留；无Rust修改。
- 原镜像digest=`sha256:4dd8760694e355de85ecf03605cb487ccdb33322f6afad513d2920829baa33b2`、config=`sha256:2e68029a49d11b454994967b2eddb94333ef321b688f865ada3c09a3b534c110`，linux/amd64。压缩29724629字节、解压80645632字节；Registry/OCI的manifest/config/layers/diff_ids及本机RootFS链全部核验。读取已核验相同基础层，仅写本题目录。受限启动确认x86_64、memory.max=4294967296、cpu.max=100000/100000，官方输入7238字节；全部9个官方文件与catalog SHA256一致。
- 指定历史runtime.json缺historical_root，原始preflight退出1；本题忽略目录runtime.json使用已验证v2与不变source/cases路径，保留官方1800秒agent/verifier时限。当前真实原题preflight为`preflight/dna-insert-original/`，本地session=11583，容器`dna-insert-oracle__env-main-1`；未改环境、参考解、评分、模型、预算，无模型调用。
- 原题参考解apt索引阶段多次Ign后完成，正在官方`apt-get install -y emboss primer3`的61MB emboss-data下载；目前尚无原题评分，不宣称完成。剩余原始时限继续运行；不得用verifier-only修复掩盖agent阶段问题。
- 确切命令、退出码、依赖/镜像/源码/fixture/checker身份、资源和失败日志位于忽略目录`evals/promptfoo/results/qwen38-suite/diagnostics/dna-insert/`（commands.json、clippy-slot.json、green.log、fixtures-final.log、children-calibration.json、parent-identity.json、parent-probe.json等）。当前资产未提交；提交遵守恢复后的29→31顺序，不持有Git索引锁等待。

## 最终阻塞结论（2026-10-27）

- 本题官方oracle真实执行1800秒后抛出`AgentTimeoutError: Agent execution timed out after 1800.0 seconds`。官方`apt-get install -y emboss primer3`在61.0MB的emboss-data上反复Ign/重试，未能在时限内完成安装和生成primers.fasta；没有用超时后产物算通过。
- preflight最终退出1，summary=`score_status: unscored`、`failure_class: infrastructure`、`reward: null`。随后官方verifier受仍运行apt的dpkg锁阻挡，curl/uvx缺失；原始reward=0不能当能力失败或有效校准。父题nop未运行；缺少整题oracle/nop，故不能标记就绪或仅待回归。
- Harbor已完成异常收集并自动结束本次runner、清理本题原题容器；未继续漫长verifier下载。原始exception.txt、agent/oracle.txt、verifier/test-stdout.txt、result.json、scoring/first.json及preparation.json均保留于`preflight/dna-insert-original/`。没有重复同条件1800秒校准、预装Agent依赖、修改原题环境/评分/预算/时限，未调用模型。
- 授权范围内已解决镜像导入及runtime执行配置问题，验证源码/镜像/资源/诊断资产；剩余阻塞在原始Agent阶段下载，verifier-only修复不能解决，需官方Ubuntu下载链路能在原始时限内完成依赖安装。
- 最终有效证据：26聚焦测试、20个fresh fixture控制、8次fresh Harbor子题oracle/nop通过；Promptfoo配置及Clippy退出0。实际Cargo槽位2，CARGO_TARGET_DIR=`/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-2`，CARGO_BUILD_BUILD_DIR同路径、jobs=1，自有锁已释放、缓存保留。
- 汇总收据`diagnostics/dna-insert/acceptance.json`及精确命令`commands.json`。保留本任务文件及未提交诊断资产，不提交、不删除。恢复入口：官方同源网络恢复后，以本题忽略目录runtime.json重新执行preflight，选择新的输出目录；不能覆盖失败目录或扩大预算。全部父题oracle/nop通过后再核验收据，按主会话协调的提交顺序提交资产并删除编号任务。

- 清理后仅做一次20秒上限的同源HEAD恢复审计：原digest、显式amd64、1CPU/4096MiB、只读根、无新增依赖，HTTP200、Content-Length=61018452、退出0。它只证明可达，不证明持续传输能在原时限完成；无权据此重跑或改变环境。审计容器已清理，docker ps -a按dna-insert过滤为空；结果见final-recovery-audit.json。
