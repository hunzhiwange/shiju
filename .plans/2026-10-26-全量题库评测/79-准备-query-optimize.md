# 任务 79: 准备-query-optimize

批次：【批次 45】；依赖批次：1、2、3、4、5。同批无文件依赖；Git 提交按任务编号串行，禁止删除其他会话的索引锁。

状态：已阻塞

目的：为“等价查询优化”准备原题完整环境与四道独立诊断题。

来源任务：Terminal-Bench2.1/query-optimize

预计会话范围：只准备一个父题的四个窄诊断题和确定性校准；不运行付费模型。长镜像拉取允许记录进程后续接，不能把未校准标成完成。

## 文件

- 新建：`evals/terminal-bench/suite/cases/query-optimize/case.json`、`evals/terminal-bench/suite/cases/query-optimize/check.py`、`evals/terminal-bench/suite/cases/query-optimize/tests/test_cases.py`、`evals/terminal-bench/suite/cases/query-optimize/fixtures/`、`evals/terminal-bench/suite/cases/query-optimize/oracle/`、`evals/terminal-bench/suite/cases/query-optimize/environment/Dockerfile`、`evals/terminal-bench/suite/cases/query-optimize/README.md`
- 只读完整上游包：`evals/promptfoo/results/qwen38-suite/source/tasks/query-optimize/`
- 忽略产物：`evals/promptfoo/results/qwen38-suite/preflight/query-optimize/`、`evals/promptfoo/results/qwen38-suite/diagnostics/query-optimize/`

## 上下文

原题 query-optimize；类别 data-science；CPU=1、内存=2048MiB、存储=10240MiB、GPU=0；Agent=900秒、verifier=1800秒。预建镜像 alexgshaw/query-optimize:20251031。原题树SHA=c04a867b98a03f27199ebd6c40d54a2f97d807ae；题面SHA256=473e12d320d2732f679fb631c2a4dab4a01deea6775686718ae0c417b8a39447；配置SHA256=6e88f2a86df699594bcb1bf81df4ed694505f79c9420f671335b02524bbe3868。读完整 instruction.md、输入、solution、tests 后设计，禁止改原题。四子题：m1 读取小型表和原查询；m2 写出等价查询；m3 检查空值重复与排序；m4 比较结果和查询计划。

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

先使用技能 ./skills/create-agent-evals/SKILL.md；每个子题定义合成输入、清楚完整题面、输出路径/API/服务行为与独立确定性判定。新增 evals/terminal-bench/suite/cases/query-optimize/tests/test_cases.py：正解通过，至少3个有意义错误变体被拒绝；运行真实程序/产物，不验证关键词、自述或仅Agent自测。错误fixture和漏检checker先失败，再最小修复。

验证命令：

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/query-optimize/tests -p test_cases.py
```

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/query-optimize/case.json --run-fixtures
```

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.run preflight --catalog evals/terminal-bench/suite/catalog.json --runtime evals/promptfoo/results/qwen38-suite/runtime.json --parent-id query-optimize --output evals/promptfoo/results/qwen38-suite/preflight/query-optimize
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


## 本次续接记录（2026-10-28）

- 依赖核对：来源 source.json/catalog 固定 7131e4375048a0e408a8fb404b5f499d726b695b；本题 instruction/task.toml SHA256 与任务所列一致。容量 capacity.json=complete；契约 contract-validation.json 有 12 测试成功；agent-validation.json 有 8 测试成功；调度/报告实现提交 078bc547f/fb2237255 存在。批次 1–4 的基础资产存在；批次 5 模型准入不能宣称已满足（readiness-recovered.json 的原始记录仍有 passed=false），不得因此重跑付费请求。诊断 ABI 尚待调度任务 69 修复。
- 调度明确限制：74 独占容器/镜像窗口，之后 69 先修诊断 ABI。本次禁止 Docker、镜像下载/导入、标准控制和父题运行；未执行这些操作，未修改公共执行、冻结运行时或 plan.md。
- 已保留本题四套独立合成资产：m1 分组计数/求和，m2 消除关联子查询，m3 NULL/重复/排序，m4 等价结果及实际索引计划；每题 gold/nop/三个 wrong。checker 实际运行宿主 SQLite，8 组独立数据逐列逐行比较并执行 EXPLAIN QUERY PLAN，未用预写输出替代数据库。
- TDD：初始漏检 checker 的 24 测试有 18 失败；实现后新增等价关联查询拒绝和替代索引写法接受回归。任务指定 unittest 命令退出 0，26 项有效测试、无跳过；仅宿主证据。完整 20 项 gold/nop/wrong 控制及真实查询计划见忽略目录 diagnostics/query-optimize/host-controls.json；red/green 日志分别 host-red.log/host-green.log。
- `PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/query-optimize/case.json` 退出 0，仅 schema/字节绑定检查，非 --run-fixtures。`evals/promptfoo/score.sh test` 退出 0，配置有效，无模型调用。
- `cargo clippy --all-targets --all-features -- -D warnings` 退出 0（无 Rust 行为测试）；动态槽位 1，CARGO_TARGET_DIR=/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1；同时设置 CARGO_BUILD_BUILD_DIR 为该绝对路径、CARGO_BUILD_JOBS=1。已释放自己的锁，保留缓存；详情 diagnostics/query-optimize/cargo-slot.txt 与 clippy.log。纯 Python 变更未运行 Rust fmt。
- 缺口：父题 oracle/nop 未执行；四子题 fresh 容器 20 控制未执行；镜像 digest/架构/真实隔离/资源/依赖收据及正式 preflight 均缺失。case.json 的全零 digest 故意不可运行，environment/Dockerfile 尚未构建；没有生成 validation.json，没有声称已就绪或完成。
- 恢复入口：调度明确释放窗口并修好 ABI 后，核对授权执行身份 ce05045daeb0b6f9a28b5cd179f2f4363cfbbaf143a099565e27218a494a4bd1，使用标准 SuiteDockerEnvironment（不覆盖 Trial/Compose/平台），容器内真实执行冻结 ELF --help；构建并记录已审查诊断镜像的真实 digest 后替换占位值。随后按本文件原命令执行 --run-fixtures 与 preflight，补齐父题官方 oracle=1/nop=0、四子题控制和收据。不得修改官方 checker、付费重试或扩大预算。
- 按最新调度指令保持已阻塞并保留资产与任务文件；未提交（待完整校准及按编号串行提交），未删除任务文件。
