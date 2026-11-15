# 任务 141: 网关路由 session-1

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次 1（依赖层级，无整批屏障）】 产物依赖：无；写入顺序：无

历史审计编号：原任务 334，仅用于对照 coverage.md、runtime-audit.md。

## 调度依据

- 无本计划内已确认前置；核对已有基线与本卡实际导出后即可进入就绪队列，不等待其它领域。
- 公共根清单、父模块、全局消息/状态和直接调用者由协调者逐项接线；公共文件冲突只串行化该次写入，不阻塞无关领域实现。
- 旧路径引用复核线索（不是迁移前置）：任务 1 的 `crate::project::instance::provide`；任务 6 的 `crate::storage::list_strict`；任务 18 的 `crate::provider::provider::invalidate_cache`；任务 38 的 `crate::worktree::environment::ensure_ready`；任务 63 的 `crate::session::agent_binding::save_scoped`；任务 65 的 `crate::agent::catalog::merged_agent_catalog`；任务 168 的 `crate::worktree`；任务 182 的 `crate::session::compaction_checkpoint::save`；任务 217 的 `crate::session::llm_raw::LlmRawArtifact`；任务 254 的 `crate::session::store::LlmRawStepKey`；任务 263 的 `crate::gateway::chat::invalidate_session_query_engine`；任务 269 的 `crate::session::ui_store::current_tenant_scope`；任务 278 的 `crate::gateway::instance::InstanceQuery`；任务 303 的 `crate::session::runtime::reserve_session`；任务 305 的 `crate::gateway`；任务 306 的 `crate::session::llm::recovery_test_support::RecoveryConfig::new`；任务 307 的 `crate::session::session::get`；任务 312 的 `crate::session`；任务 315 的 `crate::agent::get`。已有真实实现/窄适配可保持调用；只有必须消费未交付新接口时，记录符号证据后增加局部依赖。
- 执行时滚动调度，编号仅提供可行线性顺序；只等待本卡明确列出的前置完成并集成。在途旧编号按任务编号对照认领，禁止重复执行。

状态：已阻塞

目的：迁移，将本任务职责归入 `gateway-session`。

来源任务：用户要求尽可能全部拆分vw-agent；覆盖映射见coverage.md。

预计会话范围：一个领域的一组源码迁移；大领域已拆成连续任务，同包逐项接线。测试源码和资源仅随实现迁移，不执行。

## 并行实施边界

- 按 plan.md 的并行协议执行：独立工作树负责领域实现，主协调会话负责公共接线与逐项合入；禁止在同一检出目录同时执行并行任务。
- 本任务只修改下方来源文件、对应目标包及本编号任务记录。根清单、vw-agent 清单/lib.rs、清单外父模块和直接调用者由协调会话独占修改。
- 开始前检查当前基线的 use/super/宏及具体符号所有者。候选引用不是完整依赖图；发现需要另一在途任务的新接口时，报告具体符号，协调者将依赖方顺延，不猜签名或新造端口。
- 将公共接线逐文件列入本任务的“接线交接”记录，给出原/新导入、feature/cfg、注入位置和需删除的旧实现；不得仅交付泛化建议。
- 领域差异静态审阅后用 $git-commit 提交本工作树变更，保留本卡，报告提交与接线清单。此时仅为待合入产物，不标记已完成。协调会话接线、合入并复核后提交删除本卡，再归档已合入工作树。

## 文件

- 目标：`crates/libs/gateway-session/Cargo.toml`、`crates/libs/gateway-session/src/lib.rs` 及保持原职责布局的子模块；包已存在则修改。
- 公共接线（仅协调会话修改，本任务提供精确交接）：根 `Cargo.toml`、`crates/vw-agent/Cargo.toml`、`crates/vw-agent/src/lib.rs`、相关原mod.rs与搜索确认的直接调用者清单。
- 来源及相邻测试/资源：
- `crates/vw-agent/src/gateway/api/handlers/session/acp_options.rs`
- `crates/vw-agent/src/gateway/api/handlers/session/acp_source_tests.rs`
- `crates/vw-agent/src/gateway/api/handlers/session/agent_profile_acceptance_tests.rs`
- `crates/vw-agent/src/gateway/api/handlers/session/archived_delete.rs`
- `crates/vw-agent/src/gateway/api/handlers/session/archived_delete_tests.rs`
- `crates/vw-agent/src/gateway/api/handlers/session/fork_acceptance_tests.rs`
- `crates/vw-agent/src/gateway/api/handlers/session/fork_acp_tests.rs`
- `crates/vw-agent/src/gateway/api/handlers/session/message_ops.rs`
- `crates/vw-agent/src/gateway/api/handlers/session/message_ops_tests.rs`
- `crates/vw-agent/src/gateway/api/handlers/session/raw_commit.rs`
- `crates/vw-agent/src/gateway/api/handlers/session/raw_commit_tests.rs`
- `crates/vw-agent/src/gateway/api/handlers/session/session_ops.rs`
- `crates/vw-agent/src/gateway/api/handlers/session/session_ops_tests.rs`
- `crates/vw-agent/src/gateway/api/handlers/session/shared.rs`
- `crates/vw-agent/src/gateway/api/handlers/session/shared_tests.rs`

## 上下文

- 重点符号：`get`、`set`、`set_source`、`source_capabilities`、`delete`、`delete_order`、`session_message_list`、`session_message_get`、`session_message_part_delete`、`session_message_part_patch`、`RawCommitClaims`、`PendingRawCommitRegistry`、`register`、`claim`、`acknowledge`、`ui_session_list`、`ui_session_status`、`ui_session_get`。
- 当前文本引用候选：`session::ui_store`、`gateway::instance`、`gateway`、`session::agent_binding`、`session::llm`、`session::runtime`、`session`、`gateway::chat`、`global_database::run_isolated`、`project`、`session::message`、`session::fork_execution`、`agent::get`、`project::instance`。这里只是导航证据，注释/测试引用不能作为真实依赖；实际执行须追踪use别名、super和调用者。
- 领域边界：业务服务先下沉；handler按领域独立路由包，通过 gateway-types 共享状态接口；router/bootstrap最后组合。 仅依赖 gateway-types 与领域服务；不得依赖 agent-gateway 的router/bootstrap或其它handler包。

## 步骤

1. 读取上述源码、父模块声明、相邻测试和调用者；用 `rg -n 'crate::|super::|pub\(crate\)|include!' crates/vw-agent/src` 限定到本任务查看实际依赖，记录旧路径到目标模块映射。测试文件单独排程时按其生产实现实际所有者迁入，不为测试再建包。
2. 按领域边界迁移本任务源码；剥离应用宿主动作到已存在的窄接口，更新资源路径与必要可见性。禁止新增未调用抽象，不合并行为不同的旧实现。
3. 建立/扩展目标最小清单，复用已拆出的真实所有者；完整转发原feature/cfg。将调用者正向接线和旧位置薄重导出列入交接，由协调会话实施。后续任务尚未迁的调用者通过真实宿主适配接线，不让新包依赖它。
4. 逐项审阅参数/错误/异步取消、租户、权限、全局状态、序列化与平台语义等价；仅调整测试源码导入，不运行或追加测试。将尚需闭环迁移的宿主适配具体路径写入本任务提交正文。
5. 按并行实施边界提交领域变更与接线交接，保留本卡供协调会话合入后收尾；不得提前宣称任务完成。

## 静态验收

- 运行：`git diff --check`；逐文件阅读本任务完整差异和迁移后的清单。
- 运行：`rg -n 'vw-agent|vw_agent|crate::app' crates/libs/gateway-session`；审阅是否存在库对主包的反向依赖，文档与测试引用单独检查。
- 检查：本任务全部来源已迁/明确为组合转发，生产实现不重复；类型/状态只有一个所有者；工厂注册和资源完整；后续文件已分配所有者。
- 所需证据：真实迁移映射、最小依赖、窄端口调用者、cfg/feature、直接调用接线、测试/资源迁移清单及差异检查结果。仅静态证据，不能声称编译或行为通过。
- 待统一验收：本任务来源的既有测试、直接调用集成、原支持平台与特性、涉及UI的渲染和外部能力；将具体路径及组合写提交正文，本轮全部禁止运行。


## 2026-11-16 部分实施记录（未完成）

- 归档树验证与排序由 gateway-session/src/archived_delete.rs 持有，使用 session-info::Info；原 handler 薄重导出 delete_order。六个纯排序测试连同 fixture 迁往独立 archived_delete_tests.rs；原宿主文件保留全部 persistence 测试与断言。
- 原 raw_commit.rs 全部登记/声明/确认逻辑和单一全局 registry 迁至 gateway-session/src/raw_commit.rs；raw_commit_tests.rs 原样迁入。宿主只重导出 register/claim/acknowledge。依赖真实 session_port::llm_raw::LlmRawArtifact 和 session_store::LlmRawStepKey，不复制键校验、不复制全局状态；版本检查、租户/项目键、锁中毒处理保持。
- 目标新增 anyhow/session-info 与非 wasm session-port/session-store 依赖；未改公共清单/父模块/调用者，已有 gateway-session 依赖足够接线。
- 剩余 acp_options、message_ops、session_ops、shared 及 archived_delete HTTP 处理器仍在宿主，未实现完毕。运行时 create_session/load_session/reserve_session、ui_store 租户 task-local 与持久化、session::llm ACP 控制、gateway::chat 缓存仍需按实际签名接窄端口或真实领域服务。此项是剩余工作，不声称完成或接口不存在；此前 LlmRawStepKey 缺失已随真实 session-store 交付解除。
- 本轮保存部分产物，保留本卡；不删除或更改其它任务。
- 静态验证：定向 git diff --check、目标包反向依赖搜索、TOML 解析；未运行Cargo、单元测试及实机测试，待统一验收。未来覆盖原 raw_commit_tests.rs、归档排序与原宿主 persistence 测试，非 wasm 网关平台组合。
