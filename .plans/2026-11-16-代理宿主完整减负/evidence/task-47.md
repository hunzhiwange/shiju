# 任务47：兼容入口及残留专项复审

代码完成，编译/行为未验证。遵守本轮禁止 Cargo、构建、测试运行和实机验证的要求；仅核对源码、注册、依赖和差异。未宣称编译耗时收益。

## 完整映射与当前结果

原来源4文件403行，目前全部删除，宿主来源净减403行（100%）。首轮提交bf5fa2f5e迁移4条测试并删除3个转发文件；此次复审利用任务46随后建立的真实存储测试夹具迁移剩余整条测试，额外减少宿主225行及父模块2行挂载，不删除断言。

- transport_tests.rs 原399行、5条用例 → model-relay-http/src/host_migration_47_tests.rs 及其 native_http_tests.rs。源文件与旧mod全部删除。
- upstream.rs 原1行 → 直接从 model_transport 重导出 upstream_http_client，保留已有ACP客户端调用的公共兼容入口。
- xai_oauth_transport.rs 原1行 → 删除无调用转发及mod，真实所有者为model_transport。
- xai_responses.rs 原2行 → 删除无调用转发及mod，真实所有者为model_protocol。

逐用例映射（名称保留）：

1. acp_model_source_messages_selects_the_official_alibaba_token_plan_endpoint → host_migration_47_tests，同名；直接调用model_protocol::endpoint_urls::official_alibaba_messages_url。
2. acp_model_source_messages_selects_the_official_deepseek_native_endpoint → 同上，直接调用official_deepseek_messages_url。
3. acp_model_source_messages_chat_matches_the_client_thinking_mode → 同上，调用model_protocol::messages_thinking_config::configure_disabled；chat_model夹具同步迁移。
4. acp_model_source_relay_chat_oauth_expiry_and_revocation_never_use_cached_keys → 同上，调用model_transport::credential::credential；真实agent-auth类型及过期/撤销/更新密钥断言保留。
5. model_protocol_012_relay_native_conversion_and_credential_isolation → host_migration_47_tests::native_http_tests，同名。完整迁移225行测试（新增导入拆行后226行），包括Restore::drop、RestoreKey::drop、真实环境恢复、临时目录、真实项目及配置存储、上游监听、全部9种请求组合、metadata无上游访问、原生/转换协议、模型作用域、认证头隔离及取消断言。

第5条迁移静态逐字比较确认：除注释、导入、锁和领域API路径替换之外，函数体完全一致，断言未删除。crate::project原本直接重导出agent_project；crate::config::update_project_for_tenant原本直接重导出agent_config::runtime。迁入后直接调用相同实现。

## 真实调用、夹具与宿主边界

生产路径保持：宿主ProviderRelayBackend检查租户/项目后调用model_transport::provider_transport::ProviderTransport::execute，其调用真实模型目录、路由、认证和协议实现。本轮不将已有生产下沉计为新增成果，生产算法迁移0行；原兼容转发4行降为0行。

复用任务46的host_migration_46_tests::fixture，仅将模块和global_async_env_lock可见性扩大到pub(crate)以供本crate兄弟测试调用；不新增Host trait、公共生产端口、依赖或功能开关，不复制App/AppState。所有领域HTTP测试共享该锁及实际provider Host注册，真实读取项目配置、目录和用量账本。模型传输仍由真正ProviderTransport执行。

此领域夹具不证明宿主特有的当前租户权限判断。现存宿主server_tests.rs中的ExternalProvider::execute仍断言实际session::ui_store::current_tenant_scope及project::instance上下文与绑定一致，由acp_model_source_relay_binding_http_isolation_limits_and_lifecycle等宿主测试触发；本次不修改这些归其它任务所有的宿主薄边界测试，也不将夹具等同实际宿主验收。

首轮证据中“不能迁整条集成测试”和44.17%不足的结论由本次新增的可复用实际领域夹具条件替代。最终本任务来源没有保留文件或继续可迁项；其它宿主server_tests不是本任务来源，职责为实际宿主端口接线。

当前统计：宿主来源测试/夹具399→0，兼容转发4→0，领域测试/夹具新增177+226=403行（另有2行子模块注册）；领域生产算法新增0行。宿主父模块移除所有已迁出的挂载。任务1未发现此文件族额外跨目录挂载。

## 未来验证（全部未运行）

- `python3 scripts/cargo_slot.py cargo test -p model-relay-http --lib host_migration_47_tests -- --test-threads=1`：静态预期5条，必须非零且全部通过。
- `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib model_relay::transport_tests -- --list`：确认原5条名称不再挂载；此命令仅证明消失，不以零测试宣称行为通过。
- `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib model_relay::server_tests::acp_model_source_relay_binding_http_isolation_limits_and_lifecycle -- --exact --test-threads=1`：静态预期1条，用于实际宿主上下文接线验收。

本次没有执行以上命令，也没有运行任何Cargo、构建、格式化、单元测试或实机测试。未来须实际取得编译及行为证据。
