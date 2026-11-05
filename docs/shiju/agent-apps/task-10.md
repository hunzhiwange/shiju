# 任务10 本地版本与运行诊断

2026-11-06，macOS；状态：已完成。

依赖：核对来源快照、正式 vw-agent-apps 与安装实例 DTO，以及任务01–09真实产物和持久证据，任务09提交 f88c4bfda。RustCodeGraph 查询 probe_local_version、CommandDeadline 及调用者；不修改 upstream、只读 plan.md、前序任务或客户端接口。

公开入口：`vw_agent_apps::apps::discovery::version::probe_local_version`。既有 lib.rs 已公开 apps/discovery，仅新增最小领域模块声明。输入是授权执行主机已发现的绝对路径和单实例截止时间，固定执行 `--version`，不接受客户端命令、参数或 shell。输出 VersionProbe 独立保留 installed、version、runtime_status，供后续版本/API任务使用。新增 DTO 失败原因 Missing、PermissionDenied、DamagedInstallation、OutputTooLarge，避免通过错误字符串反推状态。

- 已存在入口（含失效符号链接）探测失败仍 installed=true；确实缺失为 false；权限不足、解释器缺失、超时、输出超限、无有效 stdout 版本分别报告。
- 非零退出保留实际退出码且不采信任何版本；信号终止报告损坏。stdout 严格 UTF-8、提取数字点分版本；stderr 丢弃，不暴露原始诊断载荷或把 stderr 数字当成功。
- stdout 非阻塞轮询，总输出限16 KiB；等待覆盖输出与进程结束；新建独立进程组，所有返回路径清理子进程及组内后代并 wait。真实临时脚本验证超时后的延迟写入没有发生。无安装、升级或用户配置写入。
- 本任务实现 POSIX 本地主机执行，macOS 真实进程夹具验收；不宣称 Windows/WSL 实机执行，已有跨平台路径规则不变。无 UI 变更。

Cargo阶段动态原子领取共享槽位2：

- `CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`
- `CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`，覆盖 `.cargo/config.toml` 的 build.build-dir，所有 Cargo/子进程继承。

验证证据：

1. 红：`cargo test -p vw-agent-apps --lib cc_apps_version_probe_broken_install -- --nocapture`，退出101，3个有效测试全部断言失败、52过滤；可编译空实现错误返回 installed=false，非环境或依赖失败。
2. `cargo fmt --all`，退出0，排除无关格式化差异。
3. 首轮实现后同一聚焦命令退出101，3通过、1失败。macOS 对无 shebang 文本的 Command 执行回退 shell 并真实退出127；调整损坏夹具为不存在的 shebang 解释器，明确覆盖 spawn 损坏而不伪造退出行为。
4. 补充输出限额、信号终止和后代副作用用例；再次 `cargo fmt --all` 退出0。最终同一聚焦命令退出0，5通过、0失败、52过滤；耗时0.49秒。
5. `cargo clippy --all-targets --all-features -- -D warnings`：退出0，40.48秒，无警告。
6. `git diff --check` 退出0，审查变更仅本领域实现、独立测试、模块声明、DTO原因及本文；前序编号文件删除不暂存。

全部 Cargo 与产物进程退出后释放本任务 slot-2 锁，保留缓存。下一步任务11继续版本领域；逐应用真实安装验收归后续任务。
