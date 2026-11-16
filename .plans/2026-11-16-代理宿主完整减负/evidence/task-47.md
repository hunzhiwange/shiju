# 任务47 静态迁移证据

代码完成，编译/行为未验证。本轮遵守 plan.md 禁止 Cargo、测试、构建与实机验证的约束，仅阅读源码与核对差异。未宣称性能或构建收益。

## 全文件与用例映射

来源 transport_tests.rs 原399行、5条用例。以下4条完整迁入 model-relay-http/src/host_migration_47_tests.rs（177行），由 lib.rs cfg(test) 自注册，断言及测试名保持：

- acp_model_source_messages_selects_the_official_alibaba_token_plan_endpoint → model_protocol::endpoint_urls::official_alibaba_messages_url。
- acp_model_source_messages_selects_the_official_deepseek_native_endpoint → model_protocol::endpoint_urls::official_deepseek_messages_url。
- acp_model_source_messages_chat_matches_the_client_thinking_mode → model_protocol::messages_thinking_config::configure_disabled；辅助 chat_model 一并迁移，类型使用 agent_provider::provider。
- acp_model_source_relay_chat_oauth_expiry_and_revocation_never_use_cached_keys → model_transport::credential::credential；使用 agent-auth 实际认证类型，保留过期、撤销、更新密钥全部断言。

第5条 model_protocol_012_relay_native_conversion_and_credential_isolation 整条保留于宿主 transport_tests.rs（225行）。包括 Restore::drop 恢复真实配置目录、RestoreKey::drop 恢复系统及 agent_env 密钥；不把真实环境端口模拟成领域授权验收。该测试实际调用 project::from_directory、config::update_project_for_tenant、RelayServer::start 的 Context 适配及 ProviderRelayBackend 的租户/项目检查，验证真实存储配置经过宿主装配的全路径。原生/转换、模型作用域、越权端点、认证头隔离、metadata 不访问上游及取消断言全部保留。没有把宿主 App 或上下文复制到领域库。

upstream.rs 原1行：删除文件与 mod，公共 upstream_http_client 直接重导出 model_transport 入口；session/llm/acp/model_source 的调用保持兼容。
xai_oauth_transport.rs 原1行、xai_responses.rs 原2行：全仓引用静态搜索确认仅模块注册/自身转发，删除文件与 mod。实际实现原已属于 model-transport 和 model-protocol，无生产迁移收益。

transport.rs 删除仅供迁出测试使用的 credential、configure_messages_chat 转发；仍保留其现有生产端口。旧 transport_tests 挂载保留，只承接上述真实宿主集成测试，不再挂载4条领域用例。

## 方向、规模与不足

生产路径：宿主 ProviderRelayBackend 验证租户/项目后调用 model_transport::provider_transport::ProviderTransport::execute；后者调用 credential 与 endpoint_urls。领域测试直接调用相同入口，没有反向依赖宿主或 path/include 宿主源码。configure_disabled 的生产实现已在 model-protocol，本轮仅迁其验证归属。

本任务四个来源物理源码403→225行，净减178行（44.17%），未达到70%。按职责分项：宿主测试/夹具399→225，生产算法0→0，兼容转发4→0；领域新增测试/夹具177行，领域生产算法新增0行。父模块减少3个mod并直接重导出upstream客户端；transport.rs另减少4行测试转发。无新增生产算法。

逐文件保留原因：仅 transport_tests.rs 保留225行，职责为真实租户、配置目录、项目上下文与网络中继装配集成。继续可迁项：集成用例内的协议结果断言可以在后续明确拆分整条集成职责时抽至领域，但本任务要求整个测试函数作为移动单位；不删除断言凑比例。减量不足交收口验收，不能计为已达到整轮门槛。

Cargo.toml 添加仅测试所需 agent-auth、agent-provider、model-reasoning、chrono，Cargo.lock 同步这些依赖；复用 model-protocol、model-transport 等已有依赖。任务46并发注册和依赖保持。

## 未来验证（全部未运行）

- `python3 scripts/cargo_slot.py cargo test -p model-relay-http --lib host_migration_47_tests -- --test-threads=1`：静态预期4条，必须实际非零且全部通过。
- `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib model_relay::transport_tests::model_protocol_012_relay_native_conversion_and_credential_isolation -- --exact --test-threads=1`：静态预期1条。
- `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib model_relay::transport_tests -- --list`：预期只列上述宿主集成测试，4条迁出名称不再出现；不把零测试当通过。

任务1证据读取未发现本文件族额外跨目录测试挂载。源码检查确认领域函数公开，类型来自既有领域定义；未运行任何编译、行为、格式化或实机检查。

并发说明：共享父模块/清单的本任务部分改动已由并发提交103918d99包含；本提交按最新HEAD去重，仅补齐未提交部分。
