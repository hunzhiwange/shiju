# 任务 47: 准备-install-windows-3.11

批次：【批次 27】；依赖批次：1、2、3、4、5。同批无文件依赖；Git 提交按任务编号串行，禁止删除其他会话的索引锁。

状态：已阻塞

目的：为“旧系统虚拟机”准备原题完整环境与四道独立诊断题。

来源任务：Terminal-Bench2.1/install-windows-3.11

预计会话范围：只准备一个父题的四个窄诊断题和确定性校准；不运行付费模型。长镜像拉取允许记录进程后续接，不能把未校准标成完成。

## 文件

- 新建：`evals/terminal-bench/suite/cases/install-windows-3.11/case.json`、`evals/terminal-bench/suite/cases/install-windows-3.11/check.py`、`evals/terminal-bench/suite/cases/install-windows-3.11/tests/test_cases.py`、`evals/terminal-bench/suite/cases/install-windows-3.11/fixtures/`、`evals/terminal-bench/suite/cases/install-windows-3.11/oracle/`、`evals/terminal-bench/suite/cases/install-windows-3.11/environment/Dockerfile`、`evals/terminal-bench/suite/cases/install-windows-3.11/README.md`
- 只读完整上游包：`evals/promptfoo/results/qwen38-suite/source/tasks/install-windows-3.11/`
- 忽略产物：`evals/promptfoo/results/qwen38-suite/preflight/install-windows-3.11/`、`evals/promptfoo/results/qwen38-suite/diagnostics/install-windows-3.11/`

## 上下文

原题 install-windows-3.11；类别 system-administration；CPU=2、内存=4096MiB、存储=10240MiB、GPU=0；Agent=3600秒、verifier=3600秒。预建镜像 alexgshaw/install-windows-3.11:20251031。原题树SHA=b0e31189119e3aca8a9e0657080e783b07b81fdc；题面SHA256=6757fff2157378201478ef17ff3da4d4992aa586fa577dd518c63b98bb1945c5；配置SHA256=0485184bd16cc74883396115d93520bd25b36d23d61e248da88c0dceeab2eeac。读完整 instruction.md、输入、solution、tests 后设计，禁止改原题。四子题：m1 核对磁盘和启动参数；m2 启动最小虚拟机流程；m3 验证显示与连接设置；m4 检查服务持续运行。

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

先使用技能 ./skills/create-agent-evals/SKILL.md；每个子题定义合成输入、清楚完整题面、输出路径/API/服务行为与独立确定性判定。新增 evals/terminal-bench/suite/cases/install-windows-3.11/tests/test_cases.py：正解通过，至少3个有意义错误变体被拒绝；运行真实程序/产物，不验证关键词、自述或仅Agent自测。错误fixture和漏检checker先失败，再最小修复。

验证命令：

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/install-windows-3.11/tests -p test_cases.py
```

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/install-windows-3.11/case.json --run-fixtures
```

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.run preflight --catalog evals/terminal-bench/suite/catalog.json --runtime evals/promptfoo/results/qwen38-suite/runtime.json --parent-id install-windows-3.11 --output evals/promptfoo/results/qwen38-suite/preflight/install-windows-3.11
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

## 执行记录（2026-10-28，进行中）

- 主会话确认依赖批次1–5完成；46已提交71143cc8a，等待公共修复唯一实施确认。未修改plan.md或其他题资产。
- 已读取17份完整官方材料并核对catalog逐文件SHA；题面/配置SHA与任务一致。原父题未运行、未下载层。
- 本题合成QEMU四子题已建；20个控制通过。存在性弱checker被12个错误fixture击穿（退出1）；恢复实际QMP/COM1/VNC/帧缓冲检查后20个通过。随后增加COM1随机按键回显挑战，当前运行聚焦回归。
- 公共阻塞：diagnostics.py:95拒绝点号官方ID；state.py:63同样拒绝正式预约ID。调用链run.schedule→admit/reserve→worker要求case_id等于taskId，不能用现有attempt UUID替代。仅提出两处最小点分段ID语法兼容，尚未改公共文件。
- diagnostics.py修改使全部validation实现哈希过期；state.py修改改变execution_identity；preparation嵌入validation及scheduler身份，需原会话在修复后重跑20fixture+10个fresh父/子oracle/nop，不能改旧收据hash。
- 原镜像manifest sha256:c6695097316ef8810abce0545e347e65354aef9d51d4f7f662c3242e742f6a3e，amd64，14层C=573371248字节，U未知。按3C+U+10GiB存储+2GiB缓存+1GiB归档+5GiB预留，未计U的下限21047466576字节，实测host available=19025465344，至少缺2022001232字节；尚未准入，不降低阈值、不清其他题。
- Cargo实际动态槽位1；CARGO_TARGET_DIR=/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1；同值CARGO_BUILD_BUILD_DIR，CARGO_BUILD_JOBS=1。cargo clippy --all-targets --all-features -- -D warnings退出0，行为测试0；锁已释放，缓存保留。evals/promptfoo/score.sh test退出0。
- 新证据位于忽略目录evals/promptfoo/results/qwen38-suite/diagnostics/install-windows-3.11/，含unit-green、unit-red-checker、unit-restored、unit-challenge日志、validate-blocked.log、capacity-lower-bound.json、镜像probe及clippy/score-test记录。未生成validation或preparation成功收据，未提交，未宣称就绪。

### 公共前置已完成，父题容量阻塞

- 2026-10-28唯一授权公共修复已提交04f324f87（diagnostics.py、state.py及两处相邻测试）。只扩展非空安全点分段，不改变官方ID、评分、模型、预算；Harbor现有Compose转换已验证，无额外公共范围。
- TDD：id-red.log 4个测试/4个错误；修复后id-green.log 20项通过（含scheduler真实Harbor oracle/nop控制），shared-regression.log 23项通过，compose-id.log 1项通过。一次错误地传入非digest镜像参数的失败记录单独保留，改用真实固定digest后通过，未放宽加载器。
- 本题20项真实QEMU控制（含COM1随机键挑战）通过；suite.diagnostics validate --run-fixtures退出0生成新validation.json。完整父题和八个fresh Harbor子题尚未校准，不能宣称完成或删除本任务文件。
- 公共稳定收据shared-fix.json及clippy-shared.log已保存。再次Clippy退出0，动态槽位1、绝对CARGO_TARGET_DIR=/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1，CARGO_BUILD_BUILD_DIR同值、CARGO_BUILD_JOBS=1；锁已释放，缓存保留。
- 当前唯一阻塞是父镜像磁盘准入：至少缺2022001232字节且未知U尚未计入，不能在阈值内下载或导入。只读取manifest/config，未运行父题preflight；不删除他题或业务资产、不降低存储/缓存/归档/安全余量。原题17文件SHA完整匹配。
- 恢复入口：主会话提供合规容量窗口；先刷新host/VM容量与原manifest/config身份，对未知U设安全Ucap预约，再受限下载同digest、逐层全SHA及diff_id/精确U验证，重新准入；随后核实verifier-only依赖并用本题局部runtime-v2配置运行原父题2CPU/4096MiB/3600秒和四子题fresh oracle/nop。成功后重跑最终门槛并单独提交本题资产，才可标完成及删除编号文件。公共身份变化下他题回归由主会话调度，未修改旧收据。

### 轻量恢复完成（2026-10-28）

- 按主会话授权，仅恢复子题与只读容量探测；公共文件未改，父题未下载/导入/启动。原20fixture成功日志与validation-before-tag-digest.json归档保留；case镜像现为已核验task47 tag+同一sha256:47b5cc7ff9a4771ba085761d4ab7bc06d06ed7c0318e892edbd7d2d693836aca，重新20fixture校准退出0。
- 当前公共身份下4子题8个fresh Harbor控制全部有效，oracle各1、nop各0，无异常/超时/评分恢复；children-accepted.json与children-complete-evidence.json绑定当前declared case/validation、scheduler/adapter/Harbor、各task树/评分树/资源收据；audit-children.py退出0。
- 构建初始失败均在Agent前，保留各原始日志；BuildKit本地裸digest及tag+digest解析失败，独立验证legacy builder后仅本题进程使用DOCKER_BUILDKIT=0、COMPOSE_DOCKER_CLI_BUILD=1。生成包局部Dockerfile显式FROM --platform=linux/amd64且digest不变；局部Compose只固定main为amd64，并为native侧车设置0.25CPU/64MiB，main保持1CPU/512MiB。8个主镜像均amd64，RootFS含原固定基镜像完整前缀；checker/题面/fixture与当前声明资产逐字节一致。
- 最初m1两次虽评分1/0但侧车未限额，未计有效校准；额外fresh bounded m1控制替代，m2–m4原已限额6控制直接核验复用，未重复。相关失败与排除控制均保留。当前所有本题容器已清理。
- Registry固定manifest未漂移，C仍573371248；14个gzip尾Range读取均206，ISIZE合计下界1576205312，明确仅下界非精确U，不把未知体积视作零。2C流式方案可只保留parts+已核验blob，核验每个完整blob后删除对应parts，OCI tar经pipe直接load不落第三归档；仍必须硬Ucap、全层SHA/diff_id、精确U二次准入并得到主会话放行。
- 最新host/VM、可容纳Ucap、按下界和2GiB硬Ucap的缺口见capacity-two-copy-final.json；尚无可行安全预约，父题重磁盘继续等待，未持有独占容量预约。保留已阻塞状态，本文件不删；本轮无Cargo调用，前阶段槽位1及绝对CARGO_TARGET_DIR证据仍在clippy-shared.log。

### 父题窗口及最终身份恢复（2026-10-28，进行中）

- 主会话明确放行47独占父题重磁盘。实时host=22239686656、VM=92207075328，选Ucap=1600MiB=1677721600，2C+Ucap+18GiB+16MiB=22168594144，余量71092512字节。下载受限分段续传、240秒/401更新匿名token，仅内存持有，无密钥；完整blob全SHA后删除对应parts，未落第三档。
- 14层全SHA/diff_id已校验，精确U=1576205312低于硬cap；精确U重新准入后OCI tar经pipe流入docker load，退出0，未触发free阈值，原amd64 digest/config/RootFS均匹配。import-complete.json、checkpoint-verified-layers.json、verified-download.json、capacity-admitted.json、import-capacity-samples.json为新证据；宿主导入后21110874112字节。原父题镜像及其原CMD已保存，不改上游17文件。
- 独立verifier探针通过：真Python3.13.7/uv0.9.5、pytest8.4.1/cv2 4.11.0/numpy2.3.1/pytesseract0.3.13/Pillow11.3.0及mtools/socat/vncsnapshot/tesseract均可运行。新增wheel直接SHA绑定PyPI正式文件，复用SHA固定Python/uv/curl闭包；只有verifier阶段注入，原Agent不改变。dependency-probe.json保留命令及退出码；官方uv脚本端点curl成功。
- 原Dockerfile/镜像CMD为supervisord，原解只起QEMU/websockify不单起nginx；Harbor模板覆盖sleep infinity丢失启动契约。独立原CMD/keepalive对照分别80监听/不监听。主会话授权仅main.command:null现有Harbor入口恢复；生产suite缺声明/绑定入口，因此授权唯一修改scoring.py/preparation.py/run.py及相邻测试，已提交1a19b2cf3。受信case/environment内source basename+SHA，拒绝路径/符号链接/未知键与任意额外Compose内容，只接受固定canonical main.command:null；只读临时快照、env-start/agent-start复核；未声明不传新Harbor参数，子题不继承父覆盖。
- 5项初始红测及prepare/worker故意漏接线红测均捕获错误；48项相关回归、16项作用域回归通过。实际生产harbor_trial独立探针证明仅Compose command差异、原supervisord PID1、nginx80自启、amd64/2CPU4GiB、无额外服务。完整Compose及SHA、CMD来源及root原探针已归档production-startup-probe.json/cmd-boundary-report.json。公共实现稳定，主会话已通知；旧26题回归仍由主会话处理，未改旧收据hash。
- 新公共身份使此前8个子题失效，全部保留旧收据。本题诊断镜像改为同config/同RootFS的单平台schema2 manifest cad7d685888e2160a644f370f75e29200666a0c302458312917fed731baedf8e，避免canonical materialize在ARM宿主Legacy构建选错平台；实际程序/层/配置未变，diagnostic-manifest-equivalence.json证实。case与validation旧副本保留，当前20fixture重新校准通过（validate-final.log）。无需临时改生成包，最终走标准生产preflight。
- 当前运行精确命令：PYTHONPATH=evals/terminal-bench DOCKER_BUILDKIT=0 COMPOSE_DOCKER_CLI_BUILD=1 evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.run preflight --catalog evals/terminal-bench/suite/catalog.json --runtime evals/promptfoo/results/qwen38-suite/diagnostics/install-windows-3.11/runtime.json --parent-id install-windows-3.11 --output evals/promptfoo/results/qwen38-suite/preflight/install-windows-3.11。exec session57656，首次生产父题oracle正在原样编译QEMU5.2.0（原解make-j4，cgroup仍2CPU4GiB、Agent/verifier各3600秒）；不重复已有效控制。最终必须检查父题4个真实测试及异常/超时，再新8子题并production verify_preparation。
- watch-final.py（session87067）记录真实资源/有效Compose SHA/PID1/cgroup并于结果保存后清本题容器；对诊断侧车在创建时设置0.25CPU64MiB并记录时间。capacity-guard.py（session5679）在host free低于6GiB时立即kill仅本题容器并拒绝相关评分。完成/失败后写忽略目录stop-final-watch停止两守护脚本。尚无完成声明。
- 新公共接线Clippy退出0，动态槽位1，绝对CARGO_TARGET_DIR=/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1，matching CARGO_BUILD_BUILD_DIR、CARGO_BUILD_JOBS=1，锁已释放；clippy-startup.log保留。无Rust修改，无fmt及全量单测。

### 首次最终父题校准：Rosetta基础设施阻塞（2026-10-28）

- 标准生产preflight在原父题oracle处退出1，原始结果完整保留在preflight/install-windows-3.11/calibration/install-windows-3.11-oracle；未运行nop及最终新8子题。原解完整编译2289项QEMU5.2.0，Agent实际约586秒，低于3600秒；VM启动时报`rosetta error: Unimplemented syscall number 282`。原结果exception_info为None不能当成功；原reward0、官方4测试1通过3失败0跳过，属于基础设施失败而非能力失败/有效nop。
- 现有case原CMD恢复确已生效：实际PID1为/usr/bin/python3 /usr/bin/supervisord，主容器2CPU/4GiB，完整有效Compose及SHA保存在install-windows-3-11-oracle__env-execution.json。原镜像/题源/参考解/checker/模型/预算未变。只是Rosetta未实现原QEMU调用的282。
- 仅做独立无网络真实API探针：原容器头文件确认__NR_signalfd=282、__NR_signalfd4=289；289成功创建真实FD，并接收SIGUSR1的128字节内核siginfo（signalfd4-probe.log）。原镜像无现成qemu-user路径（emulator-readonly-probe.log）。尚未添加shim、改Agent环境或重试父题。
- 若以真实289(flags=0)转发282的局部ABI兼容层修复，将变更Agent前运行环境，超出当前仅verifier依赖与CMD恢复授权；已向主会话报告具体边界，等待其判断。不得悄然预装QEMU/改原解/伪造FD/空返回/回放产物，也不得切ARM、更改全局Docker/Rosetta或业务容器。
- parent-infrastructure-failure.json明确单列该失败，保留raw0/Agent日志SHA/4个真实测试结果，未修改原评分或旧收据哈希。stop-final-watch已写入以结束两守护；任务容器已按原结果保存后清理，本题不占Cargo锁，编译缓存保留；目前保留已阻塞状态，不删除编号文件、不宣称已就绪。

### 安全收尾与切回点（2026-10-28，已阻塞）

- 主会话确认不添加shim/兼容层、不改Agent前环境、不重试同条件父题。仅有runc/io.containerd.runc.v2两个已注册每容器runtime，均指向runc；在原amd64 digest、2CPU4GiB、无网络的小型282真实探针中均退出133并复现Rosetta错误。未安装或切换后端、未改Docker/Rosetta/binfmt全局设置，未访问其他主机。证据available-runtimes.log、per-container-runtime-probes.json。
- 原raw0/4测试1pass3fail0skip、真实Rosetta282错误、约586秒原Agent及nop未运行事实均保留。parent-infrastructure-failure.json明确score_status=unscored、failure_class=infrastructure、reward=null，同时保留original_verifier_reward=0，严禁当能力结果或有效nop。当前公有实现下最终8子题未运行，旧8控制已失效，不冒充复用。
- 原父题preflight共50文件无修改rename至diagnostics/install-windows-3.11/preflight-failed-rosetta-original，preflight-archive-map.json记录原路径及逐文件SHA。原始score/result/日志/CTRF均不改；活动preflight路径已释放。stop-final-watch已停止两守护进程，核实本题容器/网络/监控/Cargo锁均为0，无下载导入进程；安全切回主会话放行48。
- 原镜像c6695097316ef8810abce0545e347e65354aef9d51d4f7f662c3242e742f6a3e无容器引用，唯一tag alexgshaw/install-windows-3.11:20251031；完整OCI14层及config/manifest保留，压缩blob再次全SHA核验。parent-cache-cleanup-candidate.json/blocked-handoff.json给精确候选及离线归档路径，只报告候选，未额外删除镜像/全局缓存。
- 最后实测host available=19163594752字节，VM available=90103689216字节；blocked-handoff.json保留原始读数、槽位1、绝对CARGO_TARGET_DIR=/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1及锁释放状态。公共04f324f87/1a19b2cf3及证据保留；本题未验证资产维持未跟踪不提交。任务明确已阻塞，不删除编号文件。
- 恢复需用户另行授权原生amd64环境或单列兼容变体，保持原镜像/题源/评分/资源/时限，重新取得有效父题oracle/nop与最终身份下8子题，不能仅改收据哈希。主会话统一协调环境，无需本会话再询问用户权限。

### 已授权精确缓存维护（2026-10-28，任务仍已阻塞）

- 主会话明确授权按capacity-policy26卸载本题原父镜像缓存，不运行Agent、不安装shim、不修改环境或任何业务/诊断镜像。维护前再次验证完整OCI index/layout/manifest/config、14层全部压缩SHA/diff_id，精确U=1576205312；官方17文件、50原始历史文件、11项已用verifier依赖SHA完整；原raw0/1pass3fail及unscored/infrastructure分类不变。
- 精确原镜像ID/manifest=sha256:c6695097316ef8810abce0545e347e65354aef9d51d4f7f662c3242e742f6a3e，唯一tag alexgshaw/install-windows-3.11:20251031；全部容器、子镜像RootFS前缀和外部别名核验均0。维护前完整审计parent-cache-audit.json。
- 已执行docker image rm --no-prune sha256:c6695097316ef8810abce0545e347e65354aef9d51d4f7f662c3242e742f6a3e，退出0，不force/prune、不操作VM全局设置。其余511镜像的tag/config哈希/RootFS完全未变，业务、vw诊断镜像和共享层保留；所有host OCI blob/元数据、源文件、历史评分、已用依赖和Cargo缓存保留。删除后再次全SHA核验所有host OCI blobs，原镜像本地已不存在、零容器引用。
- 维护前host=19139874816、VM=90103607296字节；10秒后实测host=21234409472、VM=92200452096字节，实际回收host=2094534656、VM=2096844800字节。即时/3秒/10秒原始读数和确切命令/输出/退出码见parent-cache-unload.json；maintenance-handoff.json确认窗口已释放。
- OFFLINE-RESTORE.md、verify-offline-oci.py及restore-parent-offline.py给同digest离线恢复指引。离线只读验证已再次运行；恢复helper本轮未执行，须原生amd64或另授权兼容变体及重新真实容量准入，保留2C+精确U+18GiB+16MiB规则、流式load无第三档。原解自身QEMU网络下载契约不变，不预装或用shim替换Agent。
- 本轮仅维护，无Cargo调用、无原题校准重跑、无Git提交或未验资产提交；任务精确状态仍已阻塞，编号文件保留。活动preflight/容器/网络/监控/锁均0，安全切回主会话放行48；公共04f324f87/1a19b2cf3及全部证据保留。
