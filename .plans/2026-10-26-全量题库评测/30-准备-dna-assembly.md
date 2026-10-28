# 任务 30: 准备-dna-assembly

批次：【批次 16】；依赖批次：1、2、3、4、5。同批无文件依赖；Git 提交按任务编号串行，禁止删除其他会话的索引锁。

状态：已阻塞

目的：为“序列片段装配”准备原题完整环境与四道独立诊断题。

来源任务：Terminal-Bench2.1/dna-assembly

预计会话范围：只准备一个父题的四个窄诊断题和确定性校准；不运行付费模型。长镜像拉取允许记录进程后续接，不能把未校准标成完成。

## 文件

- 新建：`evals/terminal-bench/suite/cases/dna-assembly/case.json`、`evals/terminal-bench/suite/cases/dna-assembly/check.py`、`evals/terminal-bench/suite/cases/dna-assembly/tests/test_cases.py`、`evals/terminal-bench/suite/cases/dna-assembly/fixtures/`、`evals/terminal-bench/suite/cases/dna-assembly/oracle/`、`evals/terminal-bench/suite/cases/dna-assembly/environment/Dockerfile`、`evals/terminal-bench/suite/cases/dna-assembly/README.md`
- 只读完整上游包：`evals/promptfoo/results/qwen38-suite/source/tasks/dna-assembly/`
- 忽略产物：`evals/promptfoo/results/qwen38-suite/preflight/dna-assembly/`、`evals/promptfoo/results/qwen38-suite/diagnostics/dna-assembly/`

## 上下文

原题 dna-assembly；类别 scientific-computing；CPU=1、内存=2048MiB、存储=10240MiB、GPU=0；Agent=1800秒、verifier=1800秒。预建镜像 alexgshaw/dna-assembly:20251031。原题树SHA=a6397a25168ff3b94a78a728b10d1402aee435b2；题面SHA256=88998c98beabdd966ab46c04ede822bbc74796696aa1bf87c567a9131210533c；配置SHA256=7b571544b64d9be1c91526837e5d805c2181afe0e6fd57a7f0676bba36c868d7。读完整 instruction.md、输入、solution、tests 后设计，禁止改原题。四子题：m1 读取与定向合成序列；m2 生成小型拼接方案；m3 校验连接和反向互补；m4 导出可复算的序列结果。

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

先使用技能 ./skills/create-agent-evals/SKILL.md；每个子题定义合成输入、清楚完整题面、输出路径/API/服务行为与独立确定性判定。新增 evals/terminal-bench/suite/cases/dna-assembly/tests/test_cases.py：正解通过，至少3个有意义错误变体被拒绝；运行真实程序/产物，不验证关键词、自述或仅Agent自测。错误fixture和漏检checker先失败，再最小修复。

验证命令：

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/dna-assembly/tests -p test_cases.py
```

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/dna-assembly/case.json --run-fixtures
```

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.run preflight --catalog evals/terminal-bench/suite/catalog.json --runtime evals/promptfoo/results/qwen38-suite/runtime.json --parent-id dna-assembly --output evals/promptfoo/results/qwen38-suite/preflight/dna-assembly
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

- 已核对批次1–5资产、catalog源逐文件SHA256、generation-2冻结运行时及Harbor 0.24.0源码身份；plan.md只读。
- 本题四子题资产、12个错误产物已准备；宽松checker先触发24个拒绝断言失败（22测试，退出1），修复后25个聚焦unittest通过（退出0），四gold亦经独立计算核对。
- 指定`suite.diagnostics validate --run-fixtures`退出0：20个fresh离线容器控制通过；四子题8次fresh Harbor oracle/nop全部符合1/0。validation.json绑定全部声明哈希。`evals/promptfoo/score.sh test`退出0。
- Clippy最终退出0。动态领取槽位1，CARGO_TARGET_DIR和CARGO_BUILD_BUILD_DIR均为`/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1`，CARGO_BUILD_JOBS=1；自有锁已释放，缓存保留。初次Clippy被其它会话installer_update_tests.rs缺失符号阻断（101），其代码提交后重查通过；未修改Rust。
- 原镜像digest为`sha256:d1adf6835f1dd91205ba70e452c699d0aea601010038e5617f370716efb50569`，linux/amd64，压缩29724813字节，解压80646144字节。daemon直拉TLS失败；按task2策略宿主Registry下载同一digest，全部manifest/config/layer/diff_id校验后OCI导入成功。本机RootFS链与原配置一致。受限实际启动成功，x86_64、memory.max=2147483648、cpu.max=100000/100000。初步cap-drop apt探针因setuid/setgroups限制退出100，单列保留，不作为官方环境结论。
- 指定runtime.json缺historical_root，preflight退出1；本题忽略目录runtime.json使用现有generation-2和不变source/cases路径，保留官方1800秒agent/verifier超时。
- 当前原题preflight在`preflight/dna-assembly-valid-runtime/`运行，工具session=50010；容器`dna-assembly-oracle__env-main-1`。官方oracle已完成apt索引阶段，正下载EMBOSS等依赖；尚未原题评分，不宣称完成。没有改变原题环境、脚本、评分、模型或预算，没有模型调用。
- 精确日志与收据在忽略目录`evals/promptfoo/results/qwen38-suite/diagnostics/dna-assembly/`；当前本题资产未提交。28已提交`01a4c4e14`，可在本题验收后按序提交，无需等待不可运行的29。

## 最终阻塞结论（2026-10-27）

- 上述官方原题preflight最终退出1，原题oracle为`AgentTimeoutError: Agent execution timed out after 1800.0 seconds`。官方apt在61.0MB的emboss-data包反复下载/重试，超时仍未完成，也未生成/app/primers.fasta。
- 超时后官方verifier的apt受到仍运行下载进程的dpkg锁阻挡，curl及uvx未安装；原始verifier输出reward=0，但试验带AgentTimeoutError，suite正确归类unscored/infrastructure，绝不能当能力失败或有效评分。原题nop未运行，缺少完整父题oracle/nop校准，不能标记就绪。
- 已在授权范围内排查：daemon TLS失败通过同一digest OCI导入解决；执行配置缺字段通过本题generation-2配置解决；镜像、源码、容量、实际amd64资源启动均已核验；最终Clippy通过。剩余阻塞发生在官方oracle的agent阶段下载，verifier-only修复不能解决该阶段，且本任务无权改变原题环境/参考解或延长官方超时。
- 最终检查：25个单元测试通过、20个fresh fixture控制通过、8次fresh子题Harbor oracle/nop通过，score.sh test及Clippy退出0；Cargo实际槽位1与绝对路径仍见上文。原始失败和精确命令/退出码/哈希汇总在`diagnostics/dna-assembly/acceptance.json`及`preflight/dna-assembly-valid-runtime/preparation.json`。
- Harbor已结束本轮进程并清理本题容器；`docker ps -a --filter name=dna-assembly`为空。自有Cargo锁已释放，镜像和编译缓存保留；无模型请求，plan.md未修改，未触碰27/29或其它任务内容。
- 保留本任务文件及未提交诊断资产；不提交、不删除编号任务。恢复入口：官方Ubuntu下载链路恢复后，在本题runtime.json下重新执行同一preflight，输出选择新的未存在目录；不要重用这次失败目录，不增加模型/预算/超时。完整父题通过后再重新核验收据、按序提交本题资产并删除任务文件。

- 终态协调后仅执行一次20秒上限安全恢复审计：在同一原镜像、显式linux/amd64、1CPU/2048MiB、只读根和无新增依赖环境，对官方同一emboss-data HTTP路径发HEAD；退出0、HTTP200、Content-Length=61018452。只证明端点可达，不证明61MB持续传输或整套依赖能在原1800秒内完成。原始记录`diagnostics/dna-assembly/final-recovery-audit.json`。未再次运行1800秒校准、未改变原题环境/参考解/预算，审计容器亦已清理；本题容器查询仍为空。需要外部条件：原官方Ubuntu下载链路能在固定超时内完成官方依赖安装，之后才恢复本题校准。
