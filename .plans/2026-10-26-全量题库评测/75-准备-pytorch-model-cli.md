# 任务 75: 准备-pytorch-model-cli

批次：【批次 43】；依赖批次：1、2、3、4、5。同批无文件依赖；Git 提交按任务编号串行，禁止删除其他会话的索引锁。

状态：已阻塞

目的：为“模型命令行推理”准备原题完整环境与四道独立诊断题。

来源任务：Terminal-Bench2.1/pytorch-model-cli

预计会话范围：只准备一个父题的四个窄诊断题和确定性校准；不运行付费模型。长镜像拉取允许记录进程后续接，不能把未校准标成完成。

## 文件

- 新建：`evals/terminal-bench/suite/cases/pytorch-model-cli/case.json`、`evals/terminal-bench/suite/cases/pytorch-model-cli/check.py`、`evals/terminal-bench/suite/cases/pytorch-model-cli/tests/test_cases.py`、`evals/terminal-bench/suite/cases/pytorch-model-cli/fixtures/`、`evals/terminal-bench/suite/cases/pytorch-model-cli/oracle/`、`evals/terminal-bench/suite/cases/pytorch-model-cli/environment/Dockerfile`、`evals/terminal-bench/suite/cases/pytorch-model-cli/README.md`
- 只读完整上游包：`evals/promptfoo/results/qwen38-suite/source/tasks/pytorch-model-cli/`
- 忽略产物：`evals/promptfoo/results/qwen38-suite/preflight/pytorch-model-cli/`、`evals/promptfoo/results/qwen38-suite/diagnostics/pytorch-model-cli/`

## 上下文

原题 pytorch-model-cli；类别 model-training；CPU=1、内存=2048MiB、存储=10240MiB、GPU=0；Agent=900秒、verifier=900秒。预建镜像 alexgshaw/pytorch-model-cli:20251031。原题树SHA=4cd9f2c9247794e1e260c8752bd24aa6a7a77fcd；题面SHA256=ce36ee1f6c589f81ee6a6be4967e25c76a2ed697006928023f42dfb9f28a6750；配置SHA256=fc1c018efbd5d0537ac0741d53d1ebad3886faa51d14cbde8ec953856411bc0d。读完整 instruction.md、输入、solution、tests 后设计，禁止改原题。四子题：m1 读取权重与合成图像；m2 实现小型推理链；m3 校验预处理与参数错误；m4 核对唯一标准输出。

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

先使用技能 ./skills/create-agent-evals/SKILL.md；每个子题定义合成输入、清楚完整题面、输出路径/API/服务行为与独立确定性判定。新增 evals/terminal-bench/suite/cases/pytorch-model-cli/tests/test_cases.py：正解通过，至少3个有意义错误变体被拒绝；运行真实程序/产物，不验证关键词、自述或仅Agent自测。错误fixture和漏检checker先失败，再最小修复。

验证命令：

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/pytorch-model-cli/tests -p test_cases.py
```

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/pytorch-model-cli/case.json --run-fixtures
```

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.run preflight --catalog evals/terminal-bench/suite/catalog.json --runtime evals/promptfoo/results/qwen38-suite/runtime.json --parent-id pytorch-model-cli --output evals/promptfoo/results/qwen38-suite/preflight/pytorch-model-cli
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


## 2026-10-28 宿主实施与阻塞记录

- 已应用 ./skills/do-task-plans/SKILL.md、./skills/create-agent-evals/SKILL.md，读取 plan.md、固定完整题面、配置、模型定义、参考解、测试、Dockerfile与README；输入PNG/PTH和库逐文件读取并记录SHA256。官方题面/配置SHA256与本任务一致，未改上游与plan.md。
- 依赖批次1–5所需冻结source/catalog、capacity.json、contract-validation.json、suite/runtime-v2.json和VALIDATION.md、agent-validation.json、diagnostics-format/evidence.json、readiness-validation.json均存在。它们仅为前置资产，不代替本题校准；共享接线正在由任务13更新，待释放后再次核验当前执行身份。
- 已准备四道独立合成诊断、公开输入、隐藏checker、gold/nop和每题3个错误实现。m1读取权重与PNG，m2小型MLP，m3预处理和参数错误，m4唯一stdout与prediction.txt。未扩大模型、评分或预算，未运行付费模型。
- TDD初始缺checker：规定unittest命令21测试、20 failures/1 error，退出1。独立scratch副本全接受checker缺陷对照：21测试、16 failures，退出1；正式checker不受实验修改。最终规定命令 `PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/pytorch-model-cli/tests -p test_cases.py`：21测试通过，退出0（20行为候选+1格式检查）。这些是宿主控制，不是Linux隔离校准。
- 静态 `PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/pytorch-model-cli/case.json` 退出0；未加 --run-fixtures。`evals/promptfoo/score.sh test` 退出0。
- 动态原子领取Cargo槽1，CARGO_TARGET_DIR和CARGO_BUILD_BUILD_DIR均为 `/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1`，CARGO_BUILD_JOBS=1。`cargo clippy --all-targets --all-features -- -D warnings` 退出0，0行为测试；仅既有block 0.1.6未来兼容提示。正常/信号退出清理已配置，阶段结束已释放自身锁并保留缓存。无Rust修改，未执行fmt、全量cargo test、wasm或Makefile命令。
- 原始证据：忽略目录 `evals/promptfoo/results/qwen38-suite/diagnostics/pytorch-model-cli/` 内host-red.log、checker-mutation-red.log、host-green.log、static.log、score.log、clippy.log、cargo-slot.txt、source-inspection.json、host-evidence.json。
- 具体阻塞：调度者明确告知原任务13独占容器窗口，禁止Docker、镜像层下载/导入、标准校准或父题运行，等待明确释放。本会话未执行上述动作。父题oracle/nop、四子题8标准控制、20 Docker fixtures、真实Linux工具和冻结ELF --help、镜像实际身份/内容核验、资源隔离和磁盘收据未取得。任务未就绪，不提交未校准资产、不删除本文件。
- 恢复入口：调度者明确释放窗口后，先核验公共接线与SuiteDockerEnvironment的当前身份，复核资源/磁盘/网络，再核验case.json所列32c8...共享工具基底无其他题材料。执行本任务原定 --run-fixtures 与 suite.run preflight，全部经标准生产接线，补齐新鲜oracle/nop、负例、digest及哈希收据。不得使用私有Trial/Compose/镜像覆盖。满足全部完成证据后按任务序号使用git-commit提交自身资产，再删除本文件。当前保持已阻塞并保留未提交材料。
