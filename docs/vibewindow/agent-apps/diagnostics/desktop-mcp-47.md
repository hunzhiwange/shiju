# 桌面 MCP 检测与取消（任务 47）

2026-11-12，macOS 27.0 / arm64，vw-desktop、vw-gateway-client、vw-api-types 0.6.0。直接在原仓库实施，没有创建工作树；只读 plan.md 未修改。

## 真实入口与边界

已核对本计划来源映射、任务 1–3 的《高可用诊断契约》以及任务 39 的 [SDK 证据](sdk-39.md)。现有 `GatewayClient::diagnostic_start/status/cancel`、诊断路由、授权执行端 `agent_apps/mcp/network_probe.rs::start_handshakes` 和母任务 11 的受管目录列表均已存在。本任务消费这些产物，没有重建配置管理、授权或 MCP 执行器。

- `components/system_settings_mcp.rs::view` 在现有 MCP 设置页接入 `agent_apps_diagnostics/mcp_panel.rs::view`；目录来自 `mcp_management_list::visible`，应用选择来自已连接主机的应用列表。
- `message/agent_apps_diagnostics.rs::update` 通过任务 39 SDK 提交单项/批量服务器 ID、查询和取消。复用当前项目 ID、主机、应用成员令牌、租户及网关传输上下文。提交失败不自动重发；网络失败使用 toast，提供继续查询入口，不展示原始错误载荷。
- `message/agent_apps_diagnostics/mcp_state.rs::McpState::accept` 按服务器 ID 归位结果，核对网关、租户、项目、主机、应用、凭据代次、目录修订、任务身份、提交代次及轮询序号；旧响应不能覆盖当前状态。查询与取消竞争时只接受最新轮询序号，避免多条轮询链。
- 逐项显示排队、握手、成功、失败、取消或未验证。取消请求先显示等待清理，收到服务端 `cancelled` 才确认终态；已有终态拒绝迟到响应。任务完成但缺少详细握手证据仍是未验证。
- 仅调用诊断资源，不写目录、不修改原生文件、不挂载生产连接、不执行工具。服务器限制仍由既有执行端负责。配置或凭据改变后，旧结果被隔离，不扩大授权范围。
- 新控件使用统一 `settings_select`、`rounded_action_btn_style`、显式按钮内边距和既有设置页滚动布局，没有新增滚动条。动态名称和结果均使用默认字体；结果组件 `mcp_results.rs::view` 经实际 tiny-skia 排版、绘制和像素生成，覆盖中文、emoji、明暗主题。

## 测试入口更正

桌面已拆为库 crate，`main.rs` 使用 `vw_desktop::app`。原卡片命令：

```bash
cargo test -p vw-desktop --bin shi-ju diagnostic_desktop_mcp_results_keep_server_identity -- --nocapture
```

退出 0，但 **0 passed、31 filtered**，不构成完成证据。按调度补充指令，改为生产模块实际所属的库测试：

```bash
cargo test -p vw-desktop --lib diagnostic_desktop_mcp_results_keep_server_identity -- --nocapture
```

独立测试文件仍为 `components/agent_apps_diagnostics/mcp_panel_tests.rs`，通过生产 `mcp_panel` 模块的 `#[cfg(test)]` 显式注册。测试直接调用同一生产状态、SDK 提交函数与渲染组件。未向二进制复制 app 或生产模块，也未增加对外测试支持接口。

## 自动验证

实际动态领取槽位 **7**，每阶段使用原子 `mkdir target/rust-slot-locks/slot-7.lock`，记录任务与 PID。全部 Cargo 命令继承：

```text
CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-7
CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-7
CARGO_INCREMENTAL=0
CARGO_BUILD_JOBS=1
CARGO_PROFILE_TEST_DEBUG=0
CARGO_PROFILE_DEV_DEBUG=0
CARGO_PROFILE_TEST_CODEGEN_UNITS=256
CARGO_PROFILE_TEST_LTO=off
CARGO_PROFILE_TEST_OPT_LEVEL=0
```

| 检查 | 退出码与有效数量 | 观察 |
| --- | --- | --- |
| 原 `--bin shi-ju` 聚焦命令 | 0 / 0 项 | 入口已迁移，不计通过 |
| 首次库测试接线 | 101 / 0 项 | 本任务结果组件泛型生命周期及测试缺 Renderer trait 导入，已修复；不计红灯 |
| 上述 `--lib` 聚焦命令，红灯 | 101 / 1 项 | `MCP results must be joined by server ID`；实际 Queued、预期 Succeeded；0 passed、1 failed、9363 filtered |
| 同一 `--lib` 聚焦命令，绿灯 | 0 / 1 项 | 1 passed、0 failed、9363 filtered；测试 0.18 秒，构建 2 分 05 秒 |
| `cargo fmt --all -- --emit stdout` | 0 / 不适用 | 只输出格式化结果；遵守并行约束，仅用 rustfmt 写回本任务新增 Rust 文件，未回写全仓格式化 |
| `cargo clippy --all-targets --all-features -- -D warnings`，首次 | 101 / 不执行测试 | 外部 `dispatch.rs` 取消令牌借用/移动、`failover_execution.rs` 两处 ProviderId 类型错误；对应会话已修复 |
| `cargo clippy -p vw-desktop --all-targets --all-features -- -D warnings` | 101 / 不执行测试 | 同批新增 MCP 导入弹窗缺 close_message，导入视图对 ProjectId 使用 Display；对应会话已修复 |
| `cargo clippy --all-targets --all-features -- -D warnings`，第一次重试 | 101 / 不执行测试 | 仍报告上述导入变更错误；复核最新源码后确认已修复 |
| `cargo clippy --all-targets --all-features -- -D warnings`，最终 | **0** / 不执行测试 | 全 workspace、全 targets、全 features 通过；3 分 06 秒 |
| `git diff --check` | 0 / 不适用 | 通过 |

绿灯包含：乱序结果归位；跨主机、凭据代次、目录修订、任务 ID、提交代次和轮询序号的迟到结果拒绝；本地取消不冒充清理完成；取消终态不回退；缺少证据不误报成功；逐项握手失败不因任务结束变成功。离屏渲染真实生成非空像素缓冲，覆盖明暗主题及中文动态文本。

隔离本地 HTTP 协议服务实际接受并检查 **5 次请求**：批量启动、状态查询、取消、终态查询、单项启动；启动正文保留对应服务器 ID，取消发往原任务，成员令牌头保持原值。服务线程完成并 join。这里只替代网关边界，属于受控证据；不声称真实 MCP 供应商或账号验收，也不替代执行端已有握手/子进程回收测试。本任务未拥有或改动生产连接。

链接器报告 `__eh_frame section too large` 非致命提示，测试仍退出 0。没有修改 Cargo 配置、缩减 features 或执行全量测试、wasm、Makefile。

## 交接

状态：**已完成**。聚焦测试 1 项通过，完整 Clippy 退出 0。未修改上述外部错误所属代码，只在确认其所有者已修复、源码发生变化后重试。所有本任务槽位锁均已释放，缓存保留；共享父模块仅提交本任务入口，其他会话修改和暂存保持原样。

推荐下一步：用户按母计划进行 macOS 原生窗口、真实账号与实际服务器的人工验收。
