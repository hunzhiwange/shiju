# 任务39：设置启动SDK

状态：已完成。2026-11-06，macOS。

## 依赖与边界

已读取 do-task-plans、git-commit、仓库 RustCodeGraph 技能、AGENTS.md、只读 plan.md、来源与接口.md、任务36/38证据、共享 preferences/launch DTO、实际启动处理器及相邻SDK与HTTP夹具。RustCodeGraph status/explore确认前序符号；新SDK未索引部分直接读取。依赖批次38已交付，母计划旧标签不等于接口缺失。plan.md保持只读。

新增 GatewayClient::agent_apps_preferences、agent_apps_preferences_update、agent_apps_launch，分别对应 GET/PATCH /v1/agent-apps/preferences 与 POST /v1/agent-apps/launch，供后续设置/终端交互任务43/44调用。读取/更新复用 AppPreferencesResult/AppPreferencesPatch；启动复用 LaunchRequest/LaunchResult，私有序列化包装只增加服务端已有的 providerId/accountId，可选ID不携带凭据或环境材料。当前服务端对此类身份启动和skipPermissions明确501；SDK不伪造支持或回退其他身份。

主机选择通过reqwest query编码，原样交给网关。cwd、实例ID、参数不做本地canonicalize、探测、执行或路径转换；字符串中的URL也不触发浏览器，未添加剪贴板或系统集成入口。配置更新不在SDK重启应用，原样返回restartRequired。无UI变更，不需UI渲染证据；没有真实安装、OS终端、浏览器或剪贴板动作。

AgentAppsSettingsError区分Transport、Http、Decode；Http保留HTTP状态及JSON/原始文本或空正文。复用原生dynamic_host_client禁用自动重试和重定向，响应丢失不会重复启动。Wasm明确拒绝缺少该传输保障的请求，沿用任务38边界。

## 验证证据

动态原子mkdir实际领取槽位2，所有Cargo及其子进程继承：

- CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2
- CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2

后者覆盖仓库build.build-dir=target，保证中间产物也在槽位内。仅释放本人owner及锁，保留缓存。

| 确切命令 | 退出码 | 有效测试数量 |
| --- | --- | --- |
| `cargo fmt --all`（红灯与绿灯前） | 0 | 不适用 |
| `cargo test -p vw-gateway-client --lib cc_apps_settings_launch_sdk_roundtrip -- --nocapture`（红灯） | 101 | 3失败，286过滤 |
| `cargo test -p vw-gateway-client --lib cc_apps_settings_launch_sdk_roundtrip -- --nocapture`（绿灯） | 0 | 4通过，286过滤 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 全仓通过，1m04s |
| `git diff --check` | 0 | 不适用 |

红灯使用可编译骨架Transport("not implemented")，实际成功/HTTP/Decode断言失败，不以编译错误或零测试冒充证据。绿灯4项使用真实回环HTTP/TCP，覆盖读取、主机域PATCH、启动成功、远端Windows路径含中文且本机不存在、原样argv含shell元字符和URL、特殊字符主机编码、Bearer与两个租户头、provider/account ID透传及请求无token/env字段；403/409/422/501/503/307完整错误、无效成功响应Decode、空501正文。真实TCP读取完整启动正文后丢弃响应或返回带Location的307，观察窗口内均只收到一次写入，分别返回Transport或Http错误。

差异只包括设置启动SDK、独立测试、模块/错误导出和本文；既有编号任务删除不纳入提交，无无关格式化差异。未运行无过滤全量测试、fmt检查、wasm检查或Makefile。下一步任务40及后续Iced接入。

全部验证完成，槽位2本人锁已释放、缓存保留。
