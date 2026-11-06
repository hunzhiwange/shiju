# 任务27：应用设置归一化

2026-11-06，macOS。状态：已完成。

依赖：已读取只读plan.md、来源与接口.md、任务26证据，核对正式vw-agent-apps、共享AppId/目录及现有WSL解析器。依赖26已交付；母计划旧状态不代表接口缺失。使用仓库RustCodeGraph skill检查索引、orderAppIds、上游终端与可见性规则；新增模块及未覆盖内容使用rg/文件读取。未修改upstream或plan.md。

## 实现与边界

新增独立用户偏好、主机偏好、每CLI的WSL shell/flag及结果DTO，拒绝未知字段与未知可见性ID。用户默认14项全部可见；显式空可见集返回InvalidInput，重复可见项去重。排序过滤未知/重复字符串，按来源orderAppIds算法将缺项插回最近默认前驱之后；AppId序列来自既有目录，wire名称来自共享serde契约。

normalize_preferences为执行端纯业务入口，返回完整排序、可见性及规范化主机偏好；主机平台由授权调用者选定，不提供客户端自报主机执行接口。macOS/Windows/Linux终端按来源settings.rs白名单校验，非Windows拒绝WSL配置。复用现有WSL解析器校验shell路径、basename及-c/-lc/-lic，auto转None，显式shell补默认flag；拒绝重复应用和Claude Desktop的WSL设置。解析用固定UNC文本仅为复用纯解析规则，不是探测结果或实际执行目标。

新旧主机设置都经过同一校验和规范化后比较，主机启动偏好改变仅返回restartRequired提示；用户排序/可见性可即时消费，不触发重启提示。入口不含进程、客户端或网关重启调用，也不写存储。既有lib.rs已公开apps，新增apps模块声明即可完成库导出。任务35负责现有授权上下文中的GET/PATCH及持久化，当前未声称已交付HTTP/UI功能。Windows/WSL/Linux仅显式平台夹具规则证据，没有这些系统实机执行证据。

## Cargo槽位与验证

原子mkdir顺序尝试1–10，实际领取slot-2。全部Cargo阶段继承：

- CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2
- CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2

第二项覆盖.cargo/config.toml的build.build-dir=target，避免中间产物逃离槽位。锁记录计划、任务、会话和持有进程；全部验证及产物进程结束后释放自己的锁，保留缓存。

- 红灯：`cargo test -p vw-agent-apps --lib cc_apps_preferences_normalization -- --nocapture`，Cargo退出101，3个有效测试全部失败。最小可编译签名缺少默认可见项、空集拒绝及重启提示，真实断言失败；不是依赖或工具错误。
- `cargo fmt --all`，退出0，已审查无无关Rust格式化差异。
- 绿灯：`cargo test -p vw-agent-apps --lib cc_apps_preferences_normalization -- --nocapture`，退出0，4通过、0失败、117过滤。覆盖去重/未知过滤/前驱补位、默认14项、最后可见项拒绝、三平台终端正反例、非Windows WSL拒绝、WSL shell/flag非法输入、默认flag、auto、重复CLI/桌面拒绝、重复规范化与不变配置不提示重启。
- DTO：`cargo test -p vw-api-types --lib cc_apps_preferences_normalization -- --nocapture`，退出0，1通过、0失败、74过滤；序列化往返、未知字段和未知AppId拒绝。
- `cargo clippy --all-targets --all-features -- -D warnings`，退出0，无警告。
- `git diff --check`，退出0；审查仅本任务6个Rust文件和本证据纳入提交，已有编号1–26任务删除排除。

无UI改动，不修改HOME或个人安装。下一步任务28，设置API接线由任务35负责。
