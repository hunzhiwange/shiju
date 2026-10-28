# 任务 84: 准备-rstan-to-pystan

批次：【批次 48】；依赖批次：1、2、3、4、5。同批无文件依赖；Git 提交按任务编号串行，禁止删除其他会话的索引锁。

状态：已阻塞

## 本次执行证据（2026-10-28）

- 阻塞为诊断真实工具资源限制，不是父题磁盘不足：官方 PyStan 3.10.0 / httpstan 4.13.0 / Python 3.12.12 已离线安装成功；`stan.build('parameters {real x;} model {x ~ normal(0,1);}', random_seed=1)` 的真实 gcc 编译在 1CPU / 512MiB / 无 swap 下失败，`cc1plus` 被杀，Docker `State.OOMKilled=true`，退出码 1。没有生成样本、没有模拟采样器，也没有扩大子题预算。尚未制作可宣称完成的四子题资产或生成 readiness/preparation 收据。
- 缓存工具镜像 Python 3.13 的首次路径也已真实排查：pysimdjson<7 无 cp313 wheel，官方 6.0.2 源码在 CPython 3.13 的 `_PyLong_AsByteArray` / `_PyGen_SetStopIterationValue` API 上编译失败。随后改用官方固定 Python 3.12 和 23 个真实 wheel；宿主完整 SHA、压缩/解压体积和安装配方均保留。没有修改原父 Agent 环境。
- 依赖批次 1–5：catalog、contract、runtime、agent 共 48 项聚焦测试通过；上游全部 13 文件与冻结 catalog SHA256 匹配。执行身份 `ce05045daeb0b6f9a28b5cd179f2f4363cfbbaf143a099565e27218a494a4bd1` 与授权一致。`evals/promptfoo/score.sh test` 配置验证通过，无模型请求。
- 初次 host 12573515776B / VM 93889794048B 未满足原父最低 18GiB，未下载原父层。之后外部清理使宿主约 99GiB，重新实测双边空间并按 `3C+Ucap+S10+B2+A1+安全5GiB` 准入；原父镜像 `sha256:b23d48839408524326095c15ada4145c772b87646cf299022ff1f58c97baba91`（linux/amd64），C=50931626B，完整流 SHA / diff_id 验证后的 exact U=146356736B，导入后配置与 RootFS 完全匹配。原父冻结 ELF 无密钥离线 `--help` 实际通过。
- 原父标准 `suite.scoring.harbor_trial` / `SuiteDockerEnvironment` nop 已完成，原始 verifier 6 测试（1 通过、5 缺产物失败），reward=0；oracle 已结束：原始 apt 下载失败，随后 `add-apt-repository` 缺失；官方 verifier 同样 6 测试（1 通过、5 缺产物失败），raw reward=0。该 oracle 校准标记 invalid/unscored、能力成绩 null、基础设施失败，不计模型能力 0；原始 Harbor summary/reward 保留，派生归类在 `checkpoint.json`。没有重试或预修父 Agent。资源保持 4CPU / 8192MiB / 10240MiB，Agent/verifier 各 1800 秒；无私有 Trial、platform/FROM/Compose 覆盖或全局配置修改。
- Cargo 首阶段动态领取槽位 1，绝对 `CARGO_TARGET_DIR` 和 `CARGO_BUILD_BUILD_DIR` 均为 `/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1`，`CARGO_BUILD_JOBS=1`，执行 `cargo clippy --all-targets --all-features -- -D warnings`。外部删除整个 target（含活跃锁和缓存）导致临时目录消失，退出 101，原始失败日志未覆盖；本会话未删除他人锁。已重新动态领取槽位 1 重建验证，最终退出 0；有效行为测试数 0（Clippy 是静态检查，不代替上述 48 项及评分验收）。自己的 owner/锁正常释放，缓存保留。未修改 Rust，未运行 fmt、全量测试、wasm 或 Makefile。
- 原始日志、安装失败、真实依赖/哈希、容量/OCI、父题评分与 Cargo 收据均保留于忽略目录 `evals/promptfoo/results/qwen38-suite/diagnostics/rstan-to-pystan/`。诊断 20 fixture、8 标准控制和三条任务验证命令尚无有效验收证据，不能以零测试、父题 nop 或安装成功替代；任务文件保留，不提交仅状态更新。两个停止探针容器已在完整 `/tmp` 归档 SHA256 校验后按精确 ID 删除，父题 Trial 容器已结束并清理；原镜像、唯一 OCI/层、真实依赖、原始失败/评分及 Cargo 缓存保留，独占窗口已释放。
- 恢复入口：先解决真实 Stan 扩展在既定 512MiB 下的编译可行性（保留本次 OOM；不得自动提高预算），本轮没有新增 clang 工具链，也未试图修改 PyStan 源码、预置答案或调高预算；若后续采用官方 clang，须先固定来源/SHA、重新准入并在同预算做一次真实探针。再制作/校准四个独立合成子题，完成 TDD、20 fixture、8 标准控制、官方父题 oracle/nop 与正式 preflight；重新动态领取 Cargo 槽位，使用新的双边容量实测，不沿用旧快照。

目的：为“统计模型接口转换”准备原题完整环境与四道独立诊断题。

来源任务：Terminal-Bench2.1/rstan-to-pystan

预计会话范围：只准备一个父题的四个窄诊断题和确定性校准；不运行付费模型。长镜像拉取允许记录进程后续接，不能把未校准标成完成。

## 文件

- 新建：`evals/terminal-bench/suite/cases/rstan-to-pystan/case.json`、`evals/terminal-bench/suite/cases/rstan-to-pystan/check.py`、`evals/terminal-bench/suite/cases/rstan-to-pystan/tests/test_cases.py`、`evals/terminal-bench/suite/cases/rstan-to-pystan/fixtures/`、`evals/terminal-bench/suite/cases/rstan-to-pystan/oracle/`、`evals/terminal-bench/suite/cases/rstan-to-pystan/environment/Dockerfile`、`evals/terminal-bench/suite/cases/rstan-to-pystan/README.md`
- 只读完整上游包：`evals/promptfoo/results/qwen38-suite/source/tasks/rstan-to-pystan/`
- 忽略产物：`evals/promptfoo/results/qwen38-suite/preflight/rstan-to-pystan/`、`evals/promptfoo/results/qwen38-suite/diagnostics/rstan-to-pystan/`

## 上下文

原题 rstan-to-pystan；类别 data-science；CPU=4、内存=8192MiB、存储=10240MiB、GPU=0；Agent=1800秒、verifier=1800秒。预建镜像 alexgshaw/rstan-to-pystan:20251031。原题树SHA=c0f640ad3dffd723bedb0298bce4c5503f89160f；题面SHA256=7b6dd84eb47579469556915a5eab6d303217bba438424eaee4db52375d5add2a；配置SHA256=ed0cd4382cb353e73cb575aa7a20b624e35f09ddd8769c332f40f8986980e13f。读完整 instruction.md、输入、solution、tests 后设计，禁止改原题。四子题：m1 解析合成训练与测试数据；m2 转换模型与采样接口；m3 校验参数维度与随机条件；m4 比较后验和预测产物。

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

先使用技能 ./skills/create-agent-evals/SKILL.md；每个子题定义合成输入、清楚完整题面、输出路径/API/服务行为与独立确定性判定。新增 evals/terminal-bench/suite/cases/rstan-to-pystan/tests/test_cases.py：正解通过，至少3个有意义错误变体被拒绝；运行真实程序/产物，不验证关键词、自述或仅Agent自测。错误fixture和漏检checker先失败，再最小修复。

验证命令：

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/rstan-to-pystan/tests -p test_cases.py
```

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/rstan-to-pystan/case.json --run-fixtures
```

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.run preflight --catalog evals/terminal-bench/suite/catalog.json --runtime evals/promptfoo/results/qwen38-suite/runtime.json --parent-id rstan-to-pystan --output evals/promptfoo/results/qwen38-suite/preflight/rstan-to-pystan
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
