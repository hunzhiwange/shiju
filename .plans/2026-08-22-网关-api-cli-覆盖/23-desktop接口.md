# 任务 23: 覆盖 Desktop 专用接口

批次：【批次 23】 依赖批次 22

状态：未开始

目的：提供 desktop preferences、tool content、desktop skills、external apps/path/service 和 project preferences 的显式 CLI。

来源任务：无

预计会话范围：覆盖所有 `/v1/desktop/*` 路由；宿主 open/reveal/service 命令保留但采用双重目标确认，不扩展任意进程执行。

## 文件

- 修改：`crates/vw-gateway-client/src/client/desktop_settings_api.rs`
- 测试：`crates/vw-gateway-client/src/client/desktop_settings_api_tests.rs`
- 修改：`crates/vw-cli/src/handlers/api/mod.rs`
- 新建：`crates/vw-cli/src/handlers/api/desktop.rs`
- 测试：`crates/vw-cli/src/handlers/api/desktop_tests.rs`

## 上下文

- 所有业务请求经 `GatewayClient`，复杂 body 从共享 JSON 输入读取，输出走共享 renderer。
- 该接口族需顺序注册到 `handlers/api/mod.rs`，不得修改 `plan.md`。
- 保留当前工作树中与本任务无关的 SHIJU 和桌面改动。
- desktop API 会在 Gateway 宿主执行 open/reveal/service 副作用，CLI 必须打印远端 endpoint 与精确目标。
- service command 只能接受服务端已支持的枚举，不得变成任意字符串命令执行器。
- desktop skill CRUD 与普通 skills 路由语义独立，命令名必须保留 desktop 前缀。

## 测试计划

- 行为：Desktop 命令准确区分宿主 endpoint，所有写入/打开/服务动作无确认时零请求，service 不接受未知命令。
- 失败验证测试：`crates/vw-cli/src/handlers/api/desktop_tests.rs` 中的 `api_desktop_commands_cover_preferences_skills_and_guarded_host_actions`
- 失败验证命令：`cargo test -p vw-cli api_desktop_commands_cover_preferences_skills_and_guarded_host_actions --lib`
- 预期失败原因：Desktop client 虽存在，CLI 无宿主目标确认和完整命令映射。
- 通过验证命令：`cargo test -p vw-cli api_desktop_commands_cover_preferences_skills_and_guarded_host_actions --lib`
- 模拟策略：本地服务器验证 endpoint/path/body 与请求计数；不实际打开应用、Finder 或服务。

## 步骤

1. 先写 preferences、tool content、desktop skills、external apps、reveal、service、project preferences 失败测试。
2. 补齐 client 缺失方法并为 service command 使用封闭枚举。
3. 实现 `api desktop`，帮助文本明确副作用发生在远程 Gateway 主机。
4. 所有 PATCH/PUT/POST 及宿主动作确认；未知服务命令解析失败。
5. 运行聚焦测试与 clippy。

## 验证

- 运行：`cargo test -p vw-cli api_desktop_commands_cover_preferences_skills_and_guarded_host_actions --lib`
- 运行：`cargo test -p vw-gateway-client desktop_client_uses_all_desktop_routes --lib`
- 运行：`cargo clippy --all-targets --all-features -- -D warnings`
- 预期：Desktop 路由、封闭枚举、远端目标提示和确认断言通过，clippy 通过。
- 所需证据：失败时命令缺失；通过后宿主动作拒绝分支零请求、未知命令解析失败；测试 0 失败；clippy 0 警告。

## 完成

`api desktop` 覆盖专用接口，同时对远程宿主副作用维持默认拒绝。

