# 用量分析前置契约（任务 1）

2026-11-14 静态核对。仅记录支持后续实现的入口；未执行 Cargo、单元测试、实机测试或最终回归，未确认编译和 macOS 运行行为。共享工作区的未提交实现只代表当前源码存在，不表示已验收。

本任务无直接依赖。后续按编号卡和执行分线消费产物；来源覆盖.md 的“线性执行”和“1/母1—16”是旧编排，不能据此等待全部母能力。

| 前置 | 实际入口（仓库相对路径） | 可消费事实及缺口 |
| --- | --- | --- |
| 母1/2 业务包 | `crates/vw-agent-apps/src/lib.rs`、根 `Cargo.toml` | 已注册业务包；`usage/mod.rs` 当前导出余额、套餐、缓存与脚本，不是统一请求账本。后续在该模块增量接线，不能覆盖现有导出。 |
| 母2 认证与范围 | `crates/vw-agent/src/gateway/middleware.rs`、`gateway/api/handlers/agent_apps/access.rs`、`crates/vw-api-types/src/id.rs` | 复用 TenantScope/ProjectScope 和主机访问授权；统一用量动作、项目归属及目标主机绑定仍需对应 API 任务接线。请求参数不构成授权。 |
| 母2 持久化 | `crates/vw-agent/src/gateway/settings_store.rs`、`crates/vw-agent/src/agent_apps/{sqlite,postgres}.rs` | 既有 GlobalDatabase/设置存储可参考；统一账本、来源唯一约束和游标事务尚需任务3/12，设置 KV 不能代替账本事务。 |
| 母2/3 长任务与事件 | `crates/vw-agent-apps/src/apps/lifecycle/operations.rs`、`crates/vw-agent/src/gateway/api/handlers/agent_apps/lifecycle.rs` | Operation 状态、订阅和取消机制存在；生命周期路由绑定安装动作，不能直接宣称已承载用量任务。任务32/43/48负责接入和范围隔离。 |
| 母3/4 应用与供应商 | `crates/vw-agent-apps/src/apps/`、`providers/`、`crates/vw-agent/src/app_providers.rs` | 路径由执行主机应用实例解析，供应商从获准记录取得；无证据保留 unknown，不从模型名猜供应商。 |
| 母5 模型目录 | `crates/vw-agent-apps/src/models/mod.rs` | 目录归母5；本领域只负责价格选择与同步，不另建下载服务。 |
| 母6 账号 | `crates/vw-agent/src/agent_apps/auth_accounts.rs` | 账号/绑定/凭据归授权领域；账本不能暴露原始 token 或复制凭据。 |
| 母7 限额 | `docs/shiju/apps/usage/contract.md`、`crates/vw-agent-apps/src/usage/mod.rs` | 原生余额/套餐与 UsageCache 已有独立所有者；任务41消费限额，不与请求成本重复累加。 |
| 母8/9 路由 | `crates/vw-agent/src/app_routing/` | 请求身份、重试与终结事实须由任务29/30核对接入；现有路由目录不证明已入统一账本。 |
| 母14 会话 | `crates/vw-agent-apps/src/sessions/{mod,source,identity,catalog}.rs`、`providers/` | 当前共享区已有只读解析源码。SourceRegistry 按 ProjectScope/主机/实例/provider 绑定，拒绝请求显示路径；统一扫描的 token 投影、游标和去重未交付。 |
| 母15/16 项目 | `crates/vw-agent-apps/src/workspace_paths.rs` | 本地工作区路径接口存在；会话归属及组合上下文须由服务端验证后传给用量，不接受 query 冒充项目范围。 |

后续最小交付顺序以实际接口为准：DTO → 事务账本/游标 → 各扫描投影 → 请求身份汇合 → 调度/API/SDK/桌面。未交付项仅限制其消费者，不扩大本任务去补建母能力。

契约保持：Decimal 字符串金额、未知成本为空、reasoning 不重复加到 output、原生费用不被补价覆盖、UTC 存储与 IANA 日历范围、原始日志正文不出 API、通知合并不延迟落账。未证实的同请求关联只标疑似重复。归档保留防重放身份。
