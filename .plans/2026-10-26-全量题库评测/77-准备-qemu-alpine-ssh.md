# 任务 77: 准备-qemu-alpine-ssh

批次：【批次 44】；依赖批次：1、2、3、4、5。同批无文件依赖；Git 提交按任务编号串行，禁止删除其他会话的索引锁。

状态：已阻塞

目的：为“虚拟机安全连接”准备原题完整环境与四道独立诊断题。

来源任务：Terminal-Bench2.1/qemu-alpine-ssh

预计会话范围：只准备一个父题的四个窄诊断题和确定性校准；不运行付费模型。长镜像拉取允许记录进程后续接，不能把未校准标成完成。

## 文件

- 新建：`evals/terminal-bench/suite/cases/qemu-alpine-ssh/case.json`、`evals/terminal-bench/suite/cases/qemu-alpine-ssh/check.py`、`evals/terminal-bench/suite/cases/qemu-alpine-ssh/tests/test_cases.py`、`evals/terminal-bench/suite/cases/qemu-alpine-ssh/fixtures/`、`evals/terminal-bench/suite/cases/qemu-alpine-ssh/oracle/`、`evals/terminal-bench/suite/cases/qemu-alpine-ssh/environment/Dockerfile`、`evals/terminal-bench/suite/cases/qemu-alpine-ssh/README.md`
- 只读完整上游包：`evals/promptfoo/results/qwen38-suite/source/tasks/qemu-alpine-ssh/`
- 忽略产物：`evals/promptfoo/results/qwen38-suite/preflight/qemu-alpine-ssh/`、`evals/promptfoo/results/qwen38-suite/diagnostics/qemu-alpine-ssh/`

## 上下文

原题 qemu-alpine-ssh；类别 system-administration；CPU=1、内存=4096MiB、存储=10240MiB、GPU=0；Agent=900秒、verifier=900秒。预建镜像 alexgshaw/qemu-alpine-ssh:20251031。原题树SHA=9c31b12071c4879040e12cc2dbc01568aa4b73bf；题面SHA256=8348c60490cb90bc690cba259dfe98996da4c3294c6d42bba1119b00c64de4de；配置SHA256=10a43ec6ee732eeba852c377f12e23178f7fa5a4868143f3699a15565ffb64c7。读完整 instruction.md、输入、solution、tests 后设计，禁止改原题。四子题：m1 核对镜像与网络参数；m2 启动沙箱客户机；m3 配置本地端口转发；m4 验证登录和服务存活。

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

先使用技能 ./skills/create-agent-evals/SKILL.md；每个子题定义合成输入、清楚完整题面、输出路径/API/服务行为与独立确定性判定。新增 evals/terminal-bench/suite/cases/qemu-alpine-ssh/tests/test_cases.py：正解通过，至少3个有意义错误变体被拒绝；运行真实程序/产物，不验证关键词、自述或仅Agent自测。错误fixture和漏检checker先失败，再最小修复。

验证命令：

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/qemu-alpine-ssh/tests -p test_cases.py
```

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/qemu-alpine-ssh/case.json --run-fixtures
```

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.run preflight --catalog evals/terminal-bench/suite/catalog.json --runtime evals/promptfoo/results/qwen38-suite/runtime.json --parent-id qemu-alpine-ssh --output evals/promptfoo/results/qwen38-suite/preflight/qemu-alpine-ssh
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


## 2026-10-28 只读调查与恢复入口

- 已读取 `./skills/do-task-plans/SKILL.md`、`./skills/create-agent-evals/SKILL.md`、只读 plan.md、完整上游 instruction、task.toml、Dockerfile、README、solution/solve.sh、tests/test.sh、tests/test_outputs.py；上游不存在额外输入文件。以 Python hashlib 对 catalog 中本题全部8个文件逐一校验，8/8一致，命令退出0。题面 SHA256=8348c60490cb90bc690cba259dfe98996da4c3294c6d42bba1119b00c64de4de、task.toml SHA256=10a43ec6ee732eeba852c377f12e23178f7fa5a4868143f3699a15565ffb64c7；catalog 记录树9c31b12071c4879040e12cc2dbc01568aa4b73bf。未修改上游、公共接线或 plan.md。
- 依赖批次1–5的冻结 source/catalog、capacity.json、contract-validation.json、suite/runtime-v2.json 与 VALIDATION.md、agent-validation.json 均存在；读取历史证据确认契约24个聚焦测试、适配器8个测试通过，runtime-v2 的冻结 ELF 校准在 VALIDATION.md 中记录。它们是依赖历史证据，不能替代本题新校准。当前公共接线另有未提交变更，正在任务13会话修复，生产身份须在恢复时重新确认。
- 明确阻塞：调度者本轮消息指定任务13独占容器窗口，禁止本会话 Docker、镜像层下载/导入、标准控制及父题启动，直至明确释放。收到限制后未调用 Docker；限制前仅一次只读 `docker info --format '{{json .}}'`，退出0，4CPU、12530401280字节、aarch64、9个运行中容器；这不构成本题资源准入或镜像可运行证据。没有镜像digest、QEMU客户机、SSH服务或本题oracle/nop证据，不能标已完成或待回归。
- 已读取 suite/diagnostics.py 的真实执行路径：fixture校准为离线、512MiB、只读根文件系统、120秒；标准子题gold仅复制文件到/app，由checker执行并独立探测行为。上游客户机参考解使用1024MiB并联网apk安装OpenSSH，不能直接复制该流程作为子题或假定现有诊断镜像具备ISO/sshd。恢复后须先在授权标准环境实测可用启动路径；不能提高固定policy预算、改公共资源配置、借用父题参考解前缀或伪造服务回放来凑通过。
- 独立题设计待实测后落地：m1合成启动规格与真实qcow2介质，通过qemu-img检查及真实启动验证镜像/网络参数；m2离线合成客户机介质及串口随机挑战验证真实启动；m3独立客户机及随机TCP服务，通过本地转发往返和错误端口拒绝验证；m4独立SSH客户机，通过正确/错误口令、随机命令往返和重复连接验证登录与服务存活。每题至少3种负例应分别覆盖错误介质/架构、启动退出或虚假串口、错误目标端口/无转发、错误认证/宿主冒充等实际行为，不以配置文本、进程名或自述判成功。以上仅设计，无已实现或已验证声明。
- 本轮仅宿主只读文件/哈希检查，退出0，有效本题行为测试0。未运行任务指定unittest、--run-fixtures、preflight、score.sh test或Clippy：尚无本题代码，且容器校准窗口被明确禁止。未执行任何Cargo命令，未领取槽位，CARGO_TARGET_DIR不适用，未创建或删除任何槽位锁。
- 恢复入口：等待调度者明确释放窗口，重新读取公共接线及生产身份；按capacity-policy核验资源/磁盘/镜像身份和容器依赖，实测受限QEMU与真实客户机SSH；实现四独立题及TDD错误控制，运行原定三个验证命令、8个标准子题oracle/nop和父题oracle/nop，归档隔离/资源/哈希收据；有代码后动态领取Cargo槽位执行Clippy并释放自己的锁。完整通过后才按编号提交自身资产、标已完成并删除本文件。禁止改宿主Docker/Rosetta/binfmt/全局平台设置或自动加入兼容shim；任务47的既有Rosetta失败不作为本题失败证据。当前仅任务状态/调查记录变更，无须提交，保留文件待恢复。


## 2026-10-28 宿主独立资产续接（覆盖前述“无本题代码”状态）

- 按调度者续接要求，已在本题目录补齐case.json、checker、四个完整独立题面/合成上下文、四个gold、每题三个独立错误候选、20个gold/nop/负例控制、宿主聚焦测试、README及合成客户机Dockerfile/build_guest.sh配方。没有Docker操作、镜像下载/导入、真实父题、付费调用、公共接线或冻结身份变更。所有实际改动仅本题目录及本任务文件。
- m1实际qemu-img解析变化的QCOW2并拒绝raw/非loopback/非法端口；m2实际TCG客户机串口随机命令、内核/marker/PID检查；m3真实客户机echo经hostfwd完成两次随机TCP往返；m4独立客户机SSH正确/错误认证、断开服务和两次连接检查，额外通过客户机Dropbear认证日志计数拒绝“只连一次却自述两次”。nop为标准空候选映射，不引入伪造服务。m2–m4配方使用公开父题kernel及合成BusyBox/Dropbear用户空间，不代替完整Alpine原题。
- TDD首次缺checker：26测试，4 failures/3 errors，20真实控制跳过，退出1（host-red.log）；实现后最终原定宿主unittest命令退出0：29测试、9有效宿主检查通过、20真实客户机控制明确跳过（host-green.log）。9项涵盖资产格式、16程序语法、隐藏材料隔离、缺答案拒绝/基础设施错误、真实候选的危险参数拒绝、真实shell命令及TCP探针机制；宿主TCP服务仅用于探针单元测试，绝不算客户机证明。独立scratch副本将checker替换为全接受版本，29测试中5 failures/2 errors/20跳过、退出1（checker-mutation-red.log），正式checker未被替换。
- 静态 `PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/qemu-alpine-ssh/case.json` 退出0，checks=[]；`sh -n evals/terminal-bench/suite/cases/qemu-alpine-ssh/environment/build_guest.sh` 退出0；`evals/promptfoo/score.sh test` 退出0。没有运行--run-fixtures或preflight，没有生成validation.json，静态通过不等于校准通过。
- 动态原子领取Cargo共享槽位1，CARGO_TARGET_DIR与CARGO_BUILD_BUILD_DIR均为 `/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1`，CARGO_BUILD_JOBS=1；`cargo clippy --all-targets --all-features -- -D warnings` 退出0，有效行为测试0，仅既有block未来兼容提示。Cargo进程结束后已释放自己的owner记录/锁，缓存保留；无Rust修改，无fmt、全量cargo test、wasm或Makefile命令。
- 精确阻塞仍为任务13独占容器窗口，且宿主无qemu-system-x86_64/qemu-img/sshpass与客户机boot材料。case.json临时引用既有工具基底digest，仅用于静态资产结构，README明确未准入、无QEMU能力证明；Dockerfile要求显式传入已审查的PARENT_IMAGE不可变digest，不自动构建。不得把临时基底或未执行配方作为成品镜像身份。应在窗口释放后按授权标准builder生成并实测合成客户机镜像、更新为真实digest、核查继承材料隔离，执行20个真实控制、8个标准子题oracle/nop及原题preflight，归档digest/哈希/网络/资源/隔离收据。未完整校准，继续已阻塞而非待回归。
- 原始证据及所有本题文件SHA256：忽略目录 `evals/promptfoo/results/qwen38-suite/diagnostics/qemu-alpine-ssh/host-evidence.json`、host-red.log、host-green.log、checker-mutation-red.log、static.log、score.log、clippy.log、cargo-slot.json。保留本题资产及任务文件供原会话容器续接，当前不提交未经真实校准的资产；完整通过后按任务编号串行使用git-commit。plan.md未修改。
