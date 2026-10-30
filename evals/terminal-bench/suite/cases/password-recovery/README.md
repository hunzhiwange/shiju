# password-recovery 准备结果

四个独立合成诊断题：m1 识别删除目录记录；m2 按 extent 重组实际二进制片段；m3 过滤全字符串 ASCII 候选；m4 从 JSON 选择虚构目标并保存结果。每题均提交 `/app/solution.py`，checker 运行四组黑盒请求，检查实际输出与输入文件完整性。公开 fixture 不含 oracle 或隐藏断言。

2026-10-30 完成父题与四个子题的全部重校准。每题 fresh oracle=1、nop=0，共 10 个标准 Harbor 控制；全部首次评分、无 recovery、无付费模型调用。使用公共 `suite.scoring.harbor_trial` / `SuiteDockerEnvironment`，未修改上游题面、资源、超时、评分、镜像或模型预算。

主机 unittest 21 项通过；`diagnostics validate --run-fixtures` 20 项全部符合预期（4 gold、4 nop、12 负例）。历史先红记录：m3 normalize-case 错误实现曾被接受，补充独立小写诱饵后被拒绝，没有放宽判定。

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/password-recovery/tests -p test_cases.py
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/password-recovery/case.json --run-fixtures
```

`validation.json` 绑定诊断代码、契约、fixture、oracle、checker 与镜像身份；`preparation.json` 通过 `suite.preparation.verify_preparation`，绑定当前执行身份、冻结 runtime-v2、Harbor、只读官方源及 10 个控制的评分目录哈希。起始静默检查 120 秒，每个控制启动前与全部结束后核对执行身份。旧轮次因共享代码变更而失效的控制未复用。

执行身份：`588690e423e955cab7b2c335d302678cae81598b2ec421946065c88f964ce16c`。

父题原镜像 manifest：`sha256:15fcd84bf3b06270ee148211cc6da6599b13ec2f41ac1d0894cf75c2225872ee`，Linux amd64；压缩层 C=62,185,367 bytes、解压层归档 U=195,910,144 bytes，所有 layer digest、diff_id 与实际 RootFS 链一致。上轮导入已按 3C+正硬 Ucap(1GiB)+S10/B2/A1/余量5GiB 完成容量准入，本轮复用缓存镜像，无新增压缩副本或父镜像导入。缓存 Python base 的完整层归档 U 为 910,168,064 bytes；其 Docker inspect.Size 不是解压体积。本轮同样复用该 base，新增合成 context 与可写层另受运行预算监控。

实际资源：父题 main 1CPU/2048MiB，原 agent/verifier 各 900 秒；子题 main 1CPU/512MiB，sidecar 0.25CPU/64MiB。运行监控保存实际 host/VM 空间、自己容器 SizeRw 与证据归档大小，检查 S10/B2/A1/5GiB 预留；不宣称 Docker 已强制实施 storage_mb 磁盘配额。父镜像启动时 `/app` 只有原 varsea，无 `/tests`、`/oracle`、`/solution`。

本轮完整证据（校准入口、资源与容量、依赖清单及哈希、首轮评分、最终验证）位于：

`evals/promptfoo/results/qwen38-suite/diagnostics/password-recovery/resume-final-1793372160`

`calibrate-all.py` 逐个使用标准公共 Harbor 入口，先等待共享锁与资源空闲；剩余控制通过共享锁原子领取有限窗口，另预留 1.25CPU/576MiB（无网络、只读 root），实际资源连同 main、sidecar 与探测均计入 4CPU/8GiB 上限，校准结束后已释放；`verify-final.py` 核对完整 preparation、执行身份、资源与镜像。计划中的旧 runtime.json 为历史证明文件，缺少 historical_root；实际使用本题 container-resume/runtime.json 指向已验证的冻结 runtime-v2，未更改共享配置。旧基础设施失败和中断日志保留在 resume-20261030/；前一身份的通过控制保留在 resume-final-1793370214/，未用于当前 preparation。

Clippy 与 Promptfoo 配置检查均通过。Clippy 动态领取槽位 1，绝对 CARGO_TARGET_DIR / CARGO_BUILD_BUILD_DIR 均为 `/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1`，JOBS=1；增量缓存历史诊断重放后，使用 CARGO_INCREMENTAL=0 重跑干净通过，退出 0。自有锁释放、缓存保留。未修改 Rust 源码，未运行全量 Cargo 测试。
