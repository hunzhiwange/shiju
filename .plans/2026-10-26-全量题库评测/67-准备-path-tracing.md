# 任务 67: 准备-path-tracing

批次：【批次 39】；依赖批次：1、2、3、4、5。同批无文件依赖；Git 提交按任务编号串行，禁止删除其他会话的索引锁。

状态：已完成，待回归验证

目的：为“图像生成复现”准备原题完整环境与四道独立诊断题。

来源任务：Terminal-Bench2.1/path-tracing

预计会话范围：只准备一个父题的四个窄诊断题和确定性校准；不运行付费模型。长镜像拉取允许记录进程后续接，不能把未校准标成完成。

## 文件

- 新建：`evals/terminal-bench/suite/cases/path-tracing/case.json`、`evals/terminal-bench/suite/cases/path-tracing/check.py`、`evals/terminal-bench/suite/cases/path-tracing/tests/test_cases.py`、`evals/terminal-bench/suite/cases/path-tracing/fixtures/`、`evals/terminal-bench/suite/cases/path-tracing/oracle/`、`evals/terminal-bench/suite/cases/path-tracing/environment/Dockerfile`、`evals/terminal-bench/suite/cases/path-tracing/README.md`
- 只读完整上游包：`evals/promptfoo/results/qwen38-suite/source/tasks/path-tracing/`
- 忽略产物：`evals/promptfoo/results/qwen38-suite/preflight/path-tracing/`、`evals/promptfoo/results/qwen38-suite/diagnostics/path-tracing/`

## 上下文

原题 path-tracing；类别 software-engineering；CPU=1、内存=2048MiB、存储=10240MiB、GPU=0；Agent=1800秒、verifier=1800秒。预建镜像 alexgshaw/path-tracing:20251031。原题树SHA=c5279f770359041a0c908217d46329711dfbcb7d；题面SHA256=9f36e02d636f5dc0946619bc518f2e51c24fa75251da9d1f0ca9eca612cb9572；配置SHA256=cf47013f69bf1b43242da984f231d4daf364fdf44d3845f2eea2ca2cb9308c46。读完整 instruction.md、输入、solution、tests 后设计，禁止改原题。四子题：m1 读取小型目标图像；m2 实现简单场景渲染；m3 处理坐标和颜色边界；m4 编译并比较图像产物。

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

先使用技能 ./skills/create-agent-evals/SKILL.md；每个子题定义合成输入、清楚完整题面、输出路径/API/服务行为与独立确定性判定。新增 evals/terminal-bench/suite/cases/path-tracing/tests/test_cases.py：正解通过，至少3个有意义错误变体被拒绝；运行真实程序/产物，不验证关键词、自述或仅Agent自测。错误fixture和漏检checker先失败，再最小修复。

验证命令：

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/path-tracing/tests -p test_cases.py
```

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/path-tracing/case.json --run-fixtures
```

```bash
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.run preflight --catalog evals/terminal-bench/suite/catalog.json --runtime evals/promptfoo/results/qwen38-suite/runtime.json --parent-id path-tracing --output evals/promptfoo/results/qwen38-suite/preflight/path-tracing
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


## 2026-10-28 执行检查点（独立实施后）

- 依赖批次 1–5 由主调度确认满足。当前阻塞仅为主调度暂停 Docker 新构建、父镜像下载、标准 Harbor 和容器校准，等待公共任务 7 生产/prepare/评分恢复资源修复稳定。未绕过公共入口、未改公共文件或其他题、未调用付费模型。
- 已完整阅读本题上游 instruction、task.toml、Dockerfile、orig.c、solve.sh、test_outputs.py、test.sh、README；题面和配置 SHA256 实测均符合本文件冻结值。实际官方要求静态编译、zlib 小于 2100、chroot 执行和像素余弦相似度 >=0.99，不按 README 的 98% 描述改评分。
- 已创建本题 case.json、check.py、tests/test_cases.py、四组独立合成 fixtures/oracle/gold/各三语义负例、environment/Dockerfile 和 README。m1 执行 P3 解析程序，m2 执行球体轮廓渲染，m3 执行坐标 floor/颜色 clamp 量化，m4 实际编译 C 程序并在无参考图的临时目录运行、比较新生成 PPM。使用多个合成输入，未使用父题解答或样例回放。
- TDD：初始缺资产 unittest 退出 1（仅装配失败）；实际移除最终输出比较门槛后，同一测试命令退出 1，13 个断言失败（12 语义负例与 1 回放程序）；恢复门槛后退出 0，2 个测试方法通过，覆盖 20 个 gold/nop/负例候选检查和 3 个固定回放拒绝。确切命令：`PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/path-tracing/tests -p test_cases.py`。
- `PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/path-tracing/case.json` 退出 0，仅 schema/hash 验证，checks=[]；不是容器校准，不保存为正式 validation.json。
- `evals/promptfoo/score.sh test` 退出 0，Configuration is valid。`cargo clippy --all-targets --all-features -- -D warnings` 退出 0；动态领取槽位 1，CARGO_TARGET_DIR=`/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1`，CARGO_BUILD_BUILD_DIR 同路径，CARGO_BUILD_JOBS=1。无 Rust 修改，不运行 fmt；自有 owner 与 slot-1.lock 已清理，缓存保留。Clippy 是编译静态检查，不计为行为测试。
- host 为 macOS aarch64，m4 使用系统 cc 非静态编译；Linux 正式 checker 使用 gcc -static。case 使用已有 vw-cython-diagnostics digest（见 case.json），尚未证明其 amd64/静态 libc 兼容性，未构建/拉取镜像。host 23 个候选检查不能当作容器或父题评分证据。
- 未执行：diagnostics validate --run-fixtures、preflight、父题完整 oracle/nop、4 子题 fresh oracle/nop/负例校准和资源隔离收据。状态保持已阻塞，保留本文件。恢复时由主调度确认公共入口稳定/串行窗口，按步骤 4–6 补充，不提前标记完成；下载须核算 min(host,VM)、正 Ucap、全部压缩副本和 18GiB/metadata 余量；最终公共实现身份和依赖/fixture/checker 哈希重新冻结。

### 公共入口稳定后的 checker-only 补充

- 主调度通知公共修复提交 `964419691` 稳定，已阅读 `suite/BUILD_RESOURCES.md`；标准 Harbor/父题下载窗口暂由任务 62 占用，仅允许既有镜像的 checker-only 校准。
- 原选 cython digest 实测 linux/arm64，已排除。`docker image inspect` 实测现有 `vw-memory-diagnostics@sha256:dd66171212a22465171dd84493cf5ba2d5f11142e9e79069566124b232041f3d` 为 linux/amd64；读取其工具 Dockerfile 和 `docker image history --no-trunc`，只含 Python 与 g++/Valgrind 安装，无题目/参考解。case.json 与环境 FROM 已切换到该 digest，未构建/拉取镜像。
- 确切命令：`PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/path-tracing/case.json --run-fixtures`。首轮退出 1，m4 gold 失败；受限独立探针证实 gcc 静态编译成功但 /tmp/image 执行 Permission denied（退出 126）。仅本题 checker 临时目录改到可执行的 /app 挂载下，不改公共资源策略。重复原命令退出 0，validation.json 的 20 条检查全部符合预期；日志 JSON 在忽略目录 diagnostics/path-tracing/checker-validation.json。
- 随后重跑相同 host unittest 退出 0，2 方法/23 候选控制。现已证明 Linux amd64 静态 C 编译及真实 PPM 产物，而非仅 host 编译。共享 diagnostics 入口固定 network=none、1 CPU、512MiB、pids=128、drop ALL、no-new-privileges、只读根与 64MiB tmpfs；每候选 fresh container、无父题挂载。validation.json 只证明 checker-only，不替代标准 Harbor。
- 剩余阻塞更新：不再缺诊断工具；等待任务 62 释放主调度明确保留的标准构建/下载/Harbor 校准窗口。父题 preflight/oracle/nop 与四子题标准 Harbor 控制仍未运行，不能标记已完成或删除本任务文件。

### 最终独立阶段检查点：真实容量阻塞

- 只读 `suite.preparation.inputs` 已成功验证本题 runtime-v2、官方树、Harbor/adapter 身份和 20 项 checker 校准。配置保存在忽略目录 diagnostics/path-tracing/runtime.json，identity-checkpoint.json 的 scheduler_sha256=`f759534dd79bbe107a90b14ddc75b553f285607cd7e9c3fd5ca10d841c43f8e5`。没有启动标准 Trial 或构建。
- `docker image ls` 未发现 alexgshaw/path-tracing:20251031 缓存。2026-10-28T00:10:42Z 的 shutil.disk_usage 实测 host_free_bytes=18684321792，小于单独 18GiB reserve=19327352832，尚未加正 Ucap、压缩副本或 metadata。host 已不满足主调度下载准入下界，不需要猜 VM 剩余或将未知 U 当零；未下载。精确收据：忽略目录 diagnostics/path-tracing/capacity-checkpoint.json。
- 独立可做部分已完成，标准父题校准确受容量与调度窗口双重限制；不删除他人镜像/缓存来腾空间。本题保持已阻塞，资产暂留工作区，未提交，未删除任务文件。恢复需先确认空间完整准入和串行窗口，再通过公共 production/preflight 路径验证父题及四子题，不能以现有 20 项 checker-only 结果宣称整个任务完成。

### 标准窗口尝试与最终单平台身份（当前恢复点）

- 获得窗口后通过公共 `suite.scoring.harbor_trial` / `SuiteDockerEnvironment` 发起 m1 oracle，未使用私有 Trial/Compose 包装，materialize 文件未改。首轮受限构建因原工具 digest 实为 OCI index，默认 arm64 选择失败；记录于 child-calibration-production/。
- 随后确实执行过一次仅该命令/子进程的 `DOCKER_DEFAULT_PLATFORM=linux/amd64`，没有全局 setenv；它影响 ARM sidecar，启动失败、未评分，记录于 child-calibration-production-amd64/。该尝试已退出，不再设置平台变量。第三次去掉平台变量、仅为同 index 引用添加 tag，仍选择失败，记录于 child-calibration-production-tagged/。三次都没有能力成绩，不能计为标准通过。
- 主调度纠正后只读流式 docker save 确认 index=dd661712…041f3d；amd64 leaf=`sha256:731ea515c4600aed7c9e5f6169e71366d0e7cc91a26e54863f13a35fbd5c6e32`；config=`sha256:d91779a0643e542fcf3f1814c439844c39a89ad71973ab4a0e99d872e1ccab7c`。工具镜像大小149706316字节，流式复制约150MB缓存内容、约1MiB缓冲、无磁盘归档/下载/解压/新层/新RUN；本题专用引用通过本地 OCI save/load 将 index 声明改为同 leaf，等价断言 Config、RootFS、Architecture 全部通过。原镜像未改；leaf-equivalence.json 和 local-manifest-metadata.json 记录完整身份。
- 当前 case/environment 正式引用：`vw-path-tracing-tools:single@sha256:731ea515c4600aed7c9e5f6169e71366d0e7cc91a26e54863f13a35fbd5c6e32`。`docker image inspect` Descriptor=application/vnd.oci.image.manifest.v1+json，不是 index；真实 linux/amd64。
- case 字节变化后再次运行原 diagnostics validate --run-fixtures 命令，退出0、20控制通过；当前 validation.json 绑定最终 leaf/case/checker。日志 checker-validation-leaf.json。旧收据仅历史保留，不冒用。
- 标准 leaf 控制尚未启动时，主调度将串行窗口交任务63，明确暂停本题新 builder/标准 Harbor/父下载。当前有效标准控制数量0，未宣称8控制通过。失败尝试的自有容器与空网络已清理；standard-attempt-checkpoint.json 记录。父容量仍阻塞；本任务继续已阻塞、保留资产/任务文件未提交。
- 下个窗口入口：忽略目录 calibrate-children.py 现配置 child-packages-production-leaf、child-calibration-production-leaf、children-calibration-production-leaf.json，直接调用公共 harbor_trial，原 materialize 字节、无需平台变量。运行前重新核对身份、资源与容量，完成8标准fresh控制；父题下载需另行通过完整容量准入。

### 当前最终检查点：8 标准子控制完成，仅父容量阻塞

- 任务63释放窗口后，启动前确认 DOCKER_DEFAULT_PLATFORM 未设置；最终 leaf 的20项validation字节与当前公共身份一致。通过原 materialize、公共 harbor_trial/SuiteDockerEnvironment 串行运行8个fresh标准控制，命令：`PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python evals/promptfoo/results/qwen38-suite/diagnostics/path-tracing/calibrate-children.py`，退出0。标准日志 standard-children-leaf.log，收据 children-calibration-production-leaf.json。
- m1/m2/m3/m4 分别 oracle reward=1、nop reward=0，8项全为 first scorer scored，无基础设施错误/恢复重放/付费模型。未改 materialize 的任何字节，前后任务树与 preparation.inputs 身份一致；scheduler=`f759534dd79bbe107a90b14ddc75b553f285607cd7e9c3fd5ca10d841c43f8e5`。
- 实际主容器inspect为1 CPU/536870912内存，sidecar为0.25 CPU/67108864内存；共享受限构建的 admissions、build日志与 suite-resources.json 均归档。题目Dockerfile只有FROM/WORKDIR/COPY，无RUN；没有私有Trial/Compose包装、平台覆盖或事后docker update。最终审计 finalize-checkpoint.py 退出0，评分证据树/材料树/hash与资源再次核验，所有本题容器、网络、volume已清理。
- 当前原父题registry manifest=`sha256:4b44725102cd627dfa895191a5a30d16dae3c5623bb43da6459f79135db6b482`，linux/amd64，压缩层C=399386034字节；仅metadata/config和严格206 Range每层8字节尾探针，gzip ISIZE得到U下界1095040000，不是精确U/Ucap，不将未知当0。未下载完整层。
- 最新容量：host=18662809600字节，VM=92007329792字节，min仍低于18GiB=19327352832字节总预留，尚未计全部压缩副本、正Ucap、metadata。因此父题下载/完整oracle/nop与preflight仍不可准入，父题控制数0。capacity-final.json、checkpoint.json保存精确证据；整体状态保持已阻塞，保留本文件和未提交资产，不宣称就绪。
- 最终host聚焦unittest再次退出0（2方法/23候选控制）；Clippy沿用本阶段已记录退出0与动态slot1绝对target路径，自有锁已释放。剩余恢复动作仅为容量满足完整下载准入后，经公共preflight完成原父题oracle/nop并绑定整套preparation，随后按契约提交、报告并删除编号任务文件；不等待磁盘变化、不清他人缓存。


### 2026-10-30 恢复：父容量解除、官方父控制通过

- 本轮用户授权继续突破阻塞。重新准入：下载前host=44793032704、VM=74008850432 bytes；按3C+2GiB正硬Ucap+原18GiB预留+128MiB metadata需22807212310 bytes，通过。原manifest未漂移，逐层stream SHA/diff_id全验证，精确U=1095040000 bytes；无磁盘OCI归档的stream load退出0，原manifest/config/amd64/RootFS/Config全部回读一致。父镜像保留供后续使用，旧容量不足已解除。
- 冻结ELF在原父镜像离线--help退出0；原父环境gcc、图像存在，apt/astral/PyPI端点均200。probe-parent-resume.py与parent-*-probe-resume.json保留确切命令和输出；不预改Agent环境或官方verifier。
- 本轮公共身份为7ad9affa98d787ab12315fd30889d1c650700a33e16857dc5f3d602d57ea7318，旧8项子控制不能冒用。原命令公共suite.run preflight已实际执行：父oracle首次reward=1、5项官方测试通过；父nop首次reward=0、5项官方测试失败；m1 oracle=1/nop=0。均原始官方评分，无恢复/付费调用。完整原父与上游树未改。
- m2 oracle在Agent/评分启动前因公共非阻塞资源锁BlockingIOError Errno35失败，preflight退出1；不计能力失败，不伪造整套ready。原始证据位于preflight/path-tracing，preflight-exit.json保存原命令。complete-controls.py只经公共harbor_trial补剩余m2..m4六项、启动前等待至最多一个其他任务及锁空闲，不改公共Trial/Compose/Builder/平台或公共锁；当前仍等待共享窗口，不能生成完整preparation或删除编号文件。
- 本轮聚焦unittest退出0：2方法/23候选；diagnostics validate --run-fixtures退出0：20控制。Clippy退出0，动态slot1，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR=/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1，CARGO_BUILD_JOBS=1；自有锁已清理。无Rust修改，无fmt/全量测试。Promptfoo最初等待及本地npm cookie缺失均留原日志；缓存恢复后npm_config_offline=true evals/promptfoo/score.sh test退出0，Configuration is valid，未改默认配置。
- 所有新下载/导入/身份/资源/命令日志在ignored diagnostics/path-tracing/；当前恢复入口complete-controls.log、completed-preparation.json（仅完成后存在）、audit-ready.py。最终须verify_preparation绑定10项本轮有效控制、当前身份及资源，才可提交本题资产与删除编号文件。


#### 本轮交接：实现完成，等待共享资源回归

- 公共diagnostics.py在本轮等待期间加入另一题rstan-to-pystan的2048MiB分支，导致implementation_sha256由a8a9e489…变为7c98dc25d3b3595b3166f07d8fd68d5d6764c387404b064a0b4320c021589eba；本题512MiB路径未变。重跑同一validate --run-fixtures退出0，20控制通过；重新materialize后整棵文件树与原preflight/diagnostics逐字节相同，diagnostic-implementation-impact.json保存证明。原4项标准成功评分日志与preflight收据均原样保留，没有回写历史哈希。
- 等待窗口期间，任务47预约实际2CPU/4294967296 bytes，任务86预约0.25CPU/67108864 bytes；另有共享控制器活动。按最多2题规则不能启动本题六项，且公共锁由其他活跃控制器持有，未删除/替换其锁或停止其进程/预约。当前原因是无关共享准入，不是旧父容量或本题checker；按任务契约改为“已完成，待回归验证”，保留编号文件和未提交评测资产，不标整题ready。
- 已停止自有等待进程，核验并清空本题容器、网络、卷；没有活跃本题Trial、后台控制或Cargo锁。父镜像与已校验原层/证据保留，未清共享缓存。resume-checkpoint.json保存最终当前身份、4项成功评分、剩余6项、20项当前夹具、23个host控制和Clippy/Promptfoo证据。audit-ready.py尚未运行，cases/path-tracing/preparation.json不存在。
- 可用窗口后的确切入口：`PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python evals/promptfoo/results/qwen38-suite/diagnostics/path-tracing/complete-controls.py`；它核对当前身份和materialize字节，仅调用公共harbor_trial补m2..m4共6项，并由公共verify_preparation验证完整10项后写正式preparation。随后运行同目录audit-ready.py核验资源与源码，再更新README、单独提交本题资产、报告证据并删除本编号。若公共身份再变化，先按变动影响复核；不能跳过断言或修改旧证据来强行通过。
