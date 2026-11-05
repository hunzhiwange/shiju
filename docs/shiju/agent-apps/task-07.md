# 任务07 WSL环境与Shell解析

状态：已完成。2026-11-06；macOS运行跨平台规则测试，不是Windows/WSL实机执行证据。

已读执行技能、plan.md、来源与接口、任务06证据；确认前序正式crate、实例与discovery模块已交付。通过RustCodeGraph核对wsl_distro_for_tool、wsl_distro_from_path、Shell校验与自动Shell调用链。来源快照与plan.md未修改。

新增独立wsl模块：解析授权执行主机配置根的wsl$/wsl.localhost UNC与扩展UNC，只绑定路径中的指定发行版；拒绝本地/其他服务器路径、空/超长/注入发行版，额外拒绝首字符非字母数字以阻止选项及点目录。Shell仅接受sh/bash/zsh/fish/dash裸名或干净绝对POSIX路径，输出批准的basename；拒绝相对/遍历/命令注入。参数仅允许-c/-lc/-lic，保留自动Shell与覆盖参数、显式选择与各Shell默认参数。resolve_target在非Windows主机返回Unsupported，绝不降级本机。parse_target仅做纯规则解析，可跨平台验证；不启动进程、不新增RPC、权限或配置。

验证阶段动态原子领取槽位2：
- CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2
- CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2，覆盖.cargo/config.toml的build.build-dir，防止中间产物逃逸。

验证命令及证据：
- 红：`cargo test -p vw-agent-apps --lib cc_apps_wsl_target_resolution -- --nocapture`，退出101；4个有效测试，2失败/2通过，空实现违反发行版与Shell结果断言（非依赖/工具错误）。
- `cargo fmt --all`退出0，审查无无关格式化差异。
- 绿：`cargo test -p vw-agent-apps --lib cc_apps_wsl_target_resolution -- --nocapture`，退出0；4/4通过，38过滤。
- `cargo clippy --all-targets --all-features -- -D warnings`，退出0，40.34秒，无警告。
- `git diff --check`退出0。审查模块声明、规则实现及独立测试，排除前序编号任务删除。

验证结束后释放自己的槽位锁，保留编译缓存。下一步任务08；实际WSL进程启动留给后续执行/终端任务，本任务没有Windows实机结论。
