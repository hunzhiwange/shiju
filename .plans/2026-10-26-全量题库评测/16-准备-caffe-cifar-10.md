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


## 恢复复查（2026-10-28，当前模型）

- 重读任务/只读plan.md及前两次阻塞收据，接受前置批次实现可继续、任务7待回归不阻止本题；未将任务12的平台阻塞用于阻止本题。核对本题上游全部catalog文件SHA256仍一致。
- 容量/环境：原linux/amd64镜像digest仍可inspect；宿主检查时可用25631400KiB（24.44GiB），Docker文件系统可用77895460KiB（74.29GiB），保守最小值超过原单题19.08GiB准入估算。Docker仍为4CPU/12530401280字节。只有9个既有业务容器，无本题遗留8GiB容器；未启动训练。现有镜像清单没有已准备好的Caffe环境，results文件名检索无完整cifar-10-binary.tar.gz、cifar-10-batches-bin、caffe.bin或caffemodel缓存。其它题诊断/原生架构环境不可替代冻结原题镜像。
- 以同一官方digest串行运行两个只读网络探测容器（各0.5CPU/128MiB、tmpfs4MiB、无挂载、无安装、cap-drop ALL），逐请求连接上限4秒、总15秒、请求最多1MiB，未重试历史完整训练。Ubuntu原失败libblas3包读取237978字节/1.68秒、退出0；GitHub读取311726字节/1.61秒、退出0。CIFAR三条当前路径均HTTP206但读取超时：直连原HTTPS URL收到768000字节/15.10秒（50860B/s），既有Docker代理读取原参考解HTTP URL收到904000字节/15.02秒（60186B/s），直连原链接实际跳转终点收到936000字节/15.01秒（62346B/s）；三个curl均退出28。仅证明当前有数据流，不将部分响应算完整下载。
- 当前网络尚不满足capacity-policy.md“探测失败记录为基础设施失败并阻止依赖该端点的正式运行”准入。170052171字节原数据按短样本线性估算仅下载约45–56分钟，且不含安装/编译/训练；这是风险估算，不能断言未来必然超时或冒充实测完整运行。因此没有无意义重跑已知训练链路。
- 授权内替代已核对：原镜像缓存可用但无可复用训练/数据产物；同一原数据端点的直连、既有代理、真实重定向终点均未通过有界读取。未改APT源、参考解、模型、预算、官方超时、checker或镜像，未预装候选可见依赖，也未把构建期修复称为verifier-only。
- 最小解除条件：原CIFAR端点的现有网络路径恢复到可靠完整传输，并有条件在不变3600秒agent/1200秒verifier内完成原任务；之后重做有界准入，再独占运行一次新鲜原环境校准。如果只能使用新的外部网络/缓存条件、候选可见预装环境或兼容变体，需要明确单列该环境的授权与身份，不能自动借用任务13授权。本轮无需改变评分、模型或预算来解除阻塞。
- 新原始日志和SHA256收据：忽略目录 `evals/promptfoo/results/qwen38-suite/diagnostics/caffe-cifar-10/recovery-20261028/{network.log,alternatives.log,receipt.json}`。两个探测容器外层进程退出0（内部三个curl失败已分别记录）；容器均--rm自动删除并再次确认无本题容器。保留原镜像/OCI缓存和既有证据。仅更新本任务阻塞记录，无代码/测试新资产、无提交、无Cargo命令（槽位/CARGO_TARGET_DIR未使用）、无付费模型调用；仍保留已阻塞，原题与四子题有效评分均未获得。


## 最终回归前单次有界复核（2026-10-28）

- 接受其它题完成/Ubuntu等站点改善信息，但只以CIFAR自身当前证据判定本题恢复。宿主可用61060984KiB（58.23GiB），Docker可用76806896KiB（73.25GiB），均超过本题保守19.08GiB容量门槛；启动前只有9个既有业务容器。
- 原镜像不变，以0.5CPU/128MiB只读临时容器 `vw-caffe-task16-final-probe` 对参考解原HTTP链接仅做直连/既有代理各一次GET，Range=0–4194303、连接上限4秒、请求上限20秒，不重试、不安装依赖。两者均跳转原 `https://cave.cs.toronto.edu/kriz/cifar-10-binary.tar.gz`，HTTP206；直连仅856000字节/20.097秒（42593B/s），代理仅840000字节/20.075秒（41842B/s），两个curl均退出28。外层脚本退出0不代表下载成功。
- CIFAR独立瓶颈未实质改善，当前短样本约42KB/s；170052171字节若线性外推仅数据传输约66.5–67.7分钟，已长于3600秒agent限额且未计安装/编译/训练。这是样本外推，不冒充完整下载实测或未来性能保证；更直接阻塞依据仍是capacity-policy.md的失败端点准入门槛。
- 精确最小外部条件：同一原CIFAR端点通过现有授权网络路径恢复可靠完整下载，吞吐应足以把下载、安装、构建和500步训练共同容纳在原3600秒agent限额内；其它站点改善不能代替该条件。若只有更换网络/增加缓存变体能解决，需要单独明确授权及身份，不能预装候选依赖或延长超时替代。条件未满足，保持已阻塞，不循环探测、不再重跑原训练。
- 新证据：忽略目录 `evals/promptfoo/results/qwen38-suite/diagnostics/caffe-cifar-10/final-recovery-20261028/{probe.log,receipt.json}`。容器--rm已删除，最终docker过滤确认无本题容器，原镜像与缓存保留。未修改plan.md/原题/Agent环境/评分/模型/预算；无付费调用、无Cargo命令（槽位/CARGO_TARGET_DIR均未使用）、无代码资产或提交，仅追加本编号任务阻塞证据并保留文件。本轮结束。


## 深入恢复诊断与可审查兼容方案（2026-10-29）

- 按最新授权扩大到宿主/系统既有代理/DNS/路由/协议/并行与缓存排查；随后遵守协调范围只做下载和材料，不启动任何构建、训练、候选代理变更或校准。宿主起始60963964KiB可用，容量足够。读取现有合法配置仅保留无凭证端点：系统HTTP/SOCKS代理127.0.0.1:7897，DNS114.114.114.114；未打印令牌、私密载荷或代理节点凭据。未改变全局设置。
- 宿主直连、既有系统代理、公共DNS解析所得真实原站IP128.100.3.67（保留原域名TLS校验）、HTTP/2协商、IPv6请求均约49–52KB/s。服务器实际协商HTTP/1.1；无原生AAAA证据，IPv6为IPv4映射；真实/映射地址仍经现有utun1024路由，不能断言已绕过该网络层。完整GET不带Range也为HTTP200/600000字节/12.01秒，故慢速不是Docker专属或Range专属。未通过改全局路由/DNS/代理组去“突破”。
- 四个并行256KiB请求6.63秒完成1MiB，总吞吐158066B/s；全部精确匹配Content-Range、长度，总长度一致170052171。六种单连接/协议/完整GET路径的已收前缀与拼接结果逐字节一致。证据支持每连接瓶颈，但不足以区分原站和上游路由整形；不把推测写成确定限速策略。
- 扩大检索现有/临时/下载/用户缓存后未找到旧完整原始包。随后仅4个固定非重叠Range连接下载同一原跳转端点，各1200秒上限、If-Range固定原ETag、无自动重试。全包已在702.41秒完整获取：170052171字节；四段范围/长度全部正确，官网下载页公布MD5 `c32a1d4ab5d03f1284b67883e8d87530` 完全一致；新SHA256 `c4a38c50a1bc5f3a1c5537f2155ab9d68f9f25eb1ed8d9ddda3db29a59bca1dd`。归档成员路径/长度/SHA256亦已读取并记录，未把数据注入候选容器。未绕过认证/访问控制，未购买服务，未增加连接数。
- 审查材料位于忽略目录 `evals/promptfoo/results/qwen38-suite/diagnostics/caffe-cifar-10/deep-recovery-20261029/`，入口 `compatibility-proposal.md`、`review-receipt.json`；完整原包 `cifar-10-binary.tar.gz`、官网页、四段字节和 `verified-original.json` 保留。方案保持镜像/原题命令/依赖安装/数据字节/500步训练/评分/资源/时限/模型/预算，唯一变更拟为候选可见任务级临时http_proxy，经任务自有loopback缓存仅加速原HTTP下载URL，非目标与Range流量交既有代理。该代理环境变量、原URL直接HTTP200代替301跳转和网络速度是显式可见差异；必须单列兼容身份，不称为verifier-only修复或原样就绪。
- 已完成仅宿主的可回滚代理原型验证：完整170052171字节HTTP200传输0.092987秒，接收MD5/SHA256一致；非目标TLS CONNECT+官方跳转HTML通过，非目标HTTP原libblas3包237978字节/原SHA256通过，三个curl退出0。此速度仅localhost，不冒充原站、Docker或训练速度。首次TLS页校验因测试未跟随官网301失败，原始收据保留；补--location后通过，未篡改响应。代理原型和测试脚本仅忽略材料，无产品代码变更。
- 所有自有下载/代理进程已结束（最后proxy PID 60321已停止），无本题容器；未占用构建/校准容器资源，不触碰任务26。原镜像/缓存及完整下载保留，移除的仅宿主测试生成且已验证为同字节的重复归档。再次核对系统代理选择项与原题全部catalog文件未变。
- 唯一待决策：是否明确接受上述“候选可见临时http_proxy+原字节缓存”的独立网络兼容变体，再安排fresh候选连通性/父题oracle+nop/四诊断校准；未经该决策不注入候选代理，不启动兼容训练。若坚持完全原样网络，则仍需现有路径恢复可靠足够吞吐。本轮按最新明确范围完成归档与审查材料，原样任务继续已阻塞；下载/宿主测试不是有效原题或诊断评分。无付费调用，无Cargo命令（槽位/CARGO_TARGET_DIR未使用），仅编号任务记录变更无需提交；plan.md只读。


## 独立Docker网络控制（2026-10-29，兼容决策仍待授权）

- 按新增明确范围只运行一个无候选/无原题程序的网络控制。沿原镜像创建只读容器 `b0a63bffec27a0533b7c81cfae7065a524729e0012472255362e83554ca39f20`，0.5CPU/256MiB、64PID、cap-drop ALL、192MiB tmpfs、无宿主挂载，未安装依赖。与任务26原型合计不超过当前资源预算，未触碰其容器。
- 任务缓存仍仅监听宿主127.0.0.1，容器以host.docker.internal访问成功；没有扩大到公网/全网卡/Docker专用监听，没有更改全局代理/DNS/路由/防火墙。每次仅curl参数指定代理，未设置候选或容器proxy环境变量，不改正式执行配置。
- 原HTTP下载URL返回HTTP200，170052171字节传输0.574821秒（295835000B/s）；容器内wc、md5sum、sha256sum全部独立匹配官网MD5与本地原包SHA256。非目标HTTPS使用正常TLS和原301跳转，HTTP200/20260字节/2.562980秒，官方摘要条目匹配。控制容器实际退出0，控制总4.73秒；这是独立网络与数据控制，不能算完整题/候选/训练校准或原样就绪。
- 缓存代理PID65675已结束、临时端口51785不再使用；控制容器inspect资源后仅删除该精确ID，删除退出0，再次确认无本题网络控制容器。系统代理选择项与先前记录一致，原归档/分段缓存保留。原始日志、资源、校验、清理记录见既有深度恢复目录 `docker-network-control.log`/`docker-network-control.json`；`compatibility-proposal.md`和`review-receipt.json`已更新。
- 方案现已有完整原数据等价、宿主及Docker到loopback缓存加速、正常TLS转发证据；仍等待明确决定是否接受候选可见临时http_proxy兼容变体。未获授权前不启动兼容训练/正式候选校准；原样任务保持已阻塞，无付费/Cargo调用，无提交。
