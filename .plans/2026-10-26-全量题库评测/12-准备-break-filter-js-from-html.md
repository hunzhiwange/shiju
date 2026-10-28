# 任务 12: 准备-break-filter-js-from-html

批次：【批次 7】；依赖批次：1、2、3、4、5。同批无文件依赖；Git 提交按任务编号串行，禁止删除其他会话的索引锁。

状态：已阻塞

目的：为“过滤器绕过诊断”准备原题完整环境与四道独立诊断题。

来源任务：Terminal-Bench2.1/break-filter-js-from-html

预计会话范围：只准备一个父题的四个窄诊断题和确定性校准；不运行付费模型。长镜像拉取允许记录进程后续接，不能把未校准标成完成。

## 文件

- 新建：`evals/terminal-bench/suite/cases/break-filter-js-from-html/case.json`、`evals/terminal-bench/suite/cases/break-filter-js-from-html/check.py`、`evals/terminal-bench/suite/cases/break-filter-js-from-html/tests/test_cases.py`、`evals/terminal-bench/suite/cases/break-filter-js-from-html/fixtures/`、`evals/terminal-bench/suite/cases/break-filter-js-from-html/oracle/`、`evals/terminal-bench/suite/cases/break-filter-js-from-html/environment/Dockerfile`、`evals/terminal-bench/suite/cases/break-filter-js-from-html/README.md`
- 只读完整上游包：`evals/promptfoo/results/qwen38-suite/source/tasks/break-filter-js-from-html/`
- 忽略产物：`evals/promptfoo/results/qwen38-suite/preflight/break-filter-js-from-html/`、`evals/promptfoo/results/qwen38-suite/diagnostics/break-filter-js-from-html/`

## 上下文

原题 break-filter-js-from-html；类别 security；CPU=1、内存=2048MiB、存储=10240MiB、GPU=0；Agent=1200秒、verifier=1200秒。预建镜像 alexgshaw/break-filter-js-from-html:20251031。原题树SHA=d18a5a82b3e92ae2b723d76a67ef25a757dbe3f7；题面SHA256=ad42558111b2a8e8534bdf52b584b804348e1a8d68d756f812a31d5147e65be5；配置SHA256=f820eb1b47e4ccc548b87d9a418fef519137dcf975ad73d0fb5246df00a9669b。读完整 instruction.md、输入、solution、tests 后设计，禁止改原题。四子题：m1 解析过滤器允许的输入；m2 构造本地最小绕过样例；m3 验证过滤后脚本语义；m4 输出可复现的沙箱测试。

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

先使用技能 ./skills/create-agent-evals/SKILL.md；每个子题定义合成输入、清楚完整题面、输出路径/API/服务行为与独立确定性判定。新增 evals/terminal-bench/suite/cases/break-filter-js-from-html/tests/test_cases.py：正解通过，至少3个有意义错误变体被拒绝；运行真实程序/产物，不验证关键词、自述或仅Agent自测。错误fixture和漏检checker先失败，再最小修复。

验证命令：

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/break-filter-js-from-html/tests -p test_cases.py
```

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/break-filter-js-from-html/case.json --run-fixtures
```

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.run preflight --catalog evals/terminal-bench/suite/catalog.json --runtime evals/promptfoo/results/qwen38-suite/runtime.json --parent-id break-filter-js-from-html --output evals/promptfoo/results/qwen38-suite/preflight/break-filter-js-from-html
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


## 2026-10-27 安全收尾记录

- 阻塞：协调会话通知“平台安全审查连续拒绝当前任务”，并明确要求停止实现与载荷运行；未提供底层平台原始错误码或完整错误文本，不能据此编造。恢复前需用户处理平台 Daybreak 访问条件并明确允许恢复。
- 已读取计划、任务、执行/评测技能、上游完整题面、过滤器、参考解和官方评分材料；未修改 plan.md 或原题。
- 必需 Clippy：`cargo clippy --all-targets --all-features -- -D warnings` 退出 0；有效 Rust 行为测试数量 0（此命令为静态检查）。动态领取槽位 1，`CARGO_TARGET_DIR=/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1`，`CARGO_BUILD_BUILD_DIR` 同路径，`CARGO_BUILD_JOBS=1`；已释放自己的锁，保留缓存。仅既有 block 0.1.6 future compatibility 提示。
- 指定 preflight 命令退出 1：任务指定 runtime.json 是历史身份收据，缺少 historical_root，报 KeyError；日志保留。拟用第二代冻结运行时的本题专属配置已留草稿，但未启动校准。
- 诊断草稿未校准：未运行本题 unittest、四子题 fixture 校准或父题 oracle/nop，不宣称完成，不提交草稿，不删除任务文件。
- 收尾停止明确属于本会话的原题拉取 PID 26009 和资源观察 PID 70734；诊断镜像构建进程已结束，无本题运行容器，无自有 Cargo 锁。保留下载缓存、镜像和全部日志，不清理他人资源。
- 日志与恢复入口：`evals/promptfoo/results/qwen38-suite/diagnostics/break-filter-js-from-html/`；恢复需先检查进程、缓存与草稿状态，继续遵守原预算和评分契约，不因平台拒绝改写提示或绕过限制。
- 补充已结束检查：`evals/promptfoo/score.sh test` 退出 0（配置有效）；诊断基础镜像构建退出 0，只证明依赖镜像构建成功，不构成任何行为验收。
