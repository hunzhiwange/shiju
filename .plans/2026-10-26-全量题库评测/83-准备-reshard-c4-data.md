# 任务 83: 准备-reshard-c4-data

批次：【批次 47】；依赖批次：1、2、3、4、5。同批无文件依赖；Git 提交按任务编号串行，禁止删除其他会话的索引锁。

状态：已阻塞

目的：为“数据重新分片”准备原题完整环境与四道独立诊断题。

来源任务：Terminal-Bench2.1/reshard-c4-data

预计会话范围：只准备一个父题的四个窄诊断题和确定性校准；不运行付费模型。长镜像拉取允许记录进程后续接，不能把未校准标成完成。

## 文件

- 新建：`evals/terminal-bench/suite/cases/reshard-c4-data/case.json`、`evals/terminal-bench/suite/cases/reshard-c4-data/check.py`、`evals/terminal-bench/suite/cases/reshard-c4-data/tests/test_cases.py`、`evals/terminal-bench/suite/cases/reshard-c4-data/fixtures/`、`evals/terminal-bench/suite/cases/reshard-c4-data/oracle/`、`evals/terminal-bench/suite/cases/reshard-c4-data/environment/Dockerfile`、`evals/terminal-bench/suite/cases/reshard-c4-data/README.md`
- 只读完整上游包：`evals/promptfoo/results/qwen38-suite/source/tasks/reshard-c4-data/`
- 忽略产物：`evals/promptfoo/results/qwen38-suite/preflight/reshard-c4-data/`、`evals/promptfoo/results/qwen38-suite/diagnostics/reshard-c4-data/`

## 上下文

原题 reshard-c4-data；类别 data-science；CPU=1、内存=2048MiB、存储=10240MiB、GPU=0；Agent=3600秒、verifier=3600秒。预建镜像 alexgshaw/reshard-c4-data:20251031。原题树SHA=fdc7d4f03963f227f5aa37d215bd8c2d3105a6cd；题面SHA256=1810abbf666e51e410950b4f495d3dae88d392ad056afa392e4c687ff31e3992；配置SHA256=c2f146a1e00f798f7486ff60686ad52749bb73c7b71e984d52716d859e0f7dd8。读完整 instruction.md、输入、solution、tests 后设计，禁止改原题。四子题：m1 读取合成文件与记录；m2 实现小型重分片压缩；m3 保留顺序并限制文件大小；m4 解压核验数据完整性。

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

先使用技能 ./skills/create-agent-evals/SKILL.md；每个子题定义合成输入、清楚完整题面、输出路径/API/服务行为与独立确定性判定。新增 evals/terminal-bench/suite/cases/reshard-c4-data/tests/test_cases.py：正解通过，至少3个有意义错误变体被拒绝；运行真实程序/产物，不验证关键词、自述或仅Agent自测。错误fixture和漏检checker先失败，再最小修复。

验证命令：

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/reshard-c4-data/tests -p test_cases.py
```

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/reshard-c4-data/case.json --run-fixtures
```

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.run preflight --catalog evals/terminal-bench/suite/catalog.json --runtime evals/promptfoo/results/qwen38-suite/runtime.json --parent-id reshard-c4-data --output evals/promptfoo/results/qwen38-suite/preflight/reshard-c4-data
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

## 2026-10-28 宿主阶段证据与恢复入口

依赖批次1–5的资产已具备：suite/source.json 固定89题来源；capacity.json记录4CPU/12530401280字节；contract-validation.json有12项契约通过；runtime.json与runtime/身份资料存在；agent-validation.json有8项代理绑定通过。执行身份按调度固定为ce05045daeb0b6f9a28b5cd179f2f4363cfbbaf143a099565e27218a494a4bd1，不修改公共执行或冻结资料。

已完整读取题面、task.toml、README、environment脚本、solve.sh和test_outputs.py/test.sh；files_hashes.json全部解析验证，原包哈希存于忽略证据目录。仅新增本题四个合成诊断资产。宿主 unittest命令同上，退出0，1项测试执行20控制（4gold、4nop、12wrong），每个gold执行3个真实小数据探针。故意使checker仅接受成功执行而跳过产物核验时负例失败，恢复后通过。

Clippy确切命令：cargo clippy --all-targets --all-features -- -D warnings；退出0，不计行为测试。实际动态槽位1，CARGO_TARGET_DIR=/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1；CARGO_BUILD_BUILD_DIR同路径、CARGO_BUILD_JOBS=1。自己的锁已释放，缓存保留。无Rust修改，无需fmt。

阻塞原因：调度明确任务71独占容器/镜像窗口，禁止本题Docker/下载导入/标准控制/父题运行，等待明确释放。Linux20fixture、8标准oracle/nop与父题oracle/nop均未执行，不宣称就绪；未创建validation.json。证据在evals/promptfoo/results/qwen38-suite/diagnostics/reshard-c4-data/，含host-green.log、checker-mutation-red.log、host-evidence.json、upstream-hashes.json、clippy.log、clippy-slot.txt。

窗口释放后原会话续接：重跑宿主测试；执行上述validate --run-fixtures；通过正式SuiteDockerEnvironment执行四题8个标准控制；运行上述父题preflight，记录digest/依赖/隔离/资源及oracle/nop收据；按任务编号串行使用git-commit提交本题资产。当前未提交，保留任务文件和资产，不删除其他会话文件，不修改plan.md。
