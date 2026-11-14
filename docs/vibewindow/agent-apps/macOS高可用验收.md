# macOS 无窗口高可用验收

2026-11-14：已补齐受控验收代码，尚未执行。遵循本轮指令，仅静态阅读与编辑；未运行 Cargo、格式化、单元测试、实机测试或回归。没有退出码、有效测试数、网络实测计数或通过结论，未领取构建槽位，缓存路径不适用。

入口：`crates/vw-agent/src/gateway/api/handlers/agent_apps_ha_acceptance_tests.rs` 的 `ha_macos_headless_routing_acceptance`，由父模块以 macOS/test 条件注册。

场景使用现有 `spawn_gateway` 启动独立网关子进程，不创建桌面窗口；通过生产 HTTP 管理 API 和 SDK 读取队列，使用真实 SQLite 配置、供应商事务与长期路由。仅上游故障受控：P1 使用本地 TLS 失败端点，P2 使用本地真实 HTTP 服务。断言包括：

- API 发布路由、配置队列、启用故障转移，SDK 读取相同队列。
- P1 连接失败后 P2 成功，观测实际路由与真实连接/HTTP 请求计数，客户端重试头不能扩大预算。
- P2 随后失败，两个熔断器均打开；下一请求不增加上游计数。
- 通过策略更新将冷却时间设为零，下一生产请求执行半开探测并恢复关闭状态；不使用手动健康重置代替恢复。
- 禁用保留队列，切换不改写原生配置；子进程由现有 Drop 路径 kill/wait，夹具任务取消并等待退出。

上述是待执行断言，不是已观测结果。测试执行时输出 OS、架构、包版本及受控计数。真实服务商、账号及物理窗口关闭均未验收。

直接依赖静态核对：任务23的运行时切换与事件见 `agent_apps_failover_runtime_tests.rs`；任务42的桌面队列入口见 `agent_apps_high_availability/mod.rs`；任务43策略表单位于 `policy_modal.rs`，任务44健康及实际路由展示位于 `health.rs` 和 `message/agent_apps_health.rs`。桌面不拥有本场景路由调度。

后续统一验证应通过仓库 `scripts/cargo_slot.py` 执行 `cargo test -j 1 -p vw-agent --lib ha_macos_headless_routing_acceptance -- --nocapture`，记录实际槽位、绝对缓存路径、退出码、非零有效数量和实际计数。复用既有 `ha_runtime_uses_single_retry_owner` 及领域 replay/official/tool 保护用例补充边界回归；本场景不声称已验证流重放、官方账号及工具副作用。

用户人工待验：按母计划 `人工验收.md` 关闭真实桌面窗口后确认网关继续服务，核对实际供应商及账号隔离。此项与受控自动场景分别记录，不以自动夹具代替实机结果。
