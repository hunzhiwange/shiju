# 任务13 npm安装动作

2026-11-06，macOS。状态：已完成。

核对总计划、来源与接口、任务12完成证据、正式AppId以及RustCodeGraph中上游npm_install_command_for/npm_install_extra_args与调用者。vw-agent-apps正式crate与前序发现、版本和Node门控已交付。此任务只提供后续生命周期可复用配方，不新增执行RPC或绕过执行主机授权。

交付 apps::lifecycle::npm::install_args(AppId)：仅接受强类型应用标识，返回不可修改的静态argv。十个包逐项保留来源映射；Claude/GrokBuild/OpenCode/Mcode分别保留ignore-scripts=false、optional和精确allow-scripts（Mcode包含better-sqlite3）；Dsh保留engine-strict。Mcode许可列表是一个argv，不含shell引号。ClaudeDesktop/Hermes/Agy/Omp明确返回错误；接口不接受任意包、版本、脚本或额外参数。调用方须把argv分别传给授权npm可执行文件，不能拼接shell源码。Windows/WSL复用结构化参数，不宣称对应平台实机安装完成。

验证阶段通过原子mkdir动态领取slot-2：
- CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2
- CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2，覆盖仓库build.build-dir。
- 红：cargo test -p vw-agent-apps --lib cc_apps_npm_install_allowlist -- --nocapture，退出101，3有效测试中1通过2失败，66过滤。可编译空实现返回Err，固定包和脚本参数断言失败，非底座/编译失败。
- cargo fmt --all，退出0；审查无无关格式化变更。
- 绿：cargo test -p vw-agent-apps --lib cc_apps_npm_install_allowlist -- --nocapture，退出0，3通过0失败，66过滤。
- cargo clippy --all-targets --all-features -- -D warnings，退出0，15.91秒，无警告。
- git diff --check及暂存差异检查，退出0。

测试覆盖全部十包、四组精确脚本许可、Dsh引擎强校验、其余五包没有额外参数、四个不支持应用的明确错误，以及任意包/脚本选项/shell注入/旧CLI标识不能反序列化为AppId。此阶段是纯配方测试，不启动npm，不修改个人安装或凭据；真实安装验收仍属后续应用实机任务。无UI变更。

Cargo进程已结束，释放自己的slot-2锁并保留缓存。既有任务1–12编号文件删除未纳入提交；plan.md与upstream均未改动。下一步任务14复用配方实现官方安装脚本。
