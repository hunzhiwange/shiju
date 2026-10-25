# 评测 Gateway 启动核验

任务19只核验无凭据启动与系统权限边界，不提交模型请求。依赖为任务12的
`isolation-check.md` 和任务18冻结的 `accuracy-manifest.json`。

## 执行

从仓库根目录运行（macOS、APFS、系统 Seatbelt、Python 3 与 Cargo）：

```sh
python3 evals/promptfoo/gateway-preflight.py
```

包装动态原子领取 `target/rust-slot-locks/slot-1.lock` 至 `slot-10.lock`，记录持有者，
将 `CARGO_TARGET_DIR` 设置为仓库内对应槽位的绝对路径，然后执行：

```sh
cargo build -p vw-cli --bin shiju
```

不改变默认 features。构建前后核对所有已跟踪及未忽略 Rust/Cargo 文件摘要；
源码变化则拒绝启动。记录 HEAD、dirty 文件清单、差异摘要与二进制 SHA-256，
因此本地未提交改动包含在运行版本中。证据不是干净发布版本的证明。

已有暖缓存时 `--reuse-build` 可跳过构建，仅用于调查；正式取证使用上述默认命令，
保证源码摘要覆盖本次构建。APFS 克隆产生独立二进制 inode，避免重复复制大型调试产物。

## 端口与生命周期

42617 是其他服务的端口，本包装不连接或停止该服务。使用内核分配的空闲 loopback
端口，打印实际 endpoint，并保存到忽略目录
`evals/promptfoo/results/accuracy-baseline/environment.json`。释放探测 socket 后若
发生端口竞争，Gateway 退出或 health 超时都会使核验失败，不抢占服务。

启动命令完整记录为 `launchCommand`，形式为：

```sh
/usr/bin/sandbox-exec -f <监督端生成的gateway.sb> <独占目录中的shiju> gateway --host 127.0.0.1 --port <实际端口>
```

监督端轮询 `GET <endpoint>/v1/health`，再通过 `X-Platform-ID: local`、
`X-Company-ID: local` 和精确 workRoot 读取配置与工具清单。不传租户头会收到400，
不能当作模型或权限故障。

成功、失败及 SIGINT/SIGTERM 均进入清理：终止自建进程组、等待退出、核实端口关闭，
清理独占运行目录并释放自己的槽位锁；保留 Cargo 缓存。进程组仍存在时保留锁并失败。
不可捕获的 SIGKILL 或系统崩溃后，先核实 owner PID、Gateway 与全部后代已退出，
才可清理该遗留锁。不得删除其他任务的锁、进程或缓存。

后续任务必须重新领取、启动、核验、停止；本次 endpoint 在结束后已关闭。
后续监督端从其本次 `environment.json.endpoint` 设置 Provider 的 base URL，
不能把历史端口当作长期服务，也不能跨会话持有本次槽位。

## 系统边界与准入范围

- 使用系统 `sandbox-exec` 从 Gateway 进程入口施加默认拒绝策略，后代 exec/fork
  保持同一策略；不依赖 Shell 工具内部的沙箱开关。
- 只开放精确 workRoot、独占运行状态目录的读写；二进制仅可读/执行。
  只读系统库、系统命令及必要设备；不开放整个 `/System`、HOME、仓库或公共 tmp。
  元数据读取也仅限显式系统路径及运行目录祖先。
- 不开放 mach IPC、外部网络、其他本地监听端口、宿主凭据、Docker socket、
  仓库中的 fixture 源、私有答案、verifier、保留集或监督证据。
  sysctl读取仅限硬件与系统版本，不开放任意进程参数查询。
- 环境白名单由监督端构造，HOME/TMPDIR/配置目录均指向独占空状态；不继承宿主
  token、代理或模型配置。初始化后将本次 local/local 配置设置为 `readonly`、
  `workspace_only=true`、空命令/额外根列表及启用 Bash 检查，并读取确认。
- 收集整个注册工具清单和配置摘要，包含 AgentTool、MCP、线程、批处理与后台能力；
  这些入口均不额外获得文件、IPC或网络授权。清单存在不代表逐个工具功能已验收。

13项受控 canary：允许根读写2项、禁止绝对路径读写2项、符号链接读写2项、
后代读取、仓库读取、监督证据读取、二进制写入、Data volume别名读取、
脱离会话的后代读取、对真实监督端监听器的网络连接拒绝各1项。
禁止范围测试只写可销毁 canary，不触碰宿主真实凭据。

`environment.json.startupAdmission` 仅证明该次无凭据启动环境，绑定配置/工具/
二进制/策略摘要、UID、平台、workRoot realpath/dev/ino、允许写目录和时间。
进程退出、目录删除后即不可复用。`dynamicTaskAdmission=false`，不将其传给 Provider
冒充真实题目的 admission。模型标识为本次空状态默认值，health不证明模型可用。

任务20及后续动态评测须在各自独占 workRoot、实际模型配置、精确网络路径和实际
工具能力下重新验证并产生新证明；需要模型连接时，应设计可核实的受限通道，不能
简单开放全网或导入宿主凭据。实际运行还需监督超时、后代退出与 settlement，
不能只复用本次预检的进程组检查推断任意后台任务均已回收。

## 2026-10-25 验证摘要

默认 features 构建退出0；最终 health HTTP200；canary 13/13；无模型请求。
实际独立 endpoint、PID、时间、二进制及所有路径仅存 results 忽略目录。
最终构建/启动使用槽位1；早期冷构建使用槽位2，均已释放，缓存保留。
`cargo clippy --all-targets --all-features -- -D warnings` 使用槽位1，退出0。
没有修改 Rust，没有运行全量测试、Makefile、wasm 检查或 fmt --check。

调查中先后修正了缺失租户头导致的400、`read_only`枚举拼写（正确为`readonly`），
以及全量二进制副本耗尽空间问题；最终通过证据、原始错误与输出保存在 results。
这些失败不能计为有效安全通过，也没有据此放宽隔离边界。
本次同一预检实验包含10次受控启动迭代，另一次在复制二进制时失败且没有启动；
仅最终完整核验计为本任务通过，逐次摘要见忽略目录中的 `iterations.json`。
