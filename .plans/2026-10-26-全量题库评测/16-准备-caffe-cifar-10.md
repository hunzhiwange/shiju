# 任务 16: 准备-caffe-cifar-10

批次：【批次 9】；依赖批次：1、2、3、4、5。同批无文件依赖；Git 提交按任务编号串行，禁止删除其他会话的索引锁。

状态：已阻塞

目的：为“图像分类训练”准备原题完整环境与四道独立诊断题。

来源任务：Terminal-Bench2.1/caffe-cifar-10

预计会话范围：只准备一个父题的四个窄诊断题和确定性校准；不运行付费模型。长镜像拉取允许记录进程后续接，不能把未校准标成完成。

## 文件

- 新建：`evals/terminal-bench/suite/cases/caffe-cifar-10/case.json`、`evals/terminal-bench/suite/cases/caffe-cifar-10/check.py`、`evals/terminal-bench/suite/cases/caffe-cifar-10/tests/test_cases.py`、`evals/terminal-bench/suite/cases/caffe-cifar-10/fixtures/`、`evals/terminal-bench/suite/cases/caffe-cifar-10/oracle/`、`evals/terminal-bench/suite/cases/caffe-cifar-10/environment/Dockerfile`、`evals/terminal-bench/suite/cases/caffe-cifar-10/README.md`
- 只读完整上游包：`evals/promptfoo/results/qwen38-suite/source/tasks/caffe-cifar-10/`
- 忽略产物：`evals/promptfoo/results/qwen38-suite/preflight/caffe-cifar-10/`、`evals/promptfoo/results/qwen38-suite/diagnostics/caffe-cifar-10/`

## 上下文

原题 caffe-cifar-10；类别 machine-learning；CPU=4、内存=8192MiB、存储=10240MiB、GPU=0；Agent=3600秒、verifier=1200秒。预建镜像 alexgshaw/caffe-cifar-10:20260403。原题树SHA=89d331dde5215ea49742d9824bf691671093d6d6；题面SHA256=46907b0ee048f74269c29a15e0e086546b2b1220149597017da8b5ccc05a0cea；配置SHA256=2831165ad9d9e8bbc646e6d85c0a2290640dcc4f13749547a2937c7675012a8b。读完整 instruction.md、输入、solution、tests 后设计，禁止改原题。四子题：m1 核对分类数据与网络维度；m2 运行小型前向和训练步骤；m3 固定训练次数与随机条件；m4 保存并验证模型权重。

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

先使用技能 ./skills/create-agent-evals/SKILL.md；每个子题定义合成输入、清楚完整题面、输出路径/API/服务行为与独立确定性判定。新增 evals/terminal-bench/suite/cases/caffe-cifar-10/tests/test_cases.py：正解通过，至少3个有意义错误变体被拒绝；运行真实程序/产物，不验证关键词、自述或仅Agent自测。错误fixture和漏检checker先失败，再最小修复。

验证命令：

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/caffe-cifar-10/tests -p test_cases.py
```

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/caffe-cifar-10/case.json --run-fixtures
```

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.run preflight --catalog evals/terminal-bench/suite/catalog.json --runtime evals/promptfoo/results/qwen38-suite/runtime.json --parent-id caffe-cifar-10 --output evals/promptfoo/results/qwen38-suite/preflight/caffe-cifar-10
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


## 本次执行证据（2026-10-27）

- 已阅读完整上游 instruction、README、Dockerfile、task.toml、solution/solve.sh、tests/test.sh、tests/test_outputs.py；未修改任何上游文件或 plan.md。
- 依赖批次 1–5 已核对提交与收据：catalog 对固定 revision 重建后 89 题完全一致；capacity.json、contract-validation.json、agent-validation.json、generation-2 runtime 校准和分层报告存在，冻结运行时及 Harbor 0.24.0 身份校验通过。结果见忽略目录 `evals/promptfoo/results/qwen38-suite/diagnostics/caffe-cifar-10/dependencies.json`。
- 原镜像 `sha256:929a6d631b592c82e0d0f5d4a65cf03c946dc554cfc9bb09916e82076e7d3215` 实际 linux/amd64；独占准备资源，官方容器 nano_cpus=4000000000、memory=8589934592，无宿主挂载。宿主空闲约 111GiB，容器文件系统空闲约 91GiB；未修改官方存储和超时配置。
- 容器内 GitHub、CIFAR、uv 0.9.5、PyPI 端点 HEAD 探测均为 200（探测命令退出 0）。但原参考解实际 apt 安装 BLAS/OpenCV/protobuf 等包时出现远端关闭连接、连接失败和超时；首个核心依赖安装失败，后续脚本仍继续。原始日志明确记录 `make: protoc: No such file or directory`、`make: *** [Makefile:649: .build_release/src/caffe/proto/caffe.pb.cc] Error 127`；dpkg-query 确认 protobuf-compiler、libprotobuf-dev、libopencv-dev 均未安装。
- 原样启动命令：`PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python evals/promptfoo/results/qwen38-suite/diagnostics/caffe-cifar-10/calibrate-parent.py`。该脚本调用 suite.scoring.harbor_trial 执行完整官方参考解。构建失败后上游仍继续下载 CIFAR（约 50KiB/s，剩余约 52 分钟）；为避免无效等待，归档日志后向本会话 PID 20416 发送 SIGINT，进程退出 130。未取得 oracle reward，不能将中断算为有效失败评分。
- 进一步读取同一失败 libblas3 包，直连及 Docker 已配置代理 HEAD 均返回 200、curl 退出 0；这不能证明大批包传输可靠。任务限定必要依赖修复为 verifier-only，当前失败发生在原题 solution 安装/构建阶段，不能通过改参考解、替换原镜像或预装依赖静默绕过；没有执行这些改动。
- 按任务给定完整 preflight 命令执行，退出 1，`KeyError: historical_root`：原 runtime.json 是历史证据报告。已在本题忽略目录建立 runtime.json，引用既有冻结 runtime-v2、原始 source、cases 根目录并保留 agentTimeoutSec=3600；checked_runtime 校验 generation=2 通过。此配置修正没有解决原题构建失败，也不计为校准完成。
- 未新增 case/checker/fixtures：原题校准未通过，不能宣称本题就绪。聚焦 unittest、diagnostics validate、四子题 oracle/nop/三负例及原题 nop 均尚未运行；没有有效行为测试或官方评分。没有付费模型请求。
- 原始日志、镜像/资源收据、失败包探测、命令和 SHA256 汇总均保存在 `evals/promptfoo/results/qwen38-suite/diagnostics/caffe-cifar-10/`，入口 `blocker.json`；原 oracle 日志为 `original-oracle.txt`。
- 清理：已归档 inspect、资源和完整 solver 日志，仅删除本会话容器 `11a2f6778939d4979505e1984ee556a0d9a7c1faa8754e15a6a0e30e4e531538`，已确认 runner 结束；保留原镜像和其他会话资源。未运行 Cargo，实际槽位/CARGO_TARGET_DIR 均为未使用，无锁需释放。仅更新本编号任务状态与阻塞记录，无代码变更、无提交。
- 恢复入口：修复容器到 Ubuntu 包站点的大批包传输可靠性后，在相同原镜像、官方资源和全新环境下重跑原题 oracle/nop；原题 oracle 通过后再实施四个独立诊断题及 TDD/校准、Clippy。不得将这次未评分中断计入模型成绩；本文件保留。


## 有界阻塞恢复复查（2026-10-27）

- 未重复训练或安装链路，未触碰任务13及其文件。复查旧 runner PID 20416 不存在，无本题原8GiB容器；原镜像、capacity-image.tar 及已有证据缓存均保留。原容器的临时 writable layer 已随先前明确删除的容器释放，不声称保留其未安装完的APT临时缓存。
- 当前仅有既有业务容器时，以同一官方 digest 启动一次独立只读探测容器 `vw-caffe-task16-recovery-probe`（0.5CPU/128MiB、无挂载、16MiB tmpfs、cap-drop ALL）；仅运行 curl GET、sha256sum、dpkg-deb 元数据读取，未运行 apt/dpkg 安装、未注入运行依赖。每请求连接上限4秒、总上限12秒，2个失败包各直连/既有Docker代理一次，共4次，无重试。
- libblas3 3.12.0-3build1.1 完整获取237978字节，直连6.56秒/代理4.24秒，SHA256均为 `1b0ef0ad0893ff77bf7d5ab75723771788e16137ea1efde557dbac7b7b83a758`；libopencv-highgui-dev 4.6.0+dfsg-13.1ubuntu1 完整获取176184字节，直连3.16秒/代理2.82秒，SHA256均为 `0a529ba4dc08ed9bc46c595bf7db3e7dbb0af2d36edda0c17d11d07f0ec8eb6b`。4次curl均HTTP200、退出0，4次包元数据读取均退出0；探测容器总退出0。这比先前HEAD证据更强，支持原故障为瞬时传输边界；未额外获取APT索引进行独立签名链校验，不夸大为新认证收据。
- 当前授权内的安全恢复入口：网络可靠性恢复后，使用不变的官方镜像/参考解和全新官方资源环境重新校准；两条现有网络路径均可获取相同文件，无证据要求兼容环境变体或候选可见依赖预装。单包恢复不能证明数百包连续传输、CIFAR大文件、编译、500步训练能够在原3600秒agent/1200秒verifier限制内完成，因此本次仍保持已阻塞，不无意义重跑。
- 必要外部条件：容器至原Ubuntu包站点及CIFAR站点连续完整下载稳定且吞吐足以在原超时内完成全链路。最小后续决策：在该网络条件满足并重新取得独占资源后，安排一次不变环境校准；若要显式改变执行前全局代理/环境网络条件或预装依赖，则需另行授权并单列环境身份，不将其冒充verifier-only修复。任务13的兼容变体授权不适用于本题。
- 原始证据 `diagnostics/caffe-cifar-10/recovery-get-probe.log` 和 `bounded-recovery.json` 位于既有忽略结果根下。探测容器已自动删除，最终无本题容器/进程，8GiB独占资源已释放。未运行Cargo（槽位及CARGO_TARGET_DIR均未使用），无付费调用，无代码变更，无提交；保留本任务文件。
