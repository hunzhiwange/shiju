# 任务36：终端启动API

状态：已完成。2026-11-06，macOS。

## 范围与真实依赖

已核对任务35设置API、任务33发现读取、任务28输入验证和29 POSIX终端边界。复用可信管理员授权文件、TenantScope、LocalReadEnvironment、validate_launch_request、GatewaySettingsStore和launch_posix_terminal。plan.md、来源与接口.md只读；已有编号任务删除不纳入提交。RustCodeGraph status/query确认现有符号；新处理器未在索引覆盖，直接读取实现。

生产handler_router经agent_apps::router合并POST /agent-apps/launch?targetHostId=…，继承既有外层管理API认证。独立管理员grant.launch默认false；读、设置写或生命周期权限不隐式获得启动权限。每次重新读取可信文件，按租户及目标主机授权，远程绑定明确501，离线503。cwd仅允许管理员配置的host.home及其子目录，规范化后检查，拒绝相对路径和控制字符；未新增任意项目授权或主机RPC。

请求接收既有LaunchRequest的appId、installationId、cwd、args、skipPermissions，以及可选providerId/accountId；拒绝未知字段与客户端command/program/env。供应商/账号所有者已有独立进程材料化接口，但尚无能在终端中安全保活、回收凭据的会话接口，因此有上述ID或skipPermissions时明确501，且在发现探测/终端动作之前拒绝；不会回退默认账号或伪造支持。POSIX普通终端路径使用主机发现的实例、绝对程序路径、校验后的参数和固定/bin/sh；主机终端偏好从任务35相同数据库命名空间读取。非Unix执行入口明确501，不把跨平台规则夹具视作Windows实机证据。

成功只返回既有LaunchResult的installationId；拒绝及终端错误均固定HTTP状态，无命令、路径、子进程输出或凭据。没有UI变更。实际OS终端动作由既有适配器负责；本任务用允许替代的OS终端边界夹具记录校验结果，不宣称打开真实终端窗口。

## 验证

红灯使用最小可编译未实现骨架返回503，1个有效行为用例失败（cwd应422却503），不是编译错误或零测试；随后启用真实授权、验证、存储和终端边界。

实际红灯、通过、最终回归阶段均动态领取slot-2：

- CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2
- CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2

| 确切命令 | 退出码 | 有效测试数 |
| --- | --- | --- |
| `cargo fmt --all`（红灯、通过阶段各一次） | 0 | 不适用 |
| `cargo test -p vw-agent --lib cc_apps_launch_api_target_scope -- --nocapture`（红灯） | 101 | 1失败，11462过滤 |
| `cargo test -p vw-agent --lib cc_apps_launch_api_target_scope -- --nocapture`（通过） | 0 | 1通过，11462过滤 |
| `cargo test -p vw-agent --lib cc_apps_read_api_authorization -- --nocapture` | 0 | 7通过，11456过滤，另1个路由子进程不重复计数 |
| `cargo test -p vw-agent --lib cc_apps_lifecycle_api_scope -- --nocapture` | 0 | 5通过，11458过滤 |
| `cargo test -p vw-agent --lib cc_apps_preferences_api_scope -- --nocapture` | 0 | 2通过，11461过滤 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 全仓通过 |
| `git diff --check` | 0 | 不适用 |

当前聚焦用例使用真实临时文件、codex版本可执行脚本、可信权限文件、SQLite及HTTP Router，验证host.home外cwd、伪造实例、权限绕过args、客户端command、错误账号/供应商ID、错误令牌与撤销launch权限均拒绝且终端副作用文件不存在；终端失败返回固定503且不产生成功文件；成功边界收到主机偏好及校验后的args，响应精确等于installationId。14个相邻回归用例通过，共15个顶层有效通过用例。

既有macOS链接器__eh_frame提示与任务35一致，测试退出0，不修改链接设置。所有Cargo动态mkdir领取共享槽位；CARGO_TARGET_DIR与CARGO_BUILD_BUILD_DIR同时指向领取槽位的绝对路径，覆盖仓库build.build-dir=target。EXIT/INT/TERM清理本人owner与锁，保留缓存。未运行无过滤全量测试、fmt检查、wasm检查或Makefile。

差异审查仅本任务处理器、测试、模块连线、独立launch授权与本文档；无无关格式化变更。共享槽位已释放、缓存保留。下一步任务37起SDK接入；供应商/账号终端会话需其所有者提供保活与回收接口后再收口。
