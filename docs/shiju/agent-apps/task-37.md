# 任务37：读取诊断SDK

状态：已完成。2026-11-06，macOS。

## 范围及依赖

已读取只读plan.md、来源与接口.md及任务36证据，核对任务33真实生产路由、授权上下文和vw-api-types共享AppId、AppCatalogResponse、AppReadResponse。母计划旧状态标签不代表符号缺失。RustCodeGraph status/explore检查索引；新增及未覆盖文件直接读取。已有编号任务删除不纳入提交。

新增GatewayClient的agent_apps_catalog、agent_apps_installations、agent_apps_versions、agent_apps_diagnostics；固定GET /v1/agent-apps/{catalog,installations,versions,diagnostics}，传递targetHostId及适用appId。方法消费共享DTO，通过既有请求上下文发送网关Bearer认证与TenantScope，不添加客户端发现、安装配方或主机RPC。上述方法是后续40读取界面的SDK入口，本任务没有UI变更。

公开AgentAppsReadError区分Transport、Decode和Http；Http保留状态及完整JSON结构，非JSON及空响应保留字符串。服务端当前授权错误为空正文，不能臆造业务错误码；JSON错误固件同时验证未来/外层结构化错误无损返回。AppId通过共享serde拼写编码，targetHostId通过reqwest query编码；不将ID拼入路径。

## 验证证据

所有阶段原子mkdir依次尝试共享槽位，实际均领取slot-2：

- CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2
- CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2

第二项覆盖.cargo/config.toml的build.build-dir=target，防止中间产物逃离槽位。各阶段结束后清理自己的owner及锁，保留缓存；最终验证脚本设置EXIT/INT/TERM清理。

| 确切命令 | 退出码 | 有效测试数量 |
| --- | --- | --- |
| `cargo fmt --all`（红灯、实现、最终导出修正后） | 0 | 不适用 |
| `cargo test -p vw-gateway-client --lib cc_apps_read_sdk_roundtrip -- --nocapture`（红灯） | 101 | 3失败，279过滤 |
| `cargo test -p vw-gateway-client --lib cc_apps_read_sdk_roundtrip -- --nocapture`（实现及最终导出修正后） | 0 | 每次3通过，279过滤 |
| `cargo clippy --all-targets --all-features -- -D warnings`（首次） | 101 | 发现本任务错误类型缺少crate根导出，已修复 |
| `cargo clippy --all-targets --all-features -- -D warnings`（最终） | 0 | 全仓通过 |
| `git diff --check` | 0 | 不适用 |

红灯使用最小可编译未实现骨架返回Transport("not implemented")，三例实际行为断言失败，不使用编译错误或零测试代替红灯。通过阶段使用真实回环HTTP监听器捕获请求，验证四个端点类型化结果、中文桌面路径、claude-desktop拼写、包含中文和空格及/?&#%+的目标主机query、GET无body、Bearer及两个租户头。拒绝场景覆盖400/401/403/404/501/503保留嵌套错误JSON、503空正文、成功但缺失字段的响应和关闭监听器后的真实连接失败。不可达直接返回Transport，无本地扫描路径。

差异审查只包含SDK模块、独立测试、模块登记及crate根错误类型导出、本文档；未保留无关格式化变更。未执行全量cargo test、fmt检查、wasm检查或Makefile。下一步任务38生命周期SDK。
