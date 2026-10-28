# 任务 69: 准备-polyglot-c-py

批次：【批次 40】；依赖批次：1、2、3、4、5。同批无文件依赖；Git 提交按任务编号串行，禁止删除其他会话的索引锁。

状态：已阻塞

目的：为“双语言公共源码”准备原题完整环境与四道独立诊断题。

来源任务：Terminal-Bench2.1/polyglot-c-py

预计会话范围：只准备一个父题的四个窄诊断题和确定性校准；不运行付费模型。长镜像拉取允许记录进程后续接，不能把未校准标成完成。

## 文件

- 新建：`evals/terminal-bench/suite/cases/polyglot-c-py/case.json`、`evals/terminal-bench/suite/cases/polyglot-c-py/check.py`、`evals/terminal-bench/suite/cases/polyglot-c-py/tests/test_cases.py`、`evals/terminal-bench/suite/cases/polyglot-c-py/fixtures/`、`evals/terminal-bench/suite/cases/polyglot-c-py/oracle/`、`evals/terminal-bench/suite/cases/polyglot-c-py/environment/Dockerfile`、`evals/terminal-bench/suite/cases/polyglot-c-py/README.md`
- 只读完整上游包：`evals/promptfoo/results/qwen38-suite/source/tasks/polyglot-c-py/`
- 忽略产物：`evals/promptfoo/results/qwen38-suite/preflight/polyglot-c-py/`、`evals/promptfoo/results/qwen38-suite/diagnostics/polyglot-c-py/`

## 上下文

原题 polyglot-c-py；类别 software-engineering；CPU=1、内存=2048MiB、存储=10240MiB、GPU=0；Agent=900秒、verifier=900秒。预建镜像 alexgshaw/polyglot-c-py:20251031。原题树SHA=2c32f675679be8cc5e0697eda588c65cb1c4ca88；题面SHA256=a57c0c0e1232113113cd29a154ea7e0ac9ea45e4aedc16958fbc6bc753a4ef96；配置SHA256=d31b9f60c47eed0d79e10d3be8fd56a23e50b3986606e3f2546863d44ac9a9ea。读完整 instruction.md、输入、solution、tests 后设计，禁止改原题。四子题：m1 找到两种解析器的公共语法；m2 实现共享小型算法；m3 验证参数与数值边界；m4 比较两种运行结果。

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

先使用技能 ./skills/create-agent-evals/SKILL.md；每个子题定义合成输入、清楚完整题面、输出路径/API/服务行为与独立确定性判定。新增 evals/terminal-bench/suite/cases/polyglot-c-py/tests/test_cases.py：正解通过，至少3个有意义错误变体被拒绝；运行真实程序/产物，不验证关键词、自述或仅Agent自测。错误fixture和漏检checker先失败，再最小修复。

验证命令：

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/polyglot-c-py/tests -p test_cases.py
```

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/polyglot-c-py/case.json --run-fixtures
```

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.run preflight --catalog evals/terminal-bench/suite/catalog.json --runtime evals/promptfoo/results/qwen38-suite/runtime.json --parent-id polyglot-c-py --output evals/promptfoo/results/qwen38-suite/preflight/polyglot-c-py
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


## 2026-10-28 执行证据与恢复入口

依赖批次1–5已由主调度确认满足。已读取完整上游 instruction、task、
Dockerfile、README、solution、tests 与 gitignore，未修改上游或 plan.md。
仅新增本题独立合成诊断资产，四题分别为公共语法 marker、三角和、
参数校验/阶乘数值边界、双运行时多输出比较；没有复制父题答案前缀。

已通过的新证据：
- 指定 unittest 命令退出0：1方法、20个gold/nop/三负例控制，真实编译C并运行Python。
- TDD：初始错误窄整数fixture被测试发现，修为显式转换；去掉checker输出比较产生10个断言失败，恢复后20控制全部通过。
- 指定 diagnostics validate --run-fixtures 命令退出0：20容器内控制，case目录 validation.json 绑定全部字节与工具digest。
- 8个fresh标准 suite.scoring.harbor_trial 控制全部首次评分 oracle=1/nop=0；主容器1CPU/512MiB，sidecar 0.25CPU/64MiB，使用公共受限builder，无私有平台覆盖或事后限额。
- evals/promptfoo/score.sh test 退出0；没有模型调用。
- cargo clippy --all-targets --all-features -- -D warnings 退出0；动态实际槽位1，CARGO_TARGET_DIR=/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1，CARGO_BUILD_BUILD_DIR同路径，JOBS=1。已释放自己的锁，缓存保留；无Rust改动，无全量测试。只有既有block 0.1.6未来兼容提示。

具体阻塞：主调度要求父镜像body下载及原题校准另行协调串行重阶段，
当前仅获准cached轻量诊断。只读Registry查询确认父manifest
sha256:0f1c3b7816d70cf5551573fd6aeef76893f2ae3000be2419997b6871b5d987ed，
linux/amd64，C=148540671字节；Range 206尾部探测的345676288字节仅为U下界，
exact U与正安全Ucap未知，不能按0计入2C+U+S+B+A+5GiB联合准入。
未下载父层body、未启动父题oracle/nop、未执行会隐式准备父镜像的preflight；
父题控制数0，不能宣称完成/已就绪。未清理其他任务或业务资源。

忽略证据根：evals/promptfoo/results/qwen38-suite/diagnostics/polyglot-c-py/。
包括host-test.log、red-checker.log、validation.json、standard-children.log、
children-calibration-production-leaf.json、每trial的scoring/build/admission/resources、
registry.json、layer-tail-probe.json、capacity-final.json、checkpoint.json、
clippy.log/clippy-receipt.txt及promptfoo-test.log。
初次controller在validation.json落盘前失败，未创建trial/score；原日志独立保留，
随后8个有效标准控制均只执行一次，未重跑已有评分。

恢复：先读取checkpoint与capacity-final，重新实测host/VM与联合预算，明确正安全
Ucap和全部导入副本/metadata占用，协调父题重阶段；不改变S=10240MiB与900秒超时。
准入后按本文件原preflight命令（runtime缺historical_root时使用ignored runtime.json）
完成原题oracle通过/nop失败，检查身份与收据后再提交本题资产、删除本任务文件。
本次按阻塞分支保留编号与未提交诊断资产，不修改公共文件或其他题目录。


## 2026-10-28 父题独占窗口恢复：准入拒绝与ABI阻塞

主调度已授权本题独占父body下载/导入/校准窗口，73仅host工作。
恢复时先逐字节核对原20fixtures、8标准校准、case/image/checker、原题与
runtime/adapter/scheduler身份，全部未变，未重跑有效评分；命令
`PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python evals/promptfoo/results/qwen38-suite/diagnostics/polyglot-c-py/verify-resume-evidence.py`
退出0，收据resume-identity-verification.json。

按当前实际3次host/VM共同采样，最小可用19609518080字节，VM约93527314432字节。
拟定正硬停止解压预算512MiB（不是已知upper），保守4份C、2份U、metadata128MiB，
加原S10GiB/B2GiB/A1GiB/5GiB安全余量，峰值21129475068字节，缺1519956988字节。
即便只算1C+已知U下界+原18GiB预留+128MiB metadata，也需19955787519字节，
高于上述实测。准入程序明确退出1，未拉层、未导入、未试跑父题；没有空循环等待。
本轮拒绝与3次样本见parent-admission-authorized.json（admitted=false）。

另执行同固定诊断image的冻结x86_64 ELF --help：network=none、只读root、
只读冻结bundle、无APIkey/配置、0.25CPU/256MiB，sha256核对一致。
实际失败：`rosetta error: failed to open elf at /lib64/ld-linux-x86-64.so.2`，
镜像缺ELF interpreter。失败日志frozen-agent-help.log、
frozen-agent-probe.json/resources与probe-agent-diagnostic.log完整保留；
探针容器在finally清理。该失败是ABI基础设施失败，不是模型成绩，
既有20+8只是checker/oracle/nop校准，不能据此宣称冻结Agent可运行。
未安装Agent依赖或修改镜像绕过；父image未准入，父ABI与uv/Python/pytest/
libsqlite3闭包probe均未运行。无代码修改，沿用已成功的Clippy槽1/绝对target证据。

恢复入口更新：先解决实际磁盘缺口；为诊断选择可运行冻结ELF且满足真实工具需求的
固定trueleaf，任何case/image改动均重新20fixtures+8标准控制，并先离线ABI探针。
父镜像仍须正硬cap全stream解压校验SHA/diff_id/exactU及import再次准入，
再做原父ABI/真实verifier依赖probe、必要verifier-only修复、原oracle/nop及
最终inputs/verify_preparation。不得把旧哈希改成新身份或把checker校准当Agent可运行。
最终3次实际post-cleanup容量、owned资源清理及两项阻塞见resume-blocked-checkpoint.json；
旧checkpoint/capacity与所有原失败未覆盖。本题仍已阻塞，保留未提交资产和编号。


## 2026-10-28 ce05045执行身份恢复：当前容量再次拒绝

已读取共享提交4ef4878e6/cee07225e，实际inputs与execution_identity核对为
ce05045daeb0b6f9a28b5cd179f2f4363cfbbaf143a099565e27218a494a4bd1。
父manifest/C记录一致；原checker-only20控制的全部字节与实现哈希未变，可沿用。
旧8标准控制绑定f759身份，现已明确标为过期，当前身份标准控制=0，父题控制=0；
未把旧hash改写为新身份，未启动评分、模型或Cargo。此前Clippy槽1成功证据保留。

本次唯一容器/镜像窗口内先做三次实际min(host,VM)采样：
- host=13579882496，VM=94007345152字节；
- host=13579321344，VM=94007349248字节；
- host=13578883072，VM=94007353344字节。
最小13578883072字节，明显低于原S10+B2+A1+安全5GiB=19327352832字节，
不能引用旧20.2GiB快照准入。选择352MiB正硬停止cap=369098752字节，
>=已知U下界345676288，但不是已验证upper；保守4C+2U+128MiB metadata
加固定18GiB预留，峰值20793930748字节，缺口7215047676字节。
准入命令退出1，admitted=false，尚未下载/导入任何父层，无反复空循环。

证据：忽略目录parent-admission-ce05045.json（全部样本与copies/cap预算）、
checkpoint-ce05045.py及blocked-checkpoint-ce05045.json（当前inputs、
20checker复用核验、旧8控制失效、拒绝与清理）。核验命令退出0。
三次只读探针各自--rm，结束无本题容器；未创建镜像层、网络、卷，无他题清理。
诊断冻结ELF缺interpreter的历史ABI失败仍保留、未掩盖；本轮因容量未准入，
未刷新8标准控制或重跑ABI，不声称已准备完成。

下一步：实测恢复足够空间后重新联合准入，再解决固定工具leaf的冻结ELF ABI，
按字节变化重校准20checker和当前身份8标准控制，最后完整原父oracle/nop、
原父ELF及verifier闭包probe、inputs/verify_preparation。保持本题已阻塞，
未提交诊断资产，保留编号与所有旧收据；plan.md未修改。


## 2026-10-28 诊断ABI修复完成，原父仍容量阻塞

本次独占容器窗口按诊断S1+B2+A1+安全5GiB单独准入，不再用父题容量不足
跳过可独立完成诊断。候选`vw-cython-base:single@...`名称不存在，已通过实际
docker image ls/inspect定位同digest真实缓存名，未拉取或建立别名：
`vw-log-summary-diagnostics:task52-single@sha256:32c8a19cdeab820f8ea710fcdacaef0893848898158f0d3ccc890a75ab365adf`。
Descriptor=真实Docker v2单manifest，AMD64，config=a6c16930afacbb61ef2e9f1e5f994d6c560b787fa4c5f849b5c15b3f017d111c。
审阅完整history仅工具/依赖安装及wheels拷贝；真实只读probe确认Python3.13.7、
gcc12.2.0及/app为空，不含上游/他题题面源码答案。

准入3次min(host,VM)最低12696649728字节，高于原诊断S1+B2+A1+安全5GiB
加metadata128MiB=9797894144字节；缓存复用无新增下载/导入副本。
仅修改本题case.image与environment/Dockerfile及README，无原父Agent环境/评分变更。
旧case与checker receipt已复制到忽略目录before-abi-fixed文件，全部旧失败保留。

已获得新证据：
- 指定host unittest退出0，1方法20控制；host-test-abi-fixed.log。
- 指定diagnostics validate --run-fixtures退出0，fresh20控制；case/validation.json
  与validation-abi-fixed.json绑定新case/image与checker/fixtures全部字节。
- 当前ce05045daeb0b6f9a28b5cd179f2f4363cfbbaf143a099565e27218a494a4bd1
  fresh8正式Harbor控制全部首次oracle=1/nop=0，证据abi-fixed-run/。
  公共suite.scoring.harbor_trial/SuiteDockerEnvironment，主1CPU/512MiB、
  sidecar0.25CPU/64MiB，受限build/start admission、无私有platform/Trial/Compose覆盖。
- 同固定工具镜像、同冻结x86_64 ELF（SHA847823c5cd99a6828cddfc2e3e5dbb67c5ddcc9f8693a13d539d8a8fa47c9dae）
  离线--help退出0，sidecars/ELF与当前inputs身份核对一致；network none、只读root及
  bundle、0.25CPU/256MiB，无key/config、无新Agent依赖。该证据仅ABI启动，不是模型成绩。
  abi-fixed-frozen-agent-{help.log,probe.json,probe-resources.json}保留详细证据。
- finalize-abi-fixed.py退出0，核对所有8首评分、20checker、当前inputs、task树与
  scoring文件哈希、创建时资源限制、受限builder及清理；
  diagnostics-ready-parent-blocked-ce05045.json为最终新checkpoint。
- 无新checker/Rust代码，沿用此前Clippy退出0，动态slot1、绝对target/builddir
  /Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1，JOBS1；锁已释放缓存保留。

清理后实际3次host/VM：
- 2026-10-28T03:38:14.918815+00:00: host=12792745984，VM=94005497856字节。
- 2026-10-28T03:38:16.175969+00:00: host=12792561664，VM=94005497856字节。
- 2026-10-28T03:38:17.468725+00:00: host=12792279040，VM=94005497856字节。

最低12792279040字节仍低于父题18GiB固定预留；按原父352MiB正硬停止cap的
完整保守峰值20793930748字节，缺8001651708字节。
未重试父下载/导入，父oracle/nop=0，原父ABI与uv/Python/pytest/libsqlite3
真实verifier闭包均未验证，不能宣称完整准备完成。
所有本题临时容器/网络/卷已清理；保留缓存、全部旧新原始证据，无模型调用。
任务状态仍已阻塞，保留编号与未提交资产；下一步仅在原父完整预算实际准入后，
完成父SHA/diff_id/exactU、import准入、ABI/依赖probe和原oracle/nop，
最终inputs/verify_preparation通过再按原契约提交删除。plan.md只读未改。
