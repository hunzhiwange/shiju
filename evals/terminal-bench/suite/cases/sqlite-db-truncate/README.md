# sqlite-db-truncate 诊断题

四个独立题目均使用合成 SQLite 数据，保留完整父题在忽略目录中的官方文件。这里不含官方数据库、答案或模型轨迹。

| 子题 | 主要能力 | 三种错误控制 |
| --- | --- | --- |
| m1 | 表叶页、单元偏移、记录头与序列类型 | 空输出、页号从零计、漏末记录 |
| m2 | 解码完整记录字段 | 空输出、整数按无符号解码、漏末记录 |
| m3 | 九字节 varint、有符号整数与类型边界 | 无符号整数、varint 仅八字节、交换常量 0/1 |
| m4 | 生成严格且完整的数据列表 | 空输出、无符号整数、漏末记录 |

每题提交标准库 Python 程序 `solution.py INPUT_DB OUTPUT_JSON`。checker 实际运行公开输入和第二份布局不同的隐藏合成输入，严格比较 JSON 键、顺序、数值类型，拒绝重复键、非有限数值和输入修改。输入有 512 字节页，无 overflow payload，末尾移除三字节；含 UTF-8、BLOB、NULL、实数、正负整数和九字节 rowid。仅恢复完整单元，不要求通用 SQLite 修复。完整父题的输入没有 SQLite 文件头；这些保留文件头的合成窄题不覆盖该损坏形态，须以父题官方校准另行证明。

`build_assets.py` 从插入的行值独立建立期望字段，并从页布局计算 m1 元数据；恢复程序不作为字段答案来源。使用 `python3 build_assets.py` 可重建素材；SQLite 版本变化可能改变布局，重建后必须重新校准。

公开环境只注入 `trunc.db`；checker、隐藏数据库、期望值和 gold 不进入 Agent 环境。验证期运行提交程序，每次最多 5 秒、输出至多 100KB，程序至多 64KB。外层容器禁网并限制 CPU/内存；Python `-I` 不是安全沙箱，checker 本身不提供对恶意 verifier 进程的文件系统权限隔离。

宿主回归：

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/sqlite-db-truncate/tests -p test_cases.py
```

Linux 校准：

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/sqlite-db-truncate/case.json --run-fixtures
```

`validation.json` 绑定 20 项 Linux fixture 控制及全部声明素材哈希。完整父题和四子题的 fresh oracle/nop 控制另由标准 SuiteDockerEnvironment 记录于 `preparation.json`；这些控制均不调用付费模型。

完整父题的官方 uv 安装地址在原容器内返回 403。`environment/verifier-bootstrap.sh` 只在 Agent 结束后的 verifier 阶段注入原版 uv 0.9.5 安装脚本、发行归档和固定版本 pytest wheel 缓存；哈希与来源由忽略目录下的 runtime 配置绑定。官方安装脚本和 `test_outputs.py` 保持原字节。curl 包装器仅将两个精确的安装器/归档请求映射到真实缓存，其他请求调用系统 curl；apt 包装器调用真实 apt 并保留失败状态，以 30 秒限制重复网络安装。所有步骤保留在原 900 秒评分预算内，不预装到 Agent 环境。

## 2026-10-30 校准结果

- 宿主 unittest：22项通过；Linux fixture：20项符合预期（4 gold通过、4 nop拒绝、12错误实现拒绝）。
- 当前公共执行身份下10个fresh标准控制全部符合预期：父题及m1–m4均oracle=1、nop=0；完整父题每次执行官方1项测试，无skip。
- 父/子基础镜像均实际离线执行冻结x86 ELF `shiju --help`，exit0；`verify_preparation`通过。
- 父题原镜像manifest：`sha256:aabac93c93bd1f310e6a6fb893911d7735026ed18491c72133c9196a09092ca4`，amd64，压缩层44164774字节，完整解压tar125051904字节；导入后config/RootFS与Registry逐项一致。
- 父题主容器1CPU/2048MiB，子题1CPU/512MiB、egress sidecar 0.25CPU/64MiB；隔离、容量监控及原评分日志保留在忽略结果目录。准备名额的0.25CPU/64MiB额外容器已释放。
- `cargo clippy --all-targets --all-features -- -D warnings` exit0；动态槽位1，`CARGO_TARGET_DIR`和`CARGO_BUILD_BUILD_DIR`均为仓库绝对路径下`target/rust-slot-1`，`CARGO_BUILD_JOBS=1`；没有Rust行为测试，槽锁已释放。

标准控制直接调用公共 `harbor_trial` / `SuiteDockerEnvironment`，最终收据为 `preparation.json`。最初公共CLI在旧资源锁下的启动失败单独归档；公共锁修复改变执行身份后，最终10项均完整重新执行，没有混入历史控制。最终证据位于 `evals/promptfoo/results/qwen38-suite/diagnostics/sqlite-db-truncate/` 的 `standard-preflight-lockfix/`、`acceptance-current.json`、`runtime-help-current.json` 与 `clippy-current.json`。没有调用付费模型。
