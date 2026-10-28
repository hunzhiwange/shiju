# 任务 76: 准备-pytorch-model-recovery

批次：【批次 43】；依赖批次：1、2、3、4、5。同批无文件依赖；Git 提交按任务编号串行，禁止删除其他会话的索引锁。

状态：已阻塞

目的：为“模型结构恢复”准备原题完整环境与四道独立诊断题。

来源任务：Terminal-Bench2.1/pytorch-model-recovery

预计会话范围：只准备一个父题的四个窄诊断题和确定性校准；不运行付费模型。长镜像拉取允许记录进程后续接，不能把未校准标成完成。

## 文件

- 新建：`evals/terminal-bench/suite/cases/pytorch-model-recovery/case.json`、`evals/terminal-bench/suite/cases/pytorch-model-recovery/check.py`、`evals/terminal-bench/suite/cases/pytorch-model-recovery/tests/test_cases.py`、`evals/terminal-bench/suite/cases/pytorch-model-recovery/fixtures/`、`evals/terminal-bench/suite/cases/pytorch-model-recovery/oracle/`、`evals/terminal-bench/suite/cases/pytorch-model-recovery/environment/Dockerfile`、`evals/terminal-bench/suite/cases/pytorch-model-recovery/README.md`
- 只读完整上游包：`evals/promptfoo/results/qwen38-suite/source/tasks/pytorch-model-recovery/`
- 忽略产物：`evals/promptfoo/results/qwen38-suite/preflight/pytorch-model-recovery/`、`evals/promptfoo/results/qwen38-suite/diagnostics/pytorch-model-recovery/`

## 上下文

原题 pytorch-model-recovery；类别 model-training；CPU=1、内存=2048MiB、存储=10240MiB、GPU=0；Agent=900秒、verifier=900秒。预建镜像 alexgshaw/pytorch-model-recovery:20260430。原题树SHA=e9cacc61e4ec539777458ae72bb1c1ad3450a0b8；题面SHA256=2bef07dc28be3ab2c35ae389584034c2e4dde92c2c31b3eb6bd829628c8c3f4a；配置SHA256=851a51204f53482a31f47cdf8833128cf421113b0331011286940fc857c37dfd。读完整 instruction.md、输入、solution、tests 后设计，禁止改原题。四子题：m1 解析合成状态字典；m2 重建匹配网络结构；m3 仅训练指定输出层；m4 验证序列化与损失下降。

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

先使用技能 ./skills/create-agent-evals/SKILL.md；每个子题定义合成输入、清楚完整题面、输出路径/API/服务行为与独立确定性判定。新增 evals/terminal-bench/suite/cases/pytorch-model-recovery/tests/test_cases.py：正解通过，至少3个有意义错误变体被拒绝；运行真实程序/产物，不验证关键词、自述或仅Agent自测。错误fixture和漏检checker先失败，再最小修复。

验证命令：

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/pytorch-model-recovery/tests -p test_cases.py
```

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/pytorch-model-recovery/case.json --run-fixtures
```

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.run preflight --catalog evals/terminal-bench/suite/catalog.json --runtime evals/promptfoo/results/qwen38-suite/runtime.json --parent-id pytorch-model-recovery --output evals/promptfoo/results/qwen38-suite/preflight/pytorch-model-recovery
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

- 已使用 `./skills/do-task-plans/SKILL.md`、`./skills/create-agent-evals/SKILL.md`；阅读只读 plan.md、完整官方题面、配置、参考解、评分脚本、Dockerfile、README，并以真实 PyTorch 读取 weights.pt、dataset.pt、weights_gtruth.pt。所有原始文件均记录 SHA256，题面/配置/原题树身份与任务要求一致；未修改上游或 plan.md。
- 依赖批次1–5：冻结 source/catalog、capacity.json、contract-validation.json、suite/runtime-v2.json 与 VALIDATION.md、agent-validation.json、diagnostics-format/evidence.json、readiness-validation.json 均存在且哈希已归档。重新从冻结 Git 构建89题16类别 catalog，退出0，内容与当前 catalog 完全一致。已有依赖收据仅是前置证据，不代替本题校准；公共容器接线正由任务13修复。
- 本题目录已准备 m1..m4 独立合成输入、完整 CLI 契约、确定性 checker、gold 和每题3个错误实现。m1 真实 tensor state_dict 解析；m2 Linear/ReLU/Linear 严格结构及 TorchScript；m3 仅输出层训练、逐张量冻结及输入完整性；m4 真实序列化、动态批量推理和独立 MSE 报告校验。checker 对每题另生成两套不同维度输入，不以关键词、自述或候选自测评分。未改变模型、评分、预算或付费运行范围。
- 在本题忽略目录独立 host-venv 安装真实 torch==2.7.1；没有修改共享 Harbor 环境。TDD 首次缺失 checker：21测试、20 failures/1 error，退出1。最小 checker 实现后，`PYTHONPATH=evals/terminal-bench evals/promptfoo/results/qwen38-suite/diagnostics/pytorch-model-recovery/host-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/pytorch-model-recovery/tests -p test_cases.py`：21测试全部通过，退出0，含20真实行为控制。独立 scratch 副本换为全接受 checker：21测试、16 failures，退出1，正式 checker 未被修改。
- 任务指定 Harbor Python unittest 命令退出0：21测试中仅1格式检查有效、20行为控制因缺 torch 明确跳过；不将其记为行为验证成功。真实宿主行为证据来自上一条独立 torch 环境。静态 `PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/pytorch-model-recovery/case.json` 退出0，未加 --run-fixtures，未生成 validation.json。`evals/promptfoo/score.sh test` 退出0。
- 动态原子领取 Cargo 槽位1；CARGO_TARGET_DIR 与 CARGO_BUILD_BUILD_DIR 均为 `/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1`，CARGO_BUILD_JOBS=1。`cargo clippy --all-targets --all-features -- -D warnings` 退出0、0行为测试，仅既有 block 0.1.6 未来兼容提示。进程结束已删除自己 owner 记录并释放锁，缓存保留，信号路径也会终止并等待自己的 Cargo 进程组再释放锁。无Rust修改，未运行fmt、全量cargo test、wasm或Makefile命令。
- 原始证据位于忽略目录 `evals/promptfoo/results/qwen38-suite/diagnostics/pytorch-model-recovery/`：host-red.log、host-green.log、harbor-host.log、checker-mutation-red.log、static.log、score.log、clippy.log、cargo-slot.json、source-inspection.json、catalog-rebuilt.json、host-evidence.json。后者绑定依赖/fixture/checker 哈希及测试数量。最初安装阶段出现缺 NumPy 的 torch 提示，不影响纯 tensor 控制；未以模拟推理替代 PyTorch。
- 具体阻塞：调度者明确限制任务13独占容器窗口，禁止Docker、镜像层下载/导入、标准校准和父题运行直至明确释放。本会话未启动/修改容器或下载镜像层；早期只读取一次 Docker info（4CPU、12530401280字节），不构成容量或环境准入。case.json 与 Dockerfile 暂引用32c8...共享工具基底，其真实 torch 能力、Linux工具和冻结ELF尚未核验，README 明确为未准入；不能把语法有效的digest引用当作可运行身份。
- 待做：窗口明确释放后，重新核验当前公共接线和生产身份，记录资源/磁盘/网络/隔离收据；通过授权标准 builder 准备并审查真实 torch 工具基底，更新为实测 digest（不得在 checker 静默装依赖或使用私有Trial/Compose覆盖）；核验 Linux工具及冻结ELF --help；执行原定 `--run-fixtures` 的20个真实Docker控制、8个标准子题oracle/nop、原定 `suite.run preflight` 父题oracle/nop。全部通过后按任务编号使用 git-commit 提交自身资产并删除本文件。目前保留任务及未提交资产，不宣称就绪。
