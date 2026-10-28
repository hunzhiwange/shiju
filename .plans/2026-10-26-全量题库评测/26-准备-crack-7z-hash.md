# 任务 26: 准备-crack-7z-hash

批次：【批次 14】；依赖批次：1、2、3、4、5。同批无文件依赖；Git 提交按任务编号串行，禁止删除其他会话的索引锁。

状态：已阻塞

目的：为“本地压缩包恢复”准备原题完整环境与四道独立诊断题。

来源任务：Terminal-Bench2.1/crack-7z-hash

预计会话范围：只准备一个父题的四个窄诊断题和确定性校准；不运行付费模型。长镜像拉取允许记录进程后续接，不能把未校准标成完成。

## 文件

- 新建：`evals/terminal-bench/suite/cases/crack-7z-hash/case.json`、`evals/terminal-bench/suite/cases/crack-7z-hash/check.py`、`evals/terminal-bench/suite/cases/crack-7z-hash/tests/test_cases.py`、`evals/terminal-bench/suite/cases/crack-7z-hash/fixtures/`、`evals/terminal-bench/suite/cases/crack-7z-hash/oracle/`、`evals/terminal-bench/suite/cases/crack-7z-hash/environment/Dockerfile`、`evals/terminal-bench/suite/cases/crack-7z-hash/README.md`
- 只读完整上游包：`evals/promptfoo/results/qwen38-suite/source/tasks/crack-7z-hash/`
- 忽略产物：`evals/promptfoo/results/qwen38-suite/preflight/crack-7z-hash/`、`evals/promptfoo/results/qwen38-suite/diagnostics/crack-7z-hash/`

## 上下文

原题 crack-7z-hash；类别 security；CPU=1、内存=4096MiB、存储=10240MiB、GPU=0；Agent=1800秒、verifier=900秒。预建镜像 alexgshaw/crack-7z-hash:20251031。原题树SHA=103ee113a33d0faec2427ee4268efff9acd35b83；题面SHA256=f5f89400c549eefb2c245caa7770356737b7a12098e6879787e846d9cb5eb822；配置SHA256=8e192bcbe00e817419bde7ff9b562a5fa0c1e24ec4f3384cba4a31eda0a7b013。读完整 instruction.md、输入、solution、tests 后设计，禁止改原题。四子题：m1 识别合成压缩包结构；m2 验证小字典中的候选口令；m3 处理错误口令与文件名；m4 提取并保存目标内容。

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

先使用技能 ./skills/create-agent-evals/SKILL.md；每个子题定义合成输入、清楚完整题面、输出路径/API/服务行为与独立确定性判定。新增 evals/terminal-bench/suite/cases/crack-7z-hash/tests/test_cases.py：正解通过，至少3个有意义错误变体被拒绝；运行真实程序/产物，不验证关键词、自述或仅Agent自测。错误fixture和漏检checker先失败，再最小修复。

验证命令：

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/crack-7z-hash/tests -p test_cases.py
```

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/crack-7z-hash/case.json --run-fixtures
```

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.run preflight --catalog evals/terminal-bench/suite/catalog.json --runtime evals/promptfoo/results/qwen38-suite/runtime.json --parent-id crack-7z-hash --output evals/promptfoo/results/qwen38-suite/preflight/crack-7z-hash
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


## 2026-10-27 本次执行与阻塞证据

- 已使用仓库执行计划及创建评测技能；批次 1–5 依赖已核对：89 题冻结 catalog、容量策略、实验契约、generation 2 冻结运行时与 agent 验证证据齐全。本题 9 个上游文件全部匹配 catalog；未改 plan.md、上游、共享实现、评分、模型、预算或超时。
- 原镜像 `alexgshaw/crack-7z-hash@sha256:0f4453abd774c5a3d3d7e66ba28fae88ec2e49ada3a993b324ebc16c348666d3`，linux/amd64；配置 `sha256:cd0a22dc1d928e3075936ac702626ae529a683ac25aed25103ffaf33b72074d5`。所有压缩层及解压 diff_id、实际 RootFS 链均匹配；压缩 355841799 字节、解压 tar 758772736 字节。保守磁盘峰值 22227392789 字节，低于宿主及 VM 可用空间；实测 CPU 100000/100000、memory.max=4294967296。原 storage_mb=10240 保持不变；后端不实施 writable-layer 配额。
- 原始指定 preflight 退出 1：`qwen38-suite/runtime.json` 缺少 historical_root（KeyError），这是旧身份收据，不是可执行配置。已沿用其他准备题验证过的 generation 2 生成本题忽略目录内 runtime.json，保留 workingDirectory=/app、agentTimeoutSec=1800；运行时身份复核通过。
- 真正的完整 preflight 命令：`PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.run preflight --catalog evals/terminal-bench/suite/catalog.json --runtime evals/promptfoo/results/qwen38-suite/diagnostics/crack-7z-hash/runtime.json --parent-id crack-7z-hash --output evals/promptfoo/results/qwen38-suite/preflight/crack-7z-hash-valid-runtime`，退出 1。
- 主要阻塞：官方 oracle 在 UTC 11:08:49.921977 开始，11:38:49.938881 达到原定 1800 秒，产生 AgentTimeoutError。未预装依赖、跳过 John 阶段或使用已知口令回填。官方 apt 已成功安装 libcompress-raw-lzma-perl；John 在受限 amd64 仿真中约 6 候选/秒，single 阶段用时 27:06，之后进入原 password.lst 字典；超时前仍 0g，未生成 solution.txt。当前证据不能把它当作 oracle 正常通过。
- 次要阻塞：verifier 于 UTC 11:38:50–11:43:09 尝试官方 apt curl 下载，libcurl4t64/curl/libcurl3t64-gnutls 的 Ubuntu 端点连接失败（198.18.0.111:80），继而 curl、uvx 不存在，没有运行 pytest。原始 reward=0 不是有效能力失败；suite 正确汇总 score_status=unscored、reward=null、failure_class=infrastructure。即使仅修 verifier 下载，也不能消除 oracle 已超时的主要阻塞；未在没有条件变化时重复 30 分钟 oracle。
- 四道独立合成诊断已完成草稿并验证真实程序：m1 清单与未压缩大小；m2 小字典口令完整性验证；m3 错误口令/缺失成员及空格文件名；m4 精确 UTF-8/换行内容保存。每道都有 3 个有意义、语法有效的错误程序；隐藏 probe 使用另一归档、不同口令/内容。未复制上游隐藏材料到草稿。
- TDD 红灯：故意宽松 checker 的 20 个测试中 16 个 nop/负例漏检失败，证据 red.log；修复后执行指定题级 unittest 命令退出 0，有效测试 21 项（20 个真实容器控制、1 项控制程序语法检查）。执行指定 `suite.diagnostics validate --case evals/terminal-bench/suite/cases/crack-7z-hash/case.json --run-fixtures` 退出 0，20 个 fresh/offline 控制全符合预期（4 gold、4 nop、12 负例）。
- 另行使用原 suite.scoring.harbor_trial 对四道 materialized 子题进行 8 个 fresh Harbor 控制，全部 oracle=1、nop=0，退出 0。materialized checker、全部 fixture/oracle/gold 字节已逐一与最终草稿核对一致。诊断镜像 `vw-crack-7z-diagnostics@sha256:42a4b8345a99449b538d54f05fdbc7c3bfffcc58e36b90473198aff38d8d1384`，linux/arm64；只含 digest 固定 Python 基础镜像及 p7zip，不含题目材料。fixture 容器 1CPU/512MiB、无网络、只读 rootfs、128 PIDs、cap-drop ALL，评分材料独立只读注入。
- `evals/promptfoo/score.sh test` 退出 0；`cargo clippy --all-targets --all-features -- -D warnings` 退出 0，Rust 行为测试数量 0。动态领取实际槽位 1，`CARGO_TARGET_DIR=/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1`，`CARGO_BUILD_BUILD_DIR` 同值，`CARGO_BUILD_JOBS=1`；自己的记录与锁已释放，缓存保留。仅既有 block 0.1.6 future compatibility 提示。没有 Rust 改动，未执行 fmt、全量 cargo test、wasm 或 Makefile 命令。
- 全组 preflight 在原题 oracle 失败后停止，因此没有原题 nop 或成功的完整 preparation 收据。本题不就绪；本轮模型调用 0，有效原题评分 0。诊断控制不是正式模型成绩。
- 原镜像 docker pull 已正常退出 0；并行同 digest Registry 下载曾遇 SSL EOF，保留失败与续传日志，续传后所有层校验通过，OCI 归档保留。没有导入替代镜像、改变全局 Docker 配置或清理别人的资源。所有本题下载/校准/Clippy 子进程已结束，按精确名称查询本题容器为空，未遗留自己的 Cargo 锁。
- 按阻塞约定，仅保留本编号任务文件的状态及证据变更，不提交、不删除任务文件。已验证诊断草稿全部移入忽略目录 `evals/promptfoo/results/qwen38-suite/diagnostics/crack-7z-hash/draft-case/`，不发布未就绪 case；没有留下本题源目录的未提交资产。任务 25 已于本题收尾前提交 `599f6f9b6`，没有越过串行提交限制。
- 原始证据：`evals/promptfoo/results/qwen38-suite/diagnostics/crack-7z-hash/blocked-evidence.json`、`dependencies.json`、`parent-image.json`、`image-capacity.json`、`parent-resource-probe.log`、`red.log`、`green.log`、`validate.log`、`diagnostic-harbor.log`、`clippy-slot.json` 与 `preflight.log`；原题完整 result、exception、oracle/verifier 原始日志在 `evals/promptfoo/results/qwen38-suite/preflight/crack-7z-hash-valid-runtime/calibration/crack-7z-hash-oracle/`。四子题 Harbor 原始收据在 `preflight/crack-7z-hash-children/`。
- 恢复入口：先协调并授权原生 amd64 环境，在相同原镜像、1CPU/4GiB、1800 秒下 fresh 执行原 oracle；另修复 verifier-only 依赖下载并重新校准。不得通过加 CPU/超时、预热 agent 依赖、跳过 John、已知口令或迟到产物冒充正常通过。环境条件改变后恢复 draft-case 到原题目录，重新运行题级测试/20 个 fixture 控制，再取得原题 oracle/nop 与完整 10 控制 preflight，最后才提交和删除本文件。


## 2026-10-27 有界安全恢复审计（仍已阻塞）

- 未重复 1800 秒原题 John 命令，未调用模型/Cargo，未读取或使用已知口令、缩小搜索范围、预填 solution.txt、预装 agent 依赖、增加资源或修改全局 Docker 设置。原镜像 digest、原题 9 个文件均再次匹配冻结身份；诊断草稿 validation.json 的全部声明文件摘要及 20 个 fixture 控制仍完整，另有 8 个既有子题 Harbor 控制保留，不作为整题就绪证据。
- 超时记录复核：agent_execution 起止相差 1800.016904 秒，异常确为 AgentTimeoutError（原限时 1800 秒）。原 HostConfig NanoCpus=1000000000、Memory=4294967296，与原实测 cpu.max=100000/100000、memory.max=4294967296 一致，不是误设更低 CPU/内存。Docker daemon 当前 aarch64，原镜像 amd64，短探针内部 uname=x86_64，确认跨架构执行；未臆测具体仿真引擎。
- 官方日志原本报告 4 个 OpenMP 线程。仅做两次离线内置合成基准 `/app/john/run/john --test=5 --format=7z`，每次外层硬限 40 秒，均同一原镜像、1CPU/4GiB、无网络、只读根目录、无原题输入/哈希/仓库挂载。默认环境实测 /proc 线程数 4，Cpus_allowed_list=0-3，7z 内置基准 18.2 c/s；诊断性 OMP_NUM_THREADS=1 实测线程数 1，19.9 c/s。两次均退出 0。原题命令和原题环境没有改为单线程。
- 默认四线程 cpu.stat 的 nr_throttled 从 0 增至 119，throttled_usec 增至 24129287；单线程为 30、89347。证实 1CPU 配额下四线程存在调度/节流开销，但单次合成吞吐仅提高约 9.34%。基准 padding/compression/data length 与原题不同，不能把 18–20 c/s 和原题约 6 c/s 直接比较或外推整题耗时，也没有实证支持原 oracle 可在 1800 秒内正常完成。因此不据此改变默认环境或重跑整题。
- 结论：没有发现当前授权内已验证的资源误配或瞬时恢复路径。原题主要计算超时仍未解除；verifier curl 下载失败是另一个独立问题，即使 verifier-only 网络恢复，也不能抵消 oracle 超时。下一步需协调并授权原生 amd64 执行环境，严格保留同 digest、原命令、1CPU/4GiB、1800 秒及 fresh 未知口令状态；另恢复 verifier-only 依赖端点。若将来考虑 OMP 配置，须协调后重新完整校准，不能把本次短基准当通过证据。
- 两个临时容器均先归档完整 inspect，再按精确 ID 删除，cleanup exit=0：`0a3b115aa052a9d24c99b830cbcbeb73c2d08b698cab6f6216bdc3034a0f13e1`、`590db6958301682dd080f85a44fe48f6a48e8df38063d4673e7e8a0f3a087575`。再次查询本题容器、宿主 John/审计进程及本任务 Cargo 锁均为空，其他业务容器不动。本次无 Cargo 调用，无新增槽位，既有槽位 1 缓存继续保留。
- 新证据位于忽略目录 `evals/promptfoo/results/qwen38-suite/diagnostics/crack-7z-hash/`：`recovery-timeout-review.json`、`recovery-benchmarks.json`、`recovery-default.log`、`recovery-one-thread.log`、对应 `*-inspect.json`、`recovery-audit-summary.json`。仅追加本任务文件说明；保留已阻塞，不提交、不删除任务，诊断草稿及有效校准保持原位。


## 2026-10-27 单线程资源配置复验（按授权中断，仍已阻塞）

- 判定：OMP_NUM_THREADS=1 限制已有 OpenMP 计算线程数，属于执行资源配置；不更改 John 命令、候选词典、single/wordlist 搜索阶段、口令信息、输入、镜像、依赖、评分或官方资源/时限。采用 Harbor 已有 `agent.env` 接口，仅 oracle agent 阶段注入，不修改 task.toml、solve.sh、共享 runner 或全局 Docker 设置。执行身份、Harbor 固定 revision/源码摘要、完整原题文件摘要及原镜像 digest 均记录在新 trial 的 execution-identity.json。
- 按新授权仅启动一次 fresh 完整 oracle 校准，原 1800 秒超时轮次原样保留。通过原子 O_EXCL attempt 记录限制本脚本只能启动一次；本次最终未完整完成，没有再次启动其他 oracle。原题 agent/verifier 仍为 1800/900 秒，1CPU/4GiB/10240MiB，原镜像 digest 不变，无模型调用、Cargo 调用或新增槽位。
- fresh 检查退出 0：/app/solution.txt、/app/secrets.hash、john.pot、john.rec 均不存在，未挂载旧 John 状态；输入压缩包 SHA256 仍为 a2e13fcbb4c2c8e92b1bf9d78ec91376705d1f5631d5bd4de64f82aa0553b773。实际 /proc/221/environ 确认为 OMP_NUM_THREADS=1，/proc 线程数 1；cpu.max=100000/100000、memory.max=4294967296。实际 apt 子进程路径可见 /run/rosetta/rosetta，确认当前 Rosetta 跨架构执行，而不是仅从 uname 推测。
- 实际执行证据推翻短基准外推：原题同一 single 阶段，新轮次在 John 1:34 为 1.98%、2.200 p/s；4:14 为 8.90%、2.346 p/s；7:12 为 11.10%、2.250 p/s。旧已超时轮次在 6:55 为 31.28%、约 6.082 p/s。新轮次 apt 更新另耗约 2:33，并存在原路径下载重试；John 本身仍持续明显更慢，CPU 时间接近墙钟且保持单线程。没有可核验的后续阶段加速或搜索覆盖差异足以支持在剩余官方时间追回旧失败覆盖量，不能把合成基准约 9% 改善当作原题改善。
- 收到调度明确授权停止无意义等待后，记录为 **配置复验中断、未评分、资源限制未改善**。原脚本保持原样，未顺带调整搜索策略或再发起 oracle。新 agent_execution 为 UTC 12:49:30.221459–13:01:10.738599（约 700.517 秒）；原题 John 的中断前进程快照为 elapsed 8:38、CPU 8:30。新 trial 原始异常是 CancelledError，宿主退出码 130；不是新一次 AgentTimeoutError，也不是能力失败。新 verifier/pytest 未运行，reward=null、score_status=unscored；旧轮次原始 1800.016904 秒超时继续独立保留。
- 先归档实际容器 inspect、原始 oracle 日志、进程和 cpu.stat，再仅向本脚本 PID 29670 发送 SIGINT。Harbor 自动清理本轮容器 `902d78360945e67877cd5d08bc1bdd38dff85bbe8c698ee2758ab515b7e23fdf` 及 John；后续按该精确 ID 删除返回 No such container，证明已由 Harbor 删除。独立复查 PID 29670/29738/29740 均不存在，本题容器列表为空，自有 Cargo 锁为空；未停止其他任务或业务容器。
- 本题继续已阻塞；没有原题有效 oracle/nop 或官方通过评分。已校准诊断草稿留在忽略目录 draft-case，validation.json 的全部声明字节再次匹配；原有 20 个 fixture 控制、8 个 Harbor 子题控制保留，不冒充整题就绪。下一步需要协调并授权原生 amd64 执行环境，保持原镜像、原命令、1CPU/4GiB、1800 秒、fresh 未知口令状态及原评分；另修复 verifier-only 依赖下载。不得继续在当前配置循环重试。
- 新证据目录 `evals/promptfoo/results/qwen38-suite/diagnostics/crack-7z-hash/omp1-calibration/`：`execution-identity.json`、`fresh-agent-start.json`、`authorized-interruption.json`、`interrupted-container-inspect.json`、`oracle-final.log`、`processes-final.log`、`cpu-stat-final.log`、`cleanup-and-conclusion.json` 及 `crack-7z-hash-omp1-oracle/result.json`、exception/agent 原始日志。上级目录另保留 `omp1-authorized-attempt.json`、`omp1-actual-thread-proof.log`、`omp1-live-processes.log`、`omp1-calibration.log` 与原样执行脚本。仅追加本编号任务文件说明，不提交、不删除任务，plan.md 不动。
