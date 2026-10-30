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


## 2026-10-28 当前共享工作区恢复核验（仍已阻塞）

- 按本轮恢复要求重读任务、原超时、OMP 中断及缓存维护证据；批次 1–5 实现依赖按调度已满足，7 待回归。任务 22 的完成不会解除本题独立的 John 计算瓶颈。未重复已证明无意义的长运行、短基准、子题控制、模型或付费调用；未改镜像、输入、命令、评分、资源、超时、模型或预算，也未改写请求绕过平台限制。
- 实际核验本机 uname=arm64；Docker daemon=aarch64、4CPU、12530401280 字节。现有 Docker contexts 只有 default、desktop-linux，分别为本地 unix socket；显式对两个 context 执行 docker info 均退出 0，daemon ID 都是 60413b4f-4925-4af1-91f2-8737d178d481，架构均 aarch64。因此不是存在一个尚未使用的原生 amd64 context。
- 对已配置 dslr1 仅做一次只读连接探测：`ssh -T -o BatchMode=yes -o StrictHostKeyChecking=yes -o ConnectionAttempts=1 -o ConnectTimeout=8 -o ServerAliveInterval=5 -o ServerAliveCountMax=1 dslr1 'uname -m; command -v docker; docker info --format "{{.Architecture}} {{.NCPU}} {{.MemTotal}}"'`。退出 255，Connection closed by 39.108.122.33 port 22；没有远端命令输出，不能验证其架构、Docker 或资源，未上传材料、安装依赖、改 SSH 配置/host key 或启动远程任务。
- 本轮再次核验 9 个上游文件全部匹配冻结 catalog；380 个保留文件全部重新计算 SHA256 并匹配维护前清单，包含完整 OCI、诊断草稿、原始评分/异常、1800.016904 秒 AgentTimeoutError 及 OMP CancelledError/中断证据。diagnostic validation 的 20 控制和 8 个既有 Harbor 子题控制继续保留，未冒充本轮新校准或整题就绪。原题父镜像缓存与重复 tar 已按 2026-10-28 维护报告移除，完整 OCI 与精确 tar 重建 recipe 留存；本轮未恢复/导入、重新下载或构建这些缓存。
- 当前没有可用原生 amd64 执行入口，也没有同资源约束下新的性能恢复证据。原 4 线程 single 阶段约 6 候选/秒、27:06 才结束并最终超时；已授权的单线程实题复验仅约 2.2–2.35 候选/秒且按授权中断。这些都与包下载是否改善独立。仅改善 verifier-only 下载或复用别题工具链，不能解决本题已证实的计算时限门槛；不能据此再跑一轮完整 oracle。
- 最小解除条件：提供/恢复一个已授权且可连接的原生 amd64 Docker 执行入口，能按同一原镜像 digest、原 John 命令、1CPU/4GiB/10240MiB、1800 秒和 fresh 未知口令状态执行，并另行确认 verifier-only 依赖可用。原生入口本身仍须 fresh oracle/nop 验证，不等于预先保证通过。若采用当前 ARM64 环境，则必须先有保持同一约束的实证性能恢复，不能靠推测、加资源、改候选集合、已知口令或迟到产物替代。
- 收尾：本题容器及自有 Cargo 锁为空；没有启动 John、oracle、子控制或后台下载，无 Cargo 调用/新增槽位。仅追加本任务文件记录，无需提交；保留已阻塞、不删除本文件、plan.md 只读。新只读审计收据：`evals/promptfoo/results/qwen38-suite/diagnostics/crack-7z-hash/current-recovery-20261028.json`，维护与恢复指南继续为 `maintenance-20261028/result.json`、`RESTORE.md`、`exact-tar-recipe.json`。


## 2026-10-29 执行入口与可信控制面深入恢复（仍已阻塞）

- 本轮未停留于单次 SSH 失败。只读核对全部现有服务器别名：dslr1=root@39.108.122.33:22、dslr2=root@39.108.141.168:22、dslr3=root@120.79.160.161:22；dslr4 与 dslr1 同地址/用户/端口，仅 IdentityFile 为 id_rsa_new。各别名没有 ProxyJump、ProxyCommand、ControlPath，ControlMaster=false、ControlPersist=no。配置中的用户/端口未替换；没有打印、复制或提取密钥/令牌。现有 id_rsa/id_rsa_new 归当前用户、权限 0600；ssh-agent socket 存在但 ssh-add -l 返回无身份。没有活跃 SSH 主连接、控制 socket 或已建立的端口 22 连接可复用。
- 对 dslr1 使用 -vvv、BatchMode=yes、StrictHostKeyChecking=yes、ConnectionAttempts=1、ConnectTimeout=8，客户端明确先输出 Connection established 和 Local version string，随后 kex_exchange_identification: Connection closed by remote host，退出 255。没有收到 Remote protocol version/服务器 banner，也没有开始 KEX 或认证；不能把这当作密钥不匹配、密码错误或需要放宽加密算法。dslr2、dslr3 各一次相同限定的默认路径只读探测，也在服务器 banner 前关闭，均退出 255；dslr4 的不同密钥不能修复同一端点的认证前关闭，未重复探测。
- 找到具体客户端网络证据：系统已配置 SOCKS/HTTP/HTTPS=127.0.0.1:7897；保存的 dslr1 地址路由经 utun1024/198.18.0.1，而物理默认接口为 en0、源地址 192.168.10.205、网关 192.168.10.1。仅本次连接显式使用系统已有 SOCKS（nc -w 8 -X 5 -x 127.0.0.1:7897 %h %p）仍在 banner 前关闭，退出 255；未改全局代理、TUN、路由表或安全策略。
- 按调度补充要求先阅读本机 nc/ssh_config 文档，核实 nc -b 为 socket 接口绑定、-s 为本地源地址、-G 为连接超时；SSH BindInterface 仅取接口源地址，未混为 socket 接口绑定。随后只对既有主机/原端口做有界物理接口探测。dslr1 的 nc -b en0 路径仍在 banner 前关闭；加上真实源地址的 `/usr/bin/nc -v -b en0 -s 192.168.10.205 -G 8 -w 15 %h %p` 明确返回 `connectx ... port 22 (tcp) failed: Connection refused`。dslr2、dslr3 各一次相同物理接口/源地址/原端口限定，均 `Connection timed out during banner exchange`，退出 255。合计 8 个只读连接探测，没有扫描未知主机或端口、猜账号、停用 host-key/TLS 校验，也没有遇到可据以改变服务器策略的明确许可。
- 结论限定：dslr1 的默认逻辑路由会先建立 TCP 再在 banner 前关闭，显式物理源地址路径显示 TCP 拒绝；另外两个主机 banner 超时。当前无法仅凭客户端证据判定具体是哪台服务/防火墙/中间链路拒绝，不能臆断 sshd 已停止、旧 IP 已失效或安全组规则错误。尚无正常 banner/认证，因此没有可在本机安全完成的密钥、加密算法或 SSH 地址修复；没有通过另一端口/身份/隧道绕过访问控制。
- 已检查现有计算入口：两个 Docker context 的 daemon ID 相同，均为同一 ARM64 daemon；Buildx 仅 default/desktop-linux docker driver，同样指向本机，列出 amd64 平台不等于原生 amd64 主机。Codex list_projects 返回项目均 local，所见 thread host 为 local/null，未暴露已认证的远端执行会话或复用接口。GitHub 只读 GET hunzhiwange/shiju/actions/runners 成功，self-hosted runner 总数 0；仓库现有 workflow 为发布/预检查/恢复和固定 Rust CI，不是通用命令入口，未触发发布、测试、创建工作流或新 CI 作业，也未通过输入注入利用其执行代码。只读 gh codespace list 因当前凭据无 codespace scope 返回 404；未扩大 scope，不能据此声称不存在 Codespace。当前没有 aliyun/aws/gcloud 已安装 CLI，未新建凭据或安装 CLI。
- 找到了现有已登录 Chrome 阿里云主账号控制台，依授权只读查看 ECS，而非再要求用户先提供新账号。杭州实例页为首次开通引导，深圳标准实例列表明确共 0 条；随后资源报表 https://ecs.console.aliyun.com/resources-report 在地域=全部、付费类型=全部、查询日期 2026-09-28 至 2026-10-28 时显示实例总数 0、vCPU 总数 0、内存 0.00GiB、无符合条件记录。此结果只代表当前登录账号及报表范围，不能推断其他账号/其他云产品或报表日期之后没有实例。没有找到保存主机的可信实例身份/当前 IP/SSH 端口/安全组映射，因此未改 SSH 地址，也未修改安全组、防火墙、服务、实例状态或重启实例，未启动新资源。
- 唯一必要用户动作已具体化：**请打开 dslr1 实际所属云账号中的现有实例详情页**（当前已登录账号的全地域报表截至 10-28 为 0 实例），让代理能核对当前实例身份、IP/SSH 端口/状态和只读安全组规则。无需提供密钥、密码、令牌或创建账号。若可信控制面证明保存的地址/端口过时，再形成可审查的最小连接配置修复；若端点未变，则依控制面证据定位对应监听/来源可达性，不能猜测修改安全策略。已保留现有资源报表标签页供接续。
- 没有可用原生执行入口或新的同资源性能恢复证据，因此不重跑 ARM 上的 John/oracle/短基准/子控制，不重新导入父镜像缓存、下载、构建或调用付费模型。原 1800 秒超时、OMP 按授权中断及已校准诊断草稿均保留；诊断 validation 全部声明字节再次核对一致，未冒充整题通过。尚无原题有效 oracle/nop，不删除任务、不提交状态说明，plan.md 只读。
- 收尾核查本题容器、SSH/nc 探测进程和自有 Cargo 锁均为空；未运行 Cargo，无新增槽位，其他业务容器和独立任务未触碰。详细只读证据为 `evals/promptfoo/results/qwen38-suite/diagnostics/crack-7z-hash/endpoint-recovery-20261029.json`，包含保存别名的有效配置、全部 8 次探测的实际出口/返回、文档依据、执行端/控制面范围及准确解除条件；此前维护恢复配方继续保留。


## 2026-10-29 实时云资源补核及同源 ARM64 兼容原型（原样仍阻塞）

- 未把 10-28 的历史 ECS 报表当作当前全部服务器不存在的证明。按后续指令补查当前标准 ECS 实例列表，深圳页在“账号全部资源”范围明确显示“全部地域均未查询到实例，请新建实例”、共 0 条；还从已观察到的产品入口进入同账号轻量应用服务器 SWAS，当前深圳列表无资源，地域下拉实时列出的中国、欧美及亚太全部地域数量均为 0。未找到任何保存 IP/名称所对应的可信现有实例，因而没有可进入的原实例 Workbench/免密控制台；没有推断其他账号不存在服务器，也没有改变实例、安全组、防火墙、服务或创建新资源。原样路径唯一待用户动作仍是：打开这些主机实际所属云账号中的 dslr1 实例详情页，以便核对真实 IP/SSH 端口和只读资源/规则。
- 随后按单独明确授权在忽略目录制作兼容原型材料；这不是原样校准恢复，不替换官方镜像或题包。目录：`evals/promptfoo/results/qwen38-suite/diagnostics/crack-7z-hash/compatibility-arm64-20261029/`。只从完整保留 OCI 的公开 John Git/build/package 层定向提取公开依赖身份，未提取原题 task-deps、未知口令、原题哈希、目标内容、参考解或评分材料用于原型。
- 原 John 身份可精确复现：Git commit `8b5bfefbdc9b643513037f30c8200f5dc0ffc841`，tree `ee4163c03a0f0792d2f3fb928751bc6a889e6029`。版本为 `1.9.0-jumbo-1+bleeding-8b5bfefbd 2025-05-01 10:25:09 +0200`；使用与原图像相同 `./configure --without-openssl && make`，MAKEFLAGS=-j1 保持串行。初次 git archive 的 export-subst 自动展开了两个 Makefile 版本元数据占位符，严格身份核对因此失败；没有放宽检查，已取原始 Git blob 恢复并重构建，且按原 reset --hard 固定 commit 恢复 export-ignore 排除的 CI 元数据。最终完整已跟踪树 git diff HEAD -- . 退出 0，无源码补丁；最后元数据恢复前后已验证二进制 SHA 不变。初次身份失败和全部中间日志均保留。
- 原生 Ubuntu24.04 arm64 基础缓存固定为 `sha256:24737cef17f4e22b1977df7d9a05e77c4e28b7da5b8911f8d8ccda136bf3573c`，没有下载/导入原 amd64 镜像。原型容器仅显式挂载公开 John 源码 tar 和自有 native-work 目录，未挂载原题输入、原题哈希、宿主密钥或整个工作区。所有编译及 John 原型进程限制为 1CPU/4GiB，安装/编译上限 240/600 秒；原题的 1800 秒和 1CPU/4GiB 规则未修改、未运行。依赖安装 227.800 秒、初次编译 86.487 秒、原字节复构建 9.778 秒，均退出 0。宿主没有安装/更新依赖；仅原型容器内使用公开 Ubuntu 包与可信公开 Git 工作目录配置。
- 最终 native ELF aarch64 John SHA256 `386f219891d2ae8db006b09afc56a4863d6ed94c9152a1b446f0e325254b5be1`；原始 Git 字节 tar SHA256 `08082dd15e6396f256598b452049a8ce996dc7d4478710cb8cb71645fb0b9a36`。构建为 Linux aarch64 ASIMD AC OMP、AES-CE、Crypto library None，与原 John 同版本/同源；原 x86 构建则 AVX2/AES 路径。新构建不需要同名新源码版本或任何算法改写。
- 自有合成验证有独立黑盒证据：新建独立 7z archive，声明合成口令 proto-61、payload 为 `native architecture compatibility probe` 加换行；用源码内 7z2john 生成其哈希（所需 Perl LZMA 包只在编译之后的合成验证容器中安装，未预装到任何完整题镜像）。真实 native John 在三个候选中正确恢复合成口令；独立错误字典为 0 recovered；随后另一固定 p7zip 工具容器使用实际恢复值解密，输出逐字节匹配自有 payload，SHA256 `f6a87ec9942847eb9a8307ff18ef6a6548fada0e63b359878ab19b12f9e7bb6d`。最终 synthetic-validation-final.json 全部检查通过；没有原题破解、原题 oracle/nop、子题重评或模型请求。
- 最终固定源码二进制的内置 5 秒 7z 合成基准为 52.3 c/s real、52.0 virtual，1CPU 配额下 4xOMP；成本 524288 iterations/padding4/compression128/data108，与旧 amd64 内置短基准成本相同。初次元数据展开构建的 43.5 c/s 也保留，未取多次模型结果或修改评分。两次为不同构建阶段的短合成观察，不是原题 single 阶段吞吐或稳定性证明；不得拿 52.3 与原题约 6 p/s 直接外推，更不能标记官方通过或保证 1800 秒内完成。
- 必须公开的可比性差异：源 commit/tree 和功能算法无补丁，OS 系列均 Ubuntu24.04，但架构/向量化/AES 实现及依赖字节发生变化；116 个共同包的版本也有变化，不是“仅改一个 architecture 字段”。例如 GCC13.3.0 包由 `13.3.0-6ubuntu2~24.04` 至 `...24.04.1`，libc6 由 `2.39-0ubuntu8.6` 至 `...8.9`，libgomp1 至 `...24.04.1`，git 由 ubuntu7.3 至 ubuntu7.4；make/build-essential 版本相同但 ARM 二进制字节不同。全部原/新版本、增删包及关键已安装文件 SHA 见 compatibility-delta.json、native-pretest-package-versions.lock（编译前合成依赖之前 231 包）、final-native-final-package-byte-identity.log。旧依赖字节的 ARM 精确复现未验证，不能声称已复原原环境；已证明的是 John 同源同版本原生可构建。原型构建路径为 /work，提议完整变体恢复 /app/john 路径，避免把路径元数据差异隐瞒为原样。
- 具体可审查完整变体方案：README.md 与 proposed-full-variant.Dockerfile 保持原 /app/john、configure/make、/app 工作目录与固定 task-deps 边界，固定本次 ARM 基础镜像、源 Git/tree/tar 和直接构建包版本；完整题 task-deps 未放入构建上下文，Dockerfile 未构建或校准。该方案的传递依赖仍须在正式变体前冻结完整版本/字节收据并拒绝漂移。用户须单独决定接受架构及依赖差异、以独立 `crack-7z-hash-arm64-compat` 名义完整校准，或要求先验证历史依赖快照；不得自动替代 canonical 原题、回填成绩、复用旧 John/pot/口令，或加资源/超时。用户没有批准前不执行完整变体原题校准。
- 收尾：原型缓存容器 `98eda8a336687c23bb4b7a8276e111f0d868d0f102085ce7b1bcae01efd6aebe` 已停止；所有原型验证/编译子进程正常结束后，停止空闲 PID1 sleep 使缓存容器状态 Exited(137)，stop 命令退出 0，不是合成测试超时。保留该明确归属的停止缓存及公开源码/产物，未碰别人容器。独立检查没有活跃原型/SSH探针进程、自有 Cargo 锁；没有 Cargo 调用/新增槽位或付费模型调用。原先 380 个保留文件重新 SHA256 核验全部不变，包括原超时/中断/评分材料/OCI与诊断草稿。
- 新证据入口：兼容目录 README.md（审查方案）、final-state.json（停止与边界）、synthetic-validation-final.json、independent-decryption-final.json、final-native-sevenzip-benchmark.log、full-public-checkout-verification.json、raw-source-correction.json、build-receipt.json、raw-source-rebuild.json、compatibility-delta.json、material-files-sha256.json。任务36若需参考，只复用公开 Ubuntu/GCC/源码/架构身份调查和缓存元数据，不复用评分。本次仅更新本编号任务记录，不提交、保留已阻塞、不删除任务，plan.md 只读。
