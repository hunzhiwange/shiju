# 任务 36: 准备-filter-js-from-html

批次：【批次 20】；依赖批次：1、2、3、4、5。同批无文件依赖；Git 提交按任务编号串行，禁止删除其他会话的索引锁。

状态：已阻塞

目的：为“安全页面清洗”准备原题完整环境与四道独立诊断题。

来源任务：Terminal-Bench2.1/filter-js-from-html

预计会话范围：只准备一个父题的四个窄诊断题和确定性校准；不运行付费模型。长镜像拉取允许记录进程后续接，不能把未校准标成完成。

## 文件

- 新建：`evals/terminal-bench/suite/cases/filter-js-from-html/case.json`、`evals/terminal-bench/suite/cases/filter-js-from-html/check.py`、`evals/terminal-bench/suite/cases/filter-js-from-html/tests/test_cases.py`、`evals/terminal-bench/suite/cases/filter-js-from-html/fixtures/`、`evals/terminal-bench/suite/cases/filter-js-from-html/oracle/`、`evals/terminal-bench/suite/cases/filter-js-from-html/environment/Dockerfile`、`evals/terminal-bench/suite/cases/filter-js-from-html/README.md`
- 只读完整上游包：`evals/promptfoo/results/qwen38-suite/source/tasks/filter-js-from-html/`
- 忽略产物：`evals/promptfoo/results/qwen38-suite/preflight/filter-js-from-html/`、`evals/promptfoo/results/qwen38-suite/diagnostics/filter-js-from-html/`

## 上下文

原题 filter-js-from-html；类别 security；CPU=1、内存=8192MiB、存储=10240MiB、GPU=0；Agent=1800秒、verifier=1800秒。预建镜像 alexgshaw/filter-js-from-html:20251031。原题树SHA=7939565b055a4b4fa4d87ddc76b4250f03810362；题面SHA256=48d6ac95e8ccb552195ce9fbdea86ba8d5d7ccf54e159c31590797802c5cf171；配置SHA256=f7f888951c97b8120371139bb78d39499c2d3cd7aa3836d57fd6d8b9e4199344。读完整 instruction.md、输入、solution、tests 后设计，禁止改原题。四子题：m1 解析合成页面结构；m2 移除危险节点和属性；m3 处理混淆输入并保留正常内容；m4 验证清洗器文件接口。

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

先使用技能 ./skills/create-agent-evals/SKILL.md；每个子题定义合成输入、清楚完整题面、输出路径/API/服务行为与独立确定性判定。新增 evals/terminal-bench/suite/cases/filter-js-from-html/tests/test_cases.py：正解通过，至少3个有意义错误变体被拒绝；运行真实程序/产物，不验证关键词、自述或仅Agent自测。错误fixture和漏检checker先失败，再最小修复。

验证命令：

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/filter-js-from-html/tests -p test_cases.py
```

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/filter-js-from-html/case.json --run-fixtures
```

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.run preflight --catalog evals/terminal-bench/suite/catalog.json --runtime evals/promptfoo/results/qwen38-suite/runtime.json --parent-id filter-js-from-html --output evals/promptfoo/results/qwen38-suite/preflight/filter-js-from-html
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


## 2026-10-28 阻塞交接

- 批次 1–5 已由调度主会话确认完成；只处理本题目录，未修改 plan.md、官方原题、公共实现、评分、模型、预算、时限或资源。
- 已准备 m1–m4 合成输入、独立题面、真实程序 checker、gold 和每题 3 个错误实现。符号链接替换漏检的回归先失败，再修复；证据：diagnostics/filter-js-from-html/checker-red.log。
- 指定 unittest 命令退出 0，25 项测试通过；指定 diagnostics validate --run-fixtures 命令退出 0，20 次 fresh、offline、1CPU/512MiB 容器检查通过（4 gold、4 nop、12 负例）。完整 Harbor 子题校准尚未到达，不能将离线 fixtures 冒充整套准入。
- evals/promptfoo/score.sh test 退出 0（配置校验）。cargo clippy --all-targets --all-features -- -D warnings 退出 0；动态领取槽位 1，CARGO_TARGET_DIR=/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1，CARGO_BUILD_BUILD_DIR 同路径，CARGO_BUILD_JOBS=1。自有锁已释放、缓存保留；Rust 行为测试数 0，无 Rust 变更。既有 block 0.1.6 future-compatibility 提示未改。
- 原镜像 Registry digest：sha256:92acda0f124b988036a6f426ce0bc47fac19f5efe9fc5e6ea3ea52ccb075d0a4；7 层压缩 SHA256、解压 diff_id 和本地 RootFS 全部核验。限制探针确认 1CPU、8589934592 字节、/app 无隐藏评分材料；Chromium 与 ChromeDriver 均为 140.0.7339.127，实际触发 alert 的探针通过；上游样本 HTTPS 返回 200，417 个下载样本加内置向量组成 28 批。
- 两轮原题 oracle 环境均出现被官方 helper 吞掉的真实浏览器异常；不是正常无 alert 的 Selenium TimeoutException。首轮有驱动/加载 Read timed out；可追溯 verifier-only 修复缓存依赖和 Selenium offline 解析后 fresh 重跑，仍有全局闹钟 Test timed out、页面加载 Read timed out、tab crashed。第二轮有 26 批开始、19 次正常无 alert 检查、6 条明确异常，最后一批尚未完成。曾观察到 147 个 chromium 进程、约 6.7GiB 内存；不擅自修改上游清理逻辑或扩大资源/时限。
- 确认真实基础设施失败后主动终止本题原题 verifier，保留两轮原始日志/配置/异常和评分：最终 score_status=unscored、reward=null、RewardFileNotFoundError，preflight 退出 1。没有有效原题 oracle/nop 校准；不能把异常被吞后的潜在 reward=1 当成功。无付费模型调用。
- 实际 preflight 命令：PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.run preflight --catalog evals/terminal-bench/suite/catalog.json --runtime evals/promptfoo/results/qwen38-suite/diagnostics/filter-js-from-html/runtime.json --parent-id filter-js-from-html --output evals/promptfoo/results/qwen38-suite/preflight/filter-js-from-html/harbor。使用获授权的 runtime-v2 身份与任务局部 verifier 依赖配置。
- 完整忽略证据：evals/promptfoo/results/qwen38-suite/diagnostics/filter-js-from-html/blocked-evidence.json、browser-offline-infra-captured.log、browser-memory-observation.jsonl、image-provenance.json、parent-probe.json、runtime.json；两轮归档在 preflight/filter-js-from-html/harbor-browser-infra-failure 和 harbor-browser-offline-infra-failure。不存在 case/preparation.json，防止 freeze 将本题视为就绪。
- 自有下载/preflight 进程及容器已清理，无活跃本题资源占用；本题资产保留未提交，编号任务文件保留已阻塞。恢复入口：在保持原 1CPU/8192MiB 和 1800 秒限制的环境排查浏览器执行/进程清理，优先排除本机 amd64/Rosetta 因素；只能做可追溯 verifier 依赖修复，再 fresh 跑原题 oracle/nop、4 子题及全部证据。若需改官方评分/浏览器清理逻辑，必须另获明确授权，不能在本任务绕过。


## 2026-10-28 本轮恢复条件复核

- 状态仍为已阻塞；本轮仅更新本编号任务文件，不重新运行已证明失败的长浏览器校准，不修改或提交源码、题面、官方评分/helper、资源、超时、模型、预算或 Agent 环境；plan.md 只读。
- 调度确认批次 1–5 实现依赖满足，其中任务 7 待回归；22/30/31 已恢复完成。上述恢复与网络改善不等于本题真实浏览器校准恢复。
- 本轮实际命令及结果：`df -k .` 退出 0，宿主 Available=61092776KiB（约58.3GiB）。`docker info` 退出 0，Architecture=aarch64、NCPU=4、MemTotal=12530401280；`uname -m` 为 arm64。`docker context ls` 退出 0，仅 default 与 desktop-linux 两个本地 Unix socket；`docker buildx ls` 退出 0，两者均使用本机 docker driver。列出 linux/amd64 平台不等于存在原生 amd64 执行端。未新启动容器，未把本轮宿主容量检查冒充 VM 空间复测或8GiB准入。
- 依据现有只读归档重算 SHA256：完整 OCI 的9个blob加index/layout共11项一致，config架构为amd64；任务局部runtime声明的5个verifier依赖文件哈希一致；case/validation.json绑定的31项诊断字节哈希一致，20条旧fixture收据保留且明确不是本轮新校准；官方 instruction.md 与 task.toml 哈希仍符合冻结值。没有 case/preparation.json，保持不能freeze为就绪。此次仅做文件身份核验，不运行代码/浏览器测试、Cargo或模型。
- 重新核对官方 helper：正常无alert的 Selenium TimeoutException 会输出 `No alert detected`；旧证据中的全局闹钟 `Error checking for alert: Test timed out`、页面加载 Read timed out 与 tab crashed 属于真实执行异常，不能当正常无alert。原 helper 在finally中先kill驱动进程再quit，旧环境有大量遗留Chromium进程；具体因果未在本轮重试验证，不能通过注入reaper、monkeypatch、替换清理逻辑或放宽判定绕过。
- 安全恢复方案边界：原镜像可由保留OCI及maintenance-20261028/restore.py精确离线恢复，但导入同一amd64镜像不会改变当前ARM64/Rosetta执行条件；缓存依赖与Selenium离线驱动解析此前已经fresh验证，仍失败，没有新的已验证依赖修复证据。默认/desktop-linux不是两个独立架构执行端；未重试调度已知SSH握手关闭的dslr1，不绕过平台拒绝。没有为猜测性的浏览器/驱动升级、模拟器切换或替代执行服务修改环境。
- 最小推进条件：提供可访问、获授权的原生linux/amd64执行端（或提供不改官方helper/Agent环境且已有证据针对当前异常的verifier-only依赖修复）。原生端只是允许重新检查的条件，不预先保证解决遗留进程问题；需在原digest、1CPU/8192MiB与原1800秒限制下证明浏览器真实执行，再fresh完成原题oracle/nop和四子题校准，逐批核验真实异常而非只看reward。若必须修正官方helper清理/异常处理，则需要另行明确的评分变更决策，不属于本任务现有权限。
- 当前没有本题下载、导入、Cargo、浏览器或校准进程/容器。未占用共享Cargo槽位、未恢复已卸载镜像、未重新生成重复tar/parts，原始失败和全部必要依赖继续保留。无需提交，编号任务文件保留已阻塞。


## 2026-10-29 调查与有界生命周期控制交接（仍已阻塞）

### 只读根因调查

- 已读取原helper、既有真实失败/内存/进程证据及实际评分所用Selenium4.38.0 wheel源码，未修改官方helper或评分。原helper SHA256为6b43e99fb1c47792a39307fe90df91adfde4a588497c08f3b7da7b3265ef2733。
- 确认调用链：helper先kill ChromeDriver，再调用需要通过其HTTP发送Command.QUIT的driver.quit；Selenium ChromiumDriver.quit会吞掉super.quit异常，Service.stop在驱动已死时不会正常远程shutdown。因此“Driver closed”不能证明Chrome已退出。较晚旧快照实际有168个chromium、52个crashpad、25个被PID1接管的存活Chromium根，无Z状态；早期147个chromium是另一快照，不混为同一时点。
- 旧持久Docker stats为7.348GiB/8GiB、7.984GiB/8GiB；1529/1740是PIDs线程/任务统计，不等于进程数。旧26批中19次正常无alert、6条明确异常、最后一批未完成。全局闹钟、页面Read timed out、tab crashed不是正常3秒无alert等待；没有memory.events/内核证据证明旧崩溃一定由OOM导致。
- 从保留原OCI只读提取dpkg status，确认chromium/common/driver/sandbox完整Debian版本均为140.0.7339.127-1~deb12u1、架构amd64。原浏览器、驱动、crashpad及评分wheel内Linux Manager均为x86_64 ELF。没有在本阶段验证同版本ARM64签名包的可获得性；同源同版本不等于同字节或可比成绩。原生amd64也不会自动纠正helper退出顺序。
- 可审查只读报告：/Users/Shared/work/dir/data/codes/vibe-window/evals/promptfoo/results/qwen38-suite/diagnostics/filter-js-from-html/investigation-20261029/REVIEW.md
- 原始材料SHA、进程逐项分析、库源码片段索引：/Users/Shared/work/dir/data/codes/vibe-window/evals/promptfoo/results/qwen38-suite/diagnostics/filter-js-from-html/investigation-20261029/evidence-analysis.json
- 原包身份与二进制身份：/Users/Shared/work/dir/data/codes/vibe-window/evals/promptfoo/results/qwen38-suite/diagnostics/filter-js-from-html/investigation-20261029/original-browser-packages.json；/Users/Shared/work/dir/data/codes/vibe-window/evals/promptfoo/results/qwen38-suite/diagnostics/filter-js-from-html/investigation-20261029/original-browser-binaries.json；/Users/Shared/work/dir/data/codes/vibe-window/evals/promptfoo/results/qwen38-suite/diagnostics/filter-js-from-html/investigation-20261029/extracted-metadata.json。

### 两轮成功生命周期对照及一个发现器启动失败条件

- 全部为独立harness、本地无脚本空白HTML，不含上游攻击载荷、参考解、模型或完整原题校准。每容器至多1CPU/1GiB、每臂最多2会话、外部100秒界限；顺序运行，网络none、cap-drop ALL、no-new-privileges、只读rootfs及只读材料挂载。仅从已保留OCI离线恢复本题原镜像，无公网安装、构建或Cargo。
- 第一轮安全基线使用非root(uid65534)、启用浏览器安全检查，身份/flags不同于原helper，不能视为原样校准。kill-first两次后活跃进程11→22、内存498077696→901914624字节；正常quit组活跃进程0→0、僵尸6→12、内存94593024→94818304字节。两臂均加载空白页，分别约13.76/15.51秒。收据：/Users/Shared/work/dir/data/codes/vibe-window/evals/promptfoo/results/qwen38-suite/diagnostics/filter-js-from-html/lifecycle-control-20261029/kill-first-receipt.json；/Users/Shared/work/dir/data/codes/vibe-window/evals/promptfoo/results/qwen38-suite/diagnostics/filter-js-from-html/lifecycle-control-20261029/quit-first-receipt.json。
- 随后优先复用原root身份和冻结Chrome参数，但Manager发现路径以127退出并报NoSuchDriverException，浏览器尚未创建，立即停止该条件的候选臂。不能将其混作生命周期失败、内存不足或原样通过；底层127原因未证实。收据：/Users/Shared/work/dir/data/codes/vibe-window/evals/promptfoo/results/qwen38-suite/diagnostics/filter-js-from-html/lifecycle-control-20261029/faithful/kill-first-receipt.json。
- 第二轮主要对照使用原root身份、冻结的9项Chrome参数（仅沿用原既有参数，没有新增禁用项或放宽容器策略），通过Selenium Service显式指定原归档已核验的/usr/bin/chromedriver，避开发现器差异；两臂该入口和所有依赖相同。它与原helper默认Manager入口有明确差异，属于独立机制控制身份。
- 第二轮每次均实际完成空白文件URL加载、标题核验及3秒正常无alert等待。kill-first两次后活跃浏览器族进程10→20、存活孤儿Chromium根1→2、内存495898624→896897024字节；两次驱动均退出-9，完整driver日志无COMMAND/RESPONSE Quit。正常quit组两次后活跃浏览器/根均0、内存98271232→98639872字节；两次均有Quit请求和响应，驱动退出分别0/-15（后者为Service收尾），不是一律声称退出0。分别约19.48/20.46秒；memory.events的max/oom/oom_kill均0，没有闹钟、导航或tab crash异常。
- 正常quit组仍有9→18个僵尸，不能称所有进程归零。harness为PID1，而原环境为sh/uv祖先链，这也是明确差异，不能把该组僵尸直接归因原helper。测量期间没有外部reaper替原路径兜底。
- 第二轮收据（含完整driver日志、PID/PPID/状态/starttime、阶段计时、cgroup、实际HostConfig/State及清理退出码）：/Users/Shared/work/dir/data/codes/vibe-window/evals/promptfoo/results/qwen38-suite/diagnostics/filter-js-from-html/lifecycle-control-20261029/root-explicit-driver/kill-first-receipt.json；/Users/Shared/work/dir/data/codes/vibe-window/evals/promptfoo/results/qwen38-suite/diagnostics/filter-js-from-html/lifecycle-control-20261029/root-explicit-driver/quit-first-receipt.json。
- 汇总：/Users/Shared/work/dir/data/codes/vibe-window/evals/promptfoo/results/qwen38-suite/diagnostics/filter-js-from-html/lifecycle-control-20261029/control-analysis.json；完整结果与边界：/Users/Shared/work/dir/data/codes/vibe-window/evals/promptfoo/results/qwen38-suite/diagnostics/filter-js-from-html/lifecycle-control-20261029/RESULTS.md；本轮材料SHA清单：/Users/Shared/work/dir/data/codes/vibe-window/evals/promptfoo/results/qwen38-suite/diagnostics/filter-js-from-html/lifecycle-control-20261029/delivery.json。
- 因果结论限定：短无载荷同条件控制支持kill-before-quit导致活跃浏览器遗留；不能证明原题全部旧异常的原因，也不能证明A在原sh/uv链、初始化失败/坏会话、原8GiB及28批攻击负载下完整有效。没有产生原题oracle/nop或四子题Harbor新有效校准，原样保持已阻塞。

### 最小候选、明确待决策差异

- 候选A只删除原helper中“先强杀ChromeDriver”的注释及一行kill调用，保留正常driver.quit与Selenium Service.stop。可审查绝对路径：/Users/Shared/work/dir/data/codes/vibe-window/evals/promptfoo/results/qwen38-suite/diagnostics/filter-js-from-html/lifecycle-control-20261029/candidate-A-not-applied.patch（同内容原方案：/Users/Shared/work/dir/data/codes/vibe-window/evals/promptfoo/results/qwen38-suite/diagnostics/filter-js-from-html/investigation-20261029/A-quit-before-driver-stop.proposed.patch）。**未应用**，没有改真实评分路径。
- 用户需明确决定是否批准在单独compat-cleanup身份应用A，并先用原sh/uv祖先链和失败路径做有界无载荷补充；它改变官方helper执行行为，不能悄然视为原样修复或回填original成绩。当前并无完整原题/兼容评分重跑授权。
- 异常处理候选B是另一独立决策：将真实启动/导航/闹钟异常重新抛出，保持正常无alert TimeoutException分支。路径：/Users/Shared/work/dir/data/codes/vibe-window/evals/promptfoo/results/qwen38-suite/diagnostics/filter-js-from-html/investigation-20261029/B-propagate-browser-infra.proposed.patch。它明确改变原吞异常行为及可能reward，不能冒称评分不变；即便pytest写reward=0，也必须单列infra/unscored，不能当能力0。B亦未应用，不能与A捆绑默改。
- 同版本原生ARM64浏览器/driver替换、native verifier sidecar及产物/依赖映射仍只是方案，机器码、包SHA、运行根与可能的合法辅助产物均会变化，未证明可比，不与原样身份混用。

### 资源恢复、清理及当前占用

- 当时宿主可用59795832KiB；受限VM空间探针可用76401268KiB。离线精确重建的本题tar SHA仍为35ac7afbd4b299e5f7330e7be0a25bb4cccd45b5062426f0e0ae8be02913aabb，导入原manifest92acda0f124b988036a6f426ce0bc47fac19f5efe9fc5e6ea3ea52ccb075d0a4并核对完整RootFS。
- 各控制容器结束后立即移除；最终本题控制容器剩余0。仅在确认无容器、派生镜像或其他alias引用后，非force删除本轮恢复的本题parent镜像缓存；删除本轮重建的377999360字节重复tar。既有完整OCI、全部失败、依赖、调查和本轮收据保留，不删除其他题或业务材料。
- 精确清理收据：/Users/Shared/work/dir/data/codes/vibe-window/evals/promptfoo/results/qwen38-suite/diagnostics/filter-js-from-html/lifecycle-control-20261029/cleanup-receipt.json；空间探针：/Users/Shared/work/dir/data/codes/vibe-window/evals/promptfoo/results/qwen38-suite/diagnostics/filter-js-from-html/lifecycle-control-20261029/space-before.json；导入身份：/Users/Shared/work/dir/data/codes/vibe-window/evals/promptfoo/results/qwen38-suite/diagnostics/filter-js-from-html/lifecycle-control-20261029/image-identity.json。未将逻辑Size或其他题空间变化当确定释放量。
- 本轮无Cargo/编译/付费调用，未领取Rust槽位。自有实验资源已释放；仅本编号文件追加记录，无需提交。plan.md及官方helper保持不变，状态继续已阻塞。
