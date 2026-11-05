# 任务09 安装实例去重与冲突

2026-11-06，macOS；状态：已完成。

依赖：已核对正式vw-agent-apps、vw-api-types安装实例DTO，以及任务01–08代码和持久证据。任务08提交1408b49f6。RustCodeGraph核对来源enumerate_tool_installations、is_conflicting、infer_install_source_for_install及infer_install_source；未修改upstream、只读plan.md或前序任务。

公开入口为`vw_agent_apps::apps::discovery::report::{build, requires_upgrade_confirmation, is_conflicting}`。既有lib.rs公开apps/discovery，仅添加领域模块声明。build接收授权执行主机已探测、已解析真实路径的AppInstallation，不自行在客户端文件系统解析远端路径，也不新增RPC或权限。后续版本探测/API任务消费此报告。

- 报告限定单个应用、目标主机和环境（含WSL发行版），混合范围明确拒绝；入口与真实路径均要求绝对路径，拒绝NUL、相对路径和未解析父目录。
- 按真实完整路径去重，缺失真实路径时按完整入口去重；同名不同目录仍为两处。Windows按目标语法统一斜线和ASCII大小写，POSIX保留大小写，去除重复分隔符与`.`；不以basename或版本折叠。
- 别名汇合保留PATH默认入口、对应ID，并按默认优先稳定排序。已知来源保留；Unknown按来源路径规则推断，真实Cellar/Caskroom优先识别Homebrew。未识别路径继续Unknown，避免暗示可用安装配方。
- 两处及以上即需升级确认；只有版本Option分歧或Runnable/非Runnable混合才为冲突。全部未知版本或同版本同运行状态不冲突；已知/未知版本混合仍与来源规则一致。
- 聚合仅处理DTO，不读写安装或执行命令；真实临时文件与符号链接测试确认内容、链接及目录项均不变。Windows规则在macOS运行，不代表Windows实机验证；无UI改动。

Cargo阶段原子领取共享槽位2：

- `CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`
- `CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`，覆盖`.cargo/config.toml`的build.build-dir，全部Cargo进程继承两者。

验证：

1. 红：`cargo test -p vw-agent-apps --lib cc_apps_installation_conflicts -- --nocapture`，Cargo退出101；4个有效测试全部失败，46过滤。可编译空实现未去重、不校验范围、不报告版本冲突，断言失败符合预期，非工具或依赖错误。
2. `cargo fmt --all`，退出0；审查无无关格式化差异。
3. 绿：同一聚焦命令，退出0；首轮4通过。补充运行状态独立阈值和真实临时符号链接只读用例后再次fmt及复验，6通过、0失败、46过滤。
4. `cargo clippy --all-targets --all-features -- -D warnings`，退出0；首轮及最终复验均无警告。静态检查无行为测试计数。
5. `git diff --check`及提交前暂存差异检查，退出0。仅提交本领域实现、独立测试、模块声明及本文，前序编号文件删除不暂存。

全部Cargo及产物子进程结束后释放自己的slot-2锁，缓存保留。下一步任务10继续版本探测；完整应用实机验收由后续任务负责。
