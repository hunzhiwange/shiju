# 任务 80: 准备-raman-fitting

批次：【批次 45】；依赖批次：1、2、3、4、5。同批无文件依赖；Git 提交按任务编号串行，禁止删除其他会话的索引锁。

状态：已阻塞

目的：为“谱峰参数拟合”准备原题完整环境与四道独立诊断题。

来源任务：Terminal-Bench2.1/raman-fitting

预计会话范围：只准备一个父题的四个窄诊断题和确定性校准；不运行付费模型。长镜像拉取允许记录进程后续接，不能把未校准标成完成。

## 文件

- 新建：`evals/terminal-bench/suite/cases/raman-fitting/case.json`、`evals/terminal-bench/suite/cases/raman-fitting/check.py`、`evals/terminal-bench/suite/cases/raman-fitting/tests/test_cases.py`、`evals/terminal-bench/suite/cases/raman-fitting/fixtures/`、`evals/terminal-bench/suite/cases/raman-fitting/oracle/`、`evals/terminal-bench/suite/cases/raman-fitting/environment/Dockerfile`、`evals/terminal-bench/suite/cases/raman-fitting/README.md`
- 只读完整上游包：`evals/promptfoo/results/qwen38-suite/source/tasks/raman-fitting/`
- 忽略产物：`evals/promptfoo/results/qwen38-suite/preflight/raman-fitting/`、`evals/promptfoo/results/qwen38-suite/diagnostics/raman-fitting/`

## 上下文

原题 raman-fitting；类别 scientific-computing；CPU=1、内存=2048MiB、存储=10240MiB、GPU=0；Agent=900秒、verifier=900秒。预建镜像 alexgshaw/raman-fitting:20251031。原题树SHA=13925eceab38977b960d6c061791166570fd5297；题面SHA256=1fa59a89d88daf6a4b2d422955f53779b461880b527f40d79c71b16e0ee52433；配置SHA256=0f3f6b580d7435954b3a114e77cdfb6cd0a856f15cc0ad83cd1b69c48492dfed。读完整 instruction.md、输入、solution、tests 后设计，禁止改原题。四子题：m1 读取合成光谱与单位；m2 拟合双峰小型数据；m3 处理背景噪声与参数边界；m4 核验参数文件和重建误差。

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

先使用技能 ./skills/create-agent-evals/SKILL.md；每个子题定义合成输入、清楚完整题面、输出路径/API/服务行为与独立确定性判定。新增 evals/terminal-bench/suite/cases/raman-fitting/tests/test_cases.py：正解通过，至少3个有意义错误变体被拒绝；运行真实程序/产物，不验证关键词、自述或仅Agent自测。错误fixture和漏检checker先失败，再最小修复。

验证命令：

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/raman-fitting/tests -p test_cases.py
```

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/raman-fitting/case.json --run-fixtures
```

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.run preflight --catalog evals/terminal-bench/suite/catalog.json --runtime evals/promptfoo/results/qwen38-suite/runtime.json --parent-id raman-fitting --output evals/promptfoo/results/qwen38-suite/preflight/raman-fitting
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

## 2026-10-28 宿主检查点与恢复入口

- 批次1–5依赖：catalog/source冻结存在；本题9个文件逐字节SHA256匹配catalog，tree=13925eceab38977b960d6c061791166570fd5297，题面SHA匹配任务。容量策略、contract-validation（12个contract测试与12个catalog测试）、runtime历史验证、agent-validation（8测试）收据存在。它们仅证明准备依赖，不能替代本题实时容量准入或当前runtime身份校验。当前执行身份由调度指定为ce05045daeb0b6f9a28b5cd179f2f4363cfbbaf143a099565e27218a494a4bd1。
- 已完成本题独立资产：m1单位/格式读取、m2实际双峰数值拟合、m3背景噪声/边界拟合、m4参数结构/有限数/边界与实际重建误差；各有合成输入、独立题面、gold/nop与3语义负例。未修改公共执行、冻结配置、上游材料或评分/模型/预算。
- 红灯：宽松checker下16拒绝控制失败，另计数断言失败，共17 failures；绿灯：指定unittest命令退出0，1测试方法/20实际子控制通过。具体命令为 `PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/raman-fitting/tests -p test_cases.py`。
- 静态验证：`PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/raman-fitting/case.json` 退出0，仅格式与文件哈希，无容器校准；`bash evals/promptfoo/score.sh test` 退出0，配置有效，无付费模型调用。
- Clippy：动态领取槽位1，CARGO_TARGET_DIR=/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1，CARGO_BUILD_BUILD_DIR同绝对路径，CARGO_BUILD_JOBS=1。`cargo clippy --all-targets --all-features -- -D warnings` 退出0，有效行为测试数量0；未修改Rust，无需Rust格式化。自身锁已释放，缓存保留。
- 证据目录：`evals/promptfoo/results/qwen38-suite/diagnostics/raman-fitting/`，包含red.log、green.log、host-manifest.json、source-check.json、score-test.log、clippy.log、clippy-receipt.txt、host-evidence.json。
- 明确阻塞：调度要求任务69独占容器/镜像窗口，未经明确释放不得Docker/镜像层下载导入/标准控制/父题运行。因此尚未运行真实Linux20fixture、4子题8次标准Harbor控制、完整父题oracle/nop、实时镜像digest/隔离/容量/资源校准；未生成validation.json或preparation.json，不宣称就绪。未运行付费模型。
- 恢复：原会话获得窗口释放后先按task2完整容量准入，再复核声明工具镜像leaf与干净环境、当前授权runtime身份/闭包；执行原验证计划的 `--run-fixtures`，使用正式SuiteDockerEnvironment经suite.scoring.harbor_trial跑四子题fresh oracle/nop及父题preflight（使用验证过的当前runtime执行配置，禁止私有platform/Trial/Compose覆盖）。父题Agent环境不得预修，必要verifier-only修复必须重新校准且不改官方checker。获得完整收据后再标记完成、按编号串行提交与删除任务文件。当前资产保留未提交，避免把未校准任务标为完成。
