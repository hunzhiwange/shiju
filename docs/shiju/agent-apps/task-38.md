# 任务38：生命周期SDK

状态：已完成。2026-11-06，macOS。

## 依赖与范围

已读取 do-task-plans、仓库 AGENTS.md、只读 plan.md、来源与接口.md、任务37证据与任务34实际生命周期路由及共享 actions DTO。RustCodeGraph status/explore 已执行；新文件与索引未覆盖内容直接读取。前序真实接口已交付，不以母计划旧状态标签推断缺失。plan.md 未修改。

新增 GatewayClient 类型化方法：agent_apps_action_preview、agent_apps_action_submit、agent_apps_operation、agent_apps_operation_events、agent_apps_operation_cancel，供后续41/42生命周期界面调用。使用既有 /v1/agent-apps/actions/preview、/actions、/operations/{id} 与 /operations/{id}/events；事件沿用服务端25秒长轮询快照协议，不伪造SSE。预览返回 previewId，提交只发送该引用及 confirmed；操作ID由提交生成，此后查询、事件与取消均使用该ID。预览契约本身没有 operationId，客户端不臆造或替换。

复用已有禁止重试及重定向的原生 dynamic_host_client，携带网关认证、租户头和编码后的 targetHostId；操作ID作为单一路径段编码。调用者在事件断线后查询同一操作再继续长轮询；SDK没有自动重提安装、后台重连或额外调度器。Wasm 浏览器无法使用该原生传输安全约束，明确返回不支持错误，不静默采用可重定向写请求的普通客户端。

AgentAppsLifecycleError 区分 Transport/Http/Decode；Http 保留状态及完整JSON或原始非JSON/空正文。当前服务端部分拒绝仅返回状态，SDK不臆造细分业务码；结构化 conflict、plan_expired、node_insufficient 等由固件验证无损传递。取消完成语义仍由服务端负责。没有UI变更或真实安装副作用。

## 验证证据

原子 mkdir 依次尝试，实际领取共享槽位2；所有 Cargo 命令继承：

- CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2
- CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2

后者覆盖 .cargo/config.toml 的 build.build-dir，防止中间产物逃离槽位。构建阶段结束释放自己的 owner/锁，保留缓存。

| 确切命令 | 退出码 | 有效数量 |
| --- | --- | --- |
| `cargo fmt --all` | 0 | 不适用 |
| `cargo test -p vw-gateway-client --lib cc_apps_lifecycle_sdk_roundtrip -- --nocapture`（红灯） | 101 | 2失败，282过滤 |
| `cargo test -p vw-gateway-client --lib cc_apps_lifecycle_sdk_roundtrip -- --nocapture`（最终） | 0 | 4通过，282过滤 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 全仓通过，1m04s |
| `git diff --check` | 0 | 不适用 |

红灯使用可编译骨架返回 Transport("not implemented")，两项真实结果断言失败。首次编译发现 TaskId 无 Display，改用共享字符串字段后才取得上述有效红灯，不将编译错误当行为红灯。首次绿灯阶段新加真实套接字夹具因macOS接受连接继承非阻塞状态失败，显式切回阻塞读取后通过，没有修改生产重试策略来绕过测试。

4项最终用例通过真实回环HTTP验证：预览/提交引用与类型化响应、特殊字符主机与操作ID编码、Bearer和两个租户头、查询/事件/取消不携带写正文、事件503后GET恢复同一ID再继续订阅、终态取消、409冲突和计划过期及422 Node不足/307/503完整错误、无效成功响应Decode及409空正文。额外真实TCP夹具在读取完整提交正文后丢弃响应，或返回带Location的307，观察窗口内各只有一次实际写入；无自动重试或重定向。

差异只含生命周期SDK、独立测试、模块与crate根错误导出及本文。未纳入既有编号任务删除，未执行无过滤全量测试、fmt检查、wasm检查或Makefile。下一步任务39设置与启动SDK。
