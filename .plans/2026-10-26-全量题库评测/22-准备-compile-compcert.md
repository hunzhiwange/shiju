# 任务 22: 准备-compile-compcert

批次：【批次 12】；依赖批次：1、2、3、4、5。同批无文件依赖；Git 提交按任务编号串行，禁止删除其他会话的索引锁。

状态：已阻塞

目的：为“验证编译器构建”准备原题完整环境与四道独立诊断题。

来源任务：Terminal-Bench2.1/compile-compcert

预计会话范围：只准备一个父题的四个窄诊断题和确定性校准；不运行付费模型。长镜像拉取允许记录进程后续接，不能把未校准标成完成。

## 文件

- 新建：`evals/terminal-bench/suite/cases/compile-compcert/case.json`、`evals/terminal-bench/suite/cases/compile-compcert/check.py`、`evals/terminal-bench/suite/cases/compile-compcert/tests/test_cases.py`、`evals/terminal-bench/suite/cases/compile-compcert/fixtures/`、`evals/terminal-bench/suite/cases/compile-compcert/oracle/`、`evals/terminal-bench/suite/cases/compile-compcert/environment/Dockerfile`、`evals/terminal-bench/suite/cases/compile-compcert/README.md`
- 只读完整上游包：`evals/promptfoo/results/qwen38-suite/source/tasks/compile-compcert/`
- 忽略产物：`evals/promptfoo/results/qwen38-suite/preflight/compile-compcert/`、`evals/promptfoo/results/qwen38-suite/diagnostics/compile-compcert/`

## 上下文

原题 compile-compcert；类别 system-administration；CPU=2、内存=4096MiB、存储=10240MiB、GPU=0；Agent=2400秒、verifier=2400秒。预建镜像 alexgshaw/compile-compcert:20260403。原题树SHA=c91f1208acb9c5b29afb99098a8fcae8de59c3f2；题面SHA256=d6efce07c47484fcd13c7918f68bd4af6314392de43751c4cad1e62530c2ad06；配置SHA256=1d9618d7178cfa15918a293da126b867f3a7db160e604d4fec369949884dac39。读完整 instruction.md、输入、solution、tests 后设计，禁止改原题。四子题：m1 核对配置与目标架构；m2 构建最小可用编译入口；m3 编译含边界行为的程序；m4 核验执行结果与安装路径。

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

先使用技能 ./skills/create-agent-evals/SKILL.md；每个子题定义合成输入、清楚完整题面、输出路径/API/服务行为与独立确定性判定。新增 evals/terminal-bench/suite/cases/compile-compcert/tests/test_cases.py：正解通过，至少3个有意义错误变体被拒绝；运行真实程序/产物，不验证关键词、自述或仅Agent自测。错误fixture和漏检checker先失败，再最小修复。

验证命令：

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/compile-compcert/tests -p test_cases.py
```

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/compile-compcert/case.json --run-fixtures
```

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.run preflight --catalog evals/terminal-bench/suite/catalog.json --runtime evals/promptfoo/results/qwen38-suite/runtime.json --parent-id compile-compcert --output evals/promptfoo/results/qwen38-suite/preflight/compile-compcert
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


## 2026-10-27 本次检查与阻塞证据

- 批次 1–5 依赖已核对：89 题冻结 catalog、容量策略、实验契约、generation 2 冻结运行时（ELF SHA256 `847823c5cd99a6828cddfc2e3e5dbb67c5ddcc9f8693a13d539d8a8fa47c9dae`）、agent 与诊断格式证据齐全。本题全部 10 个上游文件逐一匹配 catalog；未修改上游、plan.md、模型、预算、评分及超时。
- 原镜像 digest：`sha256:f60236a4fe55bbb3c573210fe0fcd91551f4ee97c417453cd4985be074b35411`，linux/amd64；受限容器实测 memory.max=4294967296、cpu.max=200000 100000。压缩层共 29732322 字节、解压 tar 共 80657920 字节，配置的 diff_ids 全部匹配。保守磁盘估算 19467475396 字节，宿主/VM 空闲均足够；未改变全局 Docker 配置或清理其他资源。
- 执行 `PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python evals/promptfoo/results/qwen38-suite/diagnostics/compile-compcert/calibrate-parent.py`，退出 1。内部使用原始任务路径和 `suite.scoring.harbor_trial`，oracle、官方资源和超时均未覆盖。Agent 于 UTC 09:31:34 开始，10:11:34 达到 2400 秒并产生 `AgentTimeoutError`；当时尚在 CompCert extraction，未安装 ccomp。
- 超时取消宿主 exec 后，容器内 make 子进程仍继续运行，在 verifier 阶段完成安装，造成官方 pytest 3/3 通过、原始 reward=1。这是**超时后的迟到产物**，不构成正常完成的 oracle；suite 汇总明确为 `score_status=unscored`、`reward=null`、`failure_class=infrastructure`，未计为有效成绩。不能仅凭 reward=1 标为已就绪。
- Coq 旧仓库容器内探测 HTTP 404（初始化实际 curl 失败），但原解对此允许失败；默认 opam 仓库成功安装 OCaml 4.14.2、Coq 8.16.1、menhir 20230608 等。GitHub/opam/uv/PyPI 后续容器内均 HTTP 200；不能将初始缺 CA 或可忽略的旧仓库错误误当最终原因。主要阻塞是原样参考解在当前 amd64 仿真、依赖新建环境下超过官方 agent 时限。
- `evals/promptfoo/score.sh test` 退出 0，配置有效（非行为测试）；`cargo clippy --all-targets --all-features -- -D warnings` 退出 0，Rust 行为测试数量 0。动态领取槽位 1，`CARGO_TARGET_DIR=/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1`，`CARGO_BUILD_BUILD_DIR` 同值，`CARGO_BUILD_JOBS=1`。所属占用记录及锁已释放，缓存保留。
- 原题 nop 与四子题 gold/nop/负例、题级 unittest、`suite.diagnostics validate --run-fixtures`、完整 `suite.run preflight` 均未执行：原题 oracle 正常完成门槛未满足，且没有发布 case.json。诊断 generator/checker 仅为忽略目录内未验收草稿；超时后导出的公开工具链也仅作排查材料，不能充当有效 oracle 或准入收据。有效完整题评分 0、有效诊断评分 0、模型调用 0。
- 本次唯一受限校准容器已归档资源/镜像身份并删除；再次按 compose project 查询返回空，后台校准及导出监测进程均已结束。未删除其他会话的锁、容器或资产。
- 原始证据：`evals/promptfoo/results/qwen38-suite/diagnostics/compile-compcert/`，重点为 `parent-calibration.json`、`parent-calibration/compile-compcert-oracle-parent/result.json`、`exception.txt`、`verifier/test-stdout.txt`、`scoring/first.json`、`verifier-start-processes.json`、`source-verification.json`、`resource-image-evidence.json`、`image-capacity.json`、`clippy.json` 和 `blocked-evidence.json`。
- 恢复入口：先协调原样参考解在固定 2400 秒内正常结束的环境准入问题；没有授权时不得增加超时、预装 agent 依赖、改原题环境或将迟到 reward 当通过。若需要兼容变体，须另获明确授权并单列，不能替代本原题。恢复后先取得 fresh 原题 oracle 正常通过和 nop 失败，再完成四子题 TDD/校准及完整 preflight。当前仅编号任务文件状态与证据说明变更，无提交、保留本文件。


## 2026-10-27 有界安全恢复审计（仍已阻塞）

- 未重复 2400 秒 oracle，未调用模型/Cargo，未预装工具链、复用迟到产物或变更题目。四诊断仍只有未实现/未验收草稿；迟到官方 3/3、reward=1 继续作为无效校准保留。
- 时间线复核：apt 索引 33.6MB/89 秒、依赖包 99.9MB/288 秒，两项共 377 秒；另有大量 OCaml/Coq 源码构建。按进程 elapsed 推导 CompCert make 约 UTC 10:05:50 才开始，2400 秒硬截止 UTC 10:11:34.916，迟到 verifier 10:12:50.484 结束。不是仅一次未处理网络失败导致中止。旧 Coq 仓库失败被官方脚本容忍，所需包最终成功安装。
- 同一镜像 digest、linux/amd64、2CPU/4GiB 临时原镜像容器只读下载同源原 apt 依赖约 10.7MB 包，单次硬上限 35 秒，无安装。首次探测额外 cap-drop 阻止 apt 降权，已排除此结果；修正为默认 capability 后，默认路径 35 秒未完成。显式使用既有代理 http://http.docker.internal:3128 的单请求 22.4765 秒完成 10714542 字节，SHA256 `2ca48bf0c2d6465bc39322899715a85d934b4d7442dd5586a7bebbe3ce0f806b`。
- 代理仅证明单包可达与候选路径，不能外推全部端点持续吞吐或保证 2400 秒总构建时限。原 oracle 环境仅 PATH，未注入代理；本次没有修改共享 runner、题目环境、镜像或全局网络配置，没有据此重试整轮。当前证据不足以宣称现有条件已恢复准入。
- 最小外部条件/协调决策：优先提供并授权现有原生 amd64 Docker 执行环境（同镜像、2CPU/4GiB、2400 秒、fresh 无预装）；或先协调既有代理的合法注入入口与稳定性证据，再决定是否进行 fresh 原样校准。不得通过预热工具链、迟到产物、加超时、改参考解或单列兼容变体冒充原样就绪。
- 三次临时只读探测容器均记录精确 ID、资源限制与删除退出码 0。独立复查原 compose label 和各探测精确名字均为空；原 calibration/export/clippy/audit Python 后台进程为空；本任务 Cargo 锁为空。未删除他人锁或其他容器，业务容器保持运行。本次未运行 Cargo，无新增槽位。
- 新证据在忽略目录：`recovery-audit-summary.json`、`recovery-network-audit-initial.json`、`recovery-network-default.json`、`recovery-network-audit.json`、`recovery-host-cleanup.json`。本轮仅追加本编号任务审计说明，无提交；不阻挡其他无依赖题继续准备。
