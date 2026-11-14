# API 与 SDK 统一验证交接

2026-11-14，任务23。静态审查基线 `f2a96de3a`。用户本轮明确要求只写代码，禁止 Cargo 相关命令、构建、单元测试、实机测试与最终回归；因此本文是后续验证清单，不是回归通过报告。未领取构建槽位，没有退出码或有效测试数可报告。任务卡按用户要求删除，不表示原计划的运行验收已完成。

## 当前静态结论

任务22已删除来源快照，完整许可与 MCP golden TOML 已保留，见 [snapshot-removal.md](snapshot-removal.md)。当前 crates/scripts 的 Rust、配置和脚本引用搜索未发现指向已删除 `vw-agent-apps/upstream` 的运行或测试引用；剩余 `upstream-hashes.json`、`upstream_snapshot_matrix.rs` 属其他仍存在的证据或测试路径。MCP `src/mcp/adapters/codex_tests.rs` 改为从本包 `tests/fixtures/mcp/codex-config.toml` 读取。该检查不能代替编译器依赖图检查。

本任务没有新增或修改生产行为，也没有新增测试。计划中的 `crates/vw-agent/tests/cc_convergence_*.rs` 当前不存在：各领域主要采用已有唯一所有者的零删除路径，本轮保留已有行为测试入口，不创建复制生产实现的测试。不能将这些不存在的测试记作通过或静默跳过。

## SDK 测试的覆盖层次

当前 `vw-gateway-client/src/client/mod.rs` 注册了以下四个测试模块。后续可按包、lib目标和各模块名过滤运行；本轮仅查看源码，未运行任何一组。

|模块过滤条件|静态观察到的断言范围|
|---|---|
|`client::agent_apps_api_tests::`|custom-icons 读写路径、租户头、中文载荷、错误传播、写操作不重放|
|`client::agent_apps_read_api_tests::`|应用目录读取、查询编码、凭据作用域、错误及非法成功载荷|
|`client::apps_usage_read_api_tests::`|用量读取、设置与模板、参数及结果校验|
|`client::apps_usage_write_api_tests::`|用量设置/刷新/取消/脚本试运行的请求和返回校验|

这些 SDK 测试通过 `test_support` 的本机 HTTP 固件调用真实 `GatewayClient`，不复制业务实现，但固件只记录请求并返回预设响应，**不是完整生产网关和存储验收**。不能单独用它们证明持久化、租户隔离或取消上游任务。

已有生产路由覆盖可与 SDK 测试互补：`crates/vw-agent/src/gateway/api/handlers/app_providers_tests.rs` 的 `cc_apps_provider_launch_isolation_sdk_http_sqlite_projection_process_roundtrip` 使用 SDK、本机 HTTP、handler_router 与临时数据库；`apps_usage_acceptance_tests.rs` 的 `sdk` helper 将 SDK 连接到传入的 Axum 路由，并使用隔离 fixture/授权策略；`mcp_management_create_tests.rs` 通过 Router oneshot 触发真实处理器。这些源码存在并不等于测试已执行或通过，也不能自动扩大为完整启动入口覆盖。

## 逐域待验证项

下列证据绑定真实 API、SDK、所有者及已有测试入口；不按文件名相似性合并不同作用域。

|领域证据|统一验证时必须观察的结果|
|---|---|
|[providers.md](providers.md)|SDK写入、重读及重启一致，应用供应商与推理供应商保留各自职责，越界无落盘|
|[models.md](models.md)|目录解析与能力/元数据同源，覆盖错误及账号作用域|
|[accounts.md](accounts.md)|凭据刷新和租约仅作用于授权账号，不泄漏敏感值，失败无第二刷新副作用|
|[config.md](config.md)|原生配置单事务提交，预像冲突、失败回退、跨主机拒绝|
|[mcp.md](mcp.md)|目录CRUD、授权分发/绑定及运行时检查保持不同职责；迁移后的Codex夹具消费者仍可编译并保持原字节语义|
|[skills.md](skills.md)|安装成功、失败回退、取消/进度、仓库授权及ACP挂载隔离|
|[prompts.md](prompts.md)|保存/删除/恢复的CAS、激活与文件投影事务，越界不写文件|
|[sessions.md](sessions.md)|目录去重、浏览/恢复不改原历史，维护操作授权及失败恢复|
|[usage.md](usage.md)|同请求只入账一次、原生与路由日志重叠、价格同源、跨租户拒绝|
|[database.md](database.md)|同路径工厂复用，租户键隔离，持久化重启读取|
|[network.md](network.md)|普通与受控客户端权限差异、代理/编码、流式超时、取消与无重放|
|[scheduling.md](scheduling.md)|新改动优先：WebDAV/S3 store 在加密落盘前拒绝 auto_sync=true；API授权后返回422；保存/清除后等待同目标同transport stop，无业务锁等待环、无跨作用域停止|
|[events.md](events.md)|已提交revision才发布、SSE作用域校验及取消；Iced当前刷新查询不得误称已订阅SSE|
|[installation.md](installation.md)|授权预览/执行/取消与任务终态；未知写结果不重放；桌面自身升级与应用安装仍分开|

[relay.md](relay.md) 另需覆盖 JSON/SSE、令牌撤销、项目/账号代次隔离及取消后停止上游访问。后台自动上传启动仍不支持，缺少显式持久化的对象授权选择、执行目标与远端基线合同；本轮安全拒绝不是该功能已交付的证据。

## 后续执行与恢复

统一验证在最新共享代码上按仓库槽位入口串行进行 Rust 重型编译，逐包/目标/有效过滤条件记录命令、退出码与实际运行数；零测试不算通过。优先覆盖本轮行为修改、迁移夹具和源码移除后的依赖可达性，再补上述 API/SDK 与作用域行为。不得使用真实用户目录或凭据。实机验证仍由用户安排，不能由静态结论推断 macOS 已验收。

本轮只运行 RustCodeGraph status/explore 和文件/引用读取：索引仍含已删来源，删除后的结论以当前文件静态搜索为准。未同步索引、运行应用或测试。仅本文的差异空白检查可作为文档检查结果。恢复本任务只需撤销本文，不重置其他代码或工作区已有修改。
