# 当前消费者源码引用快照

逐行保留直接路径及聚合 use，执行时必须重搜别名和再导出传播；此表不是未来 API。

```text
crates/vw-provider-resolver/src/models_tests.rs:10:use vw_shared::provider::state::from_models_dev_model;
crates/vw-provider-resolver/src/models_tests.rs:639:        |endpoint| endpoint.protocol == vw_shared::provider::protocols::ModelProtocol::Messages
crates/vw-provider-resolver/src/remote_models.rs:88:    !value.is_empty() && value != vw_shared::auth::OAUTH_DUMMY_KEY
crates/vw-provider-resolver/src/provider_tests.rs:288:    let expected = vw_shared::provider::state::resolve_model_limit(None, None, None);
crates/vw-provider-resolver/src/provider_tests.rs:417:    assert_eq!(fallback.limit.context, vw_shared::provider::state::DEFAULT_MODEL_CONTEXT_LIMIT);
crates/vw-provider-resolver/src/provider_tests.rs:418:    assert_eq!(fallback.limit.input, Some(vw_shared::provider::state::DEFAULT_MODEL_INPUT_LIMIT));
crates/vw-provider-resolver/src/provider_tests.rs:419:    assert_eq!(fallback.limit.output, vw_shared::provider::state::DEFAULT_MODEL_OUTPUT_LIMIT);
crates/vw-provider-resolver/src/provider_tests.rs:891:            vw_shared::provider::reasoning::reasoning_levels(
crates/vw-provider-resolver/src/protocol_routes_tests.rs:2:use vw_shared::provider::protocols::*;
crates/vw-provider-resolver/src/protocol_matrix_tests.rs:4:use vw_shared::provider::protocols::{ModelProtocol as P, ProtocolAuthentication as A};
crates/vw-provider-resolver/src/auth.rs:9:pub use vw_shared::auth::Info;
crates/vw-provider-resolver/src/auth.rs:18:    vw_shared::auth::store::resolve_filepath(&paths.home, &paths.data)
crates/vw-provider-resolver/src/auth.rs:31:    vw_shared::auth::store::get_from(&filepath(), provider_id)
crates/vw-provider-resolver/src/auth.rs:38:    vw_shared::auth::store::all_from(&filepath())
crates/vw-provider-resolver/src/protocol_routes.rs:4:use vw_shared::provider::protocols::{ModelProtocol, ProtocolEndpoint};
crates/vw-provider-resolver/src/protocol_routes.rs:188:        vw_shared::provider::protocols::ProtocolAuthentication::Unsupported { .. }
crates/vw-provider-resolver/src/models.rs:26:pub use vw_shared::provider::models::{
crates/vw-provider-resolver/src/models.rs:31:pub use vw_shared::provider::reasoning::ReasoningOption;
crates/vw-provider-resolver/src/provider.rs:27:pub use vw_shared::provider::state::{
crates/vw-provider-resolver/src/provider.rs:31:pub use vw_shared::provider::types::{
crates/vw-provider-resolver/src/provider.rs:36:pub use vw_shared::provider::types::{parse_model, sort};
crates/vw-provider-resolver/src/provider.rs:309:            vw_shared::provider::state::catalog_reasoning_model_ids(&database);
crates/vw-provider-resolver/src/provider.rs:440:                    let limit = vw_shared::provider::state::resolve_model_limit_with_base(
crates/vw-provider-resolver/src/provider.rs:484:                vw_shared::provider::state::apply_api_overrides(
crates/vw-provider-resolver/src/provider.rs:524:                    (ProviderSource::Api, Some(vw_shared::auth::OAUTH_DUMMY_KEY.to_string()))
crates/vw-provider-resolver/src/provider.rs:527:                    (ProviderSource::Api, Some(vw_shared::auth::OAUTH_DUMMY_KEY.to_string()))
crates/vw-provider-resolver/src/protocol_catalog.rs:6:use vw_shared::provider::protocols::{ProtocolEndpoint, ProtocolModelScope, deserialize_protocols};
crates/vw-shared/src/id/mod.rs:24://! use vw_shared::id::{Prefix, ascending, descending, schema, timestamp};
crates/vw-agent/src/session/store/postgres.rs:10:use vw_shared::session::info::Info as AgentSessionInfo;
crates/vw-agent/src/session/store/postgres.rs:11:use vw_shared::session::ui_types::{
crates/vw-agent/src/session/store/postgres_tests.rs:6:use vw_shared::project::{Info as ProjectInfo, TimeInfo as ProjectTimeInfo};
crates/vw-agent/src/session/ui_types.rs:3:/// 重导出 vw_shared::session::ui_types::*，保持外部调用路径稳定。
crates/vw-agent/src/session/ui_types.rs:4:pub use vw_shared::session::ui_types::*;
crates/vw-agent/src/session/dynamic_tool_store.rs:155:        ToolState::Pending(state) => (state.input.clone(), vw_shared::time::now_ms()),
crates/vw-agent/src/session/dynamic_tool_store.rs:163:        time: PartTime { start, end: Some(vw_shared::time::now_ms()) },
crates/libs/desktop-lsp/src/install.rs:62:    let path = vw_shared::shell::resolve_executable(program)
crates/libs/desktop-lsp/src/install.rs:65:    vw_shared::shell::apply_augmented_path_tokio_command(&mut command);
crates/libs/desktop-lsp/src/lib.rs:166:        vw_shared::shell::apply_augmented_path_std_command(&mut process);
crates/libs/desktop-lsp/src/lib.rs:321:                .and_then(|program| vw_shared::shell::resolve_executable(&program))
crates/libs/desktop-lsp/src/lib.rs:326:                        .and_then(|program| vw_shared::shell::resolve_executable(program))
crates/libs/desktop-lsp/src/lib.rs:336:    let resolved = vw_shared::shell::resolve_executable(&command.program)
crates/libs/desktop-lsp/src/lib.rs:344:        vw_shared::shell::apply_augmented_path_std_command(&mut probe);
crates/vw-agent/src/session/title.rs:51:    use vw_shared::session::session_utils::strip_task_session_source_marker;
crates/vw-agent/src/session/ui_store.rs:28:use vw_shared::session::path;
crates/vw-agent/src/session/ui_store.rs:29:use vw_shared::session::ui_store;
crates/vw-agent/src/session/store_tests.rs:8:use vw_shared::project::{Info as ProjectInfo, TimeInfo as ProjectTimeInfo};
crates/vw-agent/src/session/store_tests.rs:9:use vw_shared::session::info::{
crates/vw-agent/src/session/store_tests.rs:12:use vw_shared::session::ui_types::{
crates/vw-agent/src/health/mod.rs:120:    let build_id = vw_shared::build_identity::current_executable_build_id()
crates/vw-agent/src/session/store.rs:7:use vw_shared::session::info::Info as AgentSessionInfo;
crates/vw-agent/src/session/store.rs:8:use vw_shared::session::ui_types::{
crates/vw-agent/src/session/ui_store_tests.rs:10:use vw_shared::session::info::{
crates/vw-agent/src/providers/native_search_capability_tests.rs:3:use vw_shared::provider::state::resolve_model_capabilities;
crates/vw-shared/tests/provider_models_json.rs:3:use vw_shared::provider::models::Provider;
crates/vw-agent/src/session/turn_fork.rs:14:use vw_shared::session::ui_types::{ChatMessage, ChatRole, ChatSession};
crates/vw-agent/src/shell/mod.rs:3:/// 重导出 vw_shared::shell::*，保持外部调用路径稳定。
crates/vw-agent/src/shell/mod.rs:4:pub use vw_shared::shell::*;
crates/vw-agent/src/util/locale.rs:2://! 共享文本格式化由 vw_shared 负责，本模块只保留与当前时间语义相关的轻量封装。
crates/vw-agent/src/util/locale.rs:4:pub use vw_shared::util::duration;
crates/vw-agent/src/util/locale.rs:6:pub use vw_shared::util::number;
crates/vw-agent/src/util/locale.rs:8:pub use vw_shared::util::pluralize;
crates/vw-agent/src/util/locale.rs:10:pub use vw_shared::util::titlecase;
crates/vw-agent/src/util/locale.rs:12:pub use vw_shared::util::truncate;
crates/vw-agent/src/util/locale.rs:14:pub use vw_shared::util::truncate_middle;
crates/vw-agent/src/session/processor/context_governance.rs:47:    vw_shared::provider::state::effective_input_window(limit)
crates/vw-agent/src/session/processor/context_governance.rs:58:            .unwrap_or(vw_shared::provider::state::DEFAULT_MODEL_CONTEXT_LIMIT)
crates/vw-agent/src/session/processor/context_governance.rs:67:        vw_shared::provider::state::DEFAULT_MODEL_OUTPUT_LIMIT
crates/vw-gateway-client/src/client/provider_api.rs:3:use vw_shared::provider::types::Info;
crates/vw-gateway-client/src/client/question_api.rs:1:use vw_shared::question;
crates/vw-agent/src/util/format.rs:4:pub use vw_shared::util::format_duration;
crates/vw-agent/src/patch/mod.rs:3://! `vw-agent` 复用 `vw_shared` 中稳定的 patch 数据结构与解析/应用工具，
crates/vw-agent/src/patch/mod.rs:6:pub use vw_shared::patch::*;
crates/vw-agent/src/session/dynamic_tool_store_tests.rs:11:use vw_shared::project::{Info, TimeInfo};
crates/vw-agent/src/session/dynamic_tool_store_tests.rs:12:use vw_shared::session::ui_types::ChatSession;
crates/vw-gateway-client/src/client/session_api.rs:24:use vw_shared::session::ui_types::{
crates/vw-gateway-client/src/client/session_api.rs:27:use vw_shared::todo::Todo;
crates/vw-gateway-client/src/client/session_api.rs:44:    pub messages: Vec<vw_shared::session::ui_types::ChatMessage>,
crates/vw-agent/src/plugins/compat/mcp_elicitation.rs:201:            vw_shared::question::McpForm { server, schema: schema.clone() },
crates/vw-agent/src/plugins/compat/mcp_elicitation.rs:287:            vw_shared::question::McpForm {
crates/vw-acp/src/client/process.rs:183:            vw_shared::shell::apply_augmented_path_tokio_command(&mut cmd);
crates/vw-agent/src/plugins/compat/mcp_oauth_flow.rs:283:            }, vw_shared::question::McpForm { server: server.name.clone(), schema: serde_json::json!({"type":"object", "properties": {}}) })
crates/vw-agent/src/tools/browser/installation.rs:34:    let npm = vw_shared::shell::resolve_executable("npm")
crates/vw-agent/src/tools/browser/installation.rs:35:        .or_else(|| vw_shared::shell::resolve_executable("npm.cmd"))
crates/vw-agent/src/tools/todo/schema.rs:2:use vw_shared::todo::{de_opt_string_or_number, default_todo_priority, default_todo_status};
crates/vw-agent/src/session/llm/acp/model_source/profile.rs:43:        Option<Vec<vw_shared::provider::reasoning::ReasoningOption>>,
crates/vw-agent/src/session/llm/acp/model_source/profile.rs:84:                model_output_limit: vw_shared::provider::state::DEFAULT_MODEL_OUTPUT_LIMIT,
crates/vw-agent/src/session/llm/acp/model_source/profile.rs:85:                model_context_limit: vw_shared::provider::state::DEFAULT_MODEL_CONTEXT_LIMIT,
crates/vw-agent/src/tools/todo/mod.rs:30:pub use vw_shared::todo::{LegacyTodo, Todo};
crates/vw-agent/src/tools/todo/store.rs:6:use vw_shared::todo::{default_todo_priority, default_todo_status};
crates/vw-agent/src/session/llm/acp/model_source/drivers/codebuddy.rs:44:                vw_shared::provider::state::DEFAULT_MODEL_OUTPUT_LIMIT
crates/vw-acp/src/spawn_command_options.rs:7:use vw_shared::shell::tokio_command;
crates/vw-agent/src/agent_scheduler/integration_tests.rs:15:use vw_shared::project::{Info, TimeInfo};
crates/vw-agent/src/agent_scheduler/integration_tests.rs:16:use vw_shared::session::info::{Info as AgentSessionInfo, TimeInfo as AgentSessionTimeInfo};
crates/vw-agent/src/agent_scheduler/integration_tests.rs:17:use vw_shared::session::ui_types::{
crates/vw-agent/src/session/llm/acp/model_source/drivers/openclaw.rs:88:    let mut command = vw_shared::shell::std_command(&profile.config().command);
crates/vw-agent/src/session/llm/acp/model_source/drivers/openclaw.rs:98:    vw_shared::shell::apply_augmented_path_std_command(&mut command);
crates/vw-agent/src/session/llm/acp/model_source/drivers/vibe.rs:54:            vw_shared::provider::state::DEFAULT_MODEL_CONTEXT_LIMIT
crates/vw-agent/src/session/llm/acp/model_source/drivers/vibe.rs:59:            vw_shared::provider::state::DEFAULT_MODEL_OUTPUT_LIMIT
crates/vw-agent/src/agent_scheduler/store/artifacts_tests.rs:9:use vw_shared::project::{Info as ProjectInfo, TimeInfo as ProjectTimeInfo};
crates/vw-agent/src/session/llm/acp/model_source/drivers/copilot_tests.rs:42:            Some(vec![vw_shared::provider::reasoning::ReasoningOption::Effort {
crates/vw-agent/src/agent_scheduler/store/resources_tests.rs:11:use vw_shared::project::{Info as ProjectInfo, TimeInfo as ProjectTimeInfo};
crates/vw-acp/src/agent_registry.rs:11:use vw_shared::shell::{resolve_executable, shell_profile_env_var};
crates/vw-agent/src/session/llm/acp/model_source/drivers/mod.rs:156:) -> anyhow::Result<vw_shared::provider::protocols::ModelProtocol> {
crates/vw-agent/src/session/llm/acp/model_source/drivers/mod.rs:157:    use vw_shared::provider::protocols::ModelProtocol as P;
crates/vw-agent/src/agent_scheduler/store/tasks_tests.rs:22:use vw_shared::project::{Info as ProjectInfo, TimeInfo as ProjectTimeInfo};
crates/vw-acp/src/install.rs:15:        && vw_shared::shell::resolve_executable("uv").is_none()
crates/vw-acp/src/install.rs:51:        vw_shared::shell::resolve_executable("uv").map(|path| path.to_string_lossy().into_owned())
crates/vw-agent/src/project/store/postgres.rs:9:use vw_shared::project::{Info, TimeInfo};
crates/vw-agent/src/project/migration_tests.rs:10:use vw_shared::project::{Info, TimeInfo};
crates/vw-agent/src/auth/mod.rs:12:pub use vw_shared::auth::{ApiInfo, Info, OAUTH_DUMMY_KEY, OauthInfo, WellKnownInfo};
crates/vw-agent/src/auth/mod.rs:23:    vw_shared::auth::store::resolve_filepath(&paths.home, &paths.data)
crates/vw-agent/src/auth/mod.rs:41:    vw_shared::auth::store::get_from(&filepath(), provider_id)
crates/vw-agent/src/auth/mod.rs:51:    vw_shared::auth::store::all_from(&filepath())
crates/vw-agent/src/auth/mod.rs:65:    vw_shared::auth::store::set_to(&filepath(), key, info)
crates/vw-agent/src/auth/mod.rs:78:    vw_shared::auth::store::remove_from(&filepath(), key)
crates/vw-agent/src/project/migration.rs:10:use vw_shared::project::Info;
crates/vw-agent/src/project/mod.rs:65:pub use vw_shared::project::{
crates/vw-agent/src/agent_scheduler/store/postgres/mod_tests.rs:7:use vw_shared::project::{Info as ProjectInfo, TimeInfo as ProjectTimeInfo};
crates/vw-agent/src/session/llm/acp/model_source/drivers/openclaw_tests.rs:130:    let python = vw_shared::shell::resolve_executable("python3").expect("Python fixture runtime");
crates/vw-agent/src/tools/session_tool/tests.rs:112:                vw_shared::session::session_utils::parse_task_session_source_marker(content)
crates/vw-agent/src/tools/session_tool/tests.rs:377:    let source = vw_shared::session::session_utils::parse_task_session_source_marker(
crates/vw-agent/src/session/session.rs:220:pub use vw_shared::session::info::{Info, RevertInfo, ShareInfo, Summary, TimeInfo};
crates/vw-agent/src/session/session.rs:221:pub use vw_shared::session::session_utils::{create_slug, is_default_title};
crates/vw-agent/src/tools/session_tool/mod.rs:25:use vw_shared::session::session_utils::encode_task_session_source_marker;
crates/vw-agent/src/tools/mcp_common/mod.rs:722:                vw_shared::shell::apply_augmented_path_tokio_command(&mut command);
crates/vw-agent/src/session/llm/acp/model_source/drivers/copilot.rs:4:use vw_shared::provider::reasoning::{ReasoningFormat, reasoning_levels, reasoning_profile};
crates/vw-agent/src/snapshot/mod.rs:45:pub use vw_shared::snapshot::{DiffStatus, FileDiff};
crates/vw-agent/src/session/llm/acp/model_source/drivers/claude.rs:52:            vw_shared::provider::state::DEFAULT_MODEL_OUTPUT_LIMIT.to_string()
crates/vw-agent/src/agent_scheduler/store/postgres/dependencies_tests.rs:9:use vw_shared::project::{Info as ProjectInfo, TimeInfo as ProjectTimeInfo};
crates/vw-agent/src/session/llm/acp/model_source/drivers/snow.rs:38:            vw_shared::provider::state::DEFAULT_MODEL_OUTPUT_LIMIT
crates/vw-agent/src/session/llm/acp/model_source/drivers/droid.rs:46:                vw_shared::provider::state::DEFAULT_MODEL_OUTPUT_LIMIT
crates/vw-agent/src/project/store_tests.rs:9:use vw_shared::project::{Commands, Icon, Info, TimeInfo, Vcs};
crates/vw-agent/src/session/llm/acp/model_source/drivers/hermes.rs:96:        vw_shared::provider::state::DEFAULT_MODEL_OUTPUT_LIMIT
crates/vw-agent/src/session/message.rs:3://! 本模块复用 `vw_shared` 中的消息类型，并负责把消息/part 写入本地 storage 后发布 bus
crates/vw-agent/src/session/message.rs:7:pub use vw_shared::message::types::*;
crates/vw-agent/src/session/llm/acp/model_source/drivers/drivers_tests.rs:5:use vw_shared::provider::protocols::{ModelProtocol as P, ProtocolEndpoint};
crates/vw-agent/src/session/llm/acp/model_source/drivers/drivers_tests.rs:31:    use vw_shared::provider::protocols::*;
crates/vw-agent/src/session/llm/acp/model_source/drivers/drivers_tests.rs:125:    use vw_shared::provider::protocols::{ProtocolAuthentication, ProtocolModelScope};
crates/vw-agent/src/project/store.rs:13:use vw_shared::project::{Info, TimeInfo, Vcs};
crates/vw-agent/src/tools/skill/mod_tests.rs:482:            vw_shared::permission::Rule {
crates/vw-agent/src/tools/skill/mod_tests.rs:485:                action: vw_shared::permission::Action::Deny,
crates/vw-agent/src/session/llm/acp/model_source/drivers/opencode.rs:4:use vw_shared::provider::reasoning::{ReasoningFormat, reasoning_levels, reasoning_profile};
crates/vw-agent/src/session/llm/acp/model_source/drivers/opencode.rs:49:    use vw_shared::provider::ModelProtocol as P;
crates/vw-desktop/src/apps/workflow/state/ids_tests.rs:136:    let before = vw_shared::time::now_ms() as u128;
crates/vw-desktop/src/apps/workflow/state/ids_tests.rs:147:    let after = vw_shared::time::now_ms() as u128;
crates/vw-agent/src/session/llm/acp/model_source/drivers/opencode_tests.rs:33:    use vw_shared::provider::reasoning::ReasoningOption;
crates/vw-agent/src/tools/context/mod.rs:127:    delegated_skill_rules: Vec<vw_shared::permission::Ruleset>,
crates/vw-agent/src/tools/context/mod.rs:363:        rules: vw_shared::permission::Ruleset,
crates/vw-agent/src/tools/context/mod.rs:369:    pub(crate) fn delegated_skill_rules(&self) -> &[vw_shared::permission::Ruleset] {
crates/vw-agent/src/agent_scheduler/store/mod.rs:209:        let info = vw_shared::project::Info {
crates/vw-agent/src/agent_scheduler/store/mod.rs:216:            time: vw_shared::project::TimeInfo { created: 0, updated: 0, initialized: None },
crates/vw-agent/src/agent_scheduler/store/logs_tests.rs:10:use vw_shared::project::{Info as ProjectInfo, TimeInfo as ProjectTimeInfo};
crates/vw-agent/src/usage_bridge/context.rs:9:    Ok(vw_shared::local_apps::HOST_ID.to_owned())
crates/vw-agent/src/question/mod.rs:8:pub use vw_shared::question::*;
crates/vw-agent/src/question/mod.rs:112:    form: vw_shared::question::McpForm,
crates/vw-agent/src/question/mod.rs:120:    mcp_form: Option<vw_shared::question::McpForm>,
crates/vw-agent/src/usage_bridge/native.rs:49:        anyhow::ensure!(vw_shared::local_apps::restore(&crate::global::paths().home,
crates/vw-agent/src/agent_scheduler/store/outbox_tests.rs:13:use vw_shared::project::{Info as ProjectInfo, TimeInfo as ProjectTimeInfo};
crates/vw-agent/src/agent_scheduler/store/runs_tests.rs:12:use vw_shared::project::{Info as ProjectInfo, TimeInfo as ProjectTimeInfo};
crates/vw-agent/src/session/llm/aisdk/reasoning.rs:4:use vw_shared::provider::reasoning::{ReasoningFormat, reasoning_levels, reasoning_profile};
crates/vw-agent/src/session/llm/aisdk/reasoning.rs:5:use vw_shared::provider::types::Model;
crates/vw-agent/src/agent_scheduler/store/events_tests.rs:10:use vw_shared::project::{Info as ProjectInfo, TimeInfo as ProjectTimeInfo};
crates/vw-agent/src/session/llm/aisdk/reasoning_tests.rs:18:    vw_shared::provider::state::from_models_dev_model(&raw_provider, &raw_model)
crates/vw-agent/src/agent_scheduler/store/schema_tests.rs:8:use vw_shared::project::{Info, TimeInfo};
crates/vw-gateway-client/src/client/session_api_tests.rs:13:use vw_shared::session::ui_types::{ChatMessage, ChatRole, ChatSession};
crates/vw-gateway-client/src/client/session_api_tests.rs:14:use vw_shared::session::ui_types::{SessionMessageDelta, SessionMessageRecord};
crates/vw-agent/src/agent_scheduler/task_board.rs:28:    mut task: vw_shared::task::Task,
crates/vw-agent/src/agent_scheduler/task_board.rs:29:) -> Result<vw_shared::task::Task> {
crates/vw-agent/src/agent_scheduler/task_board.rs:94:) -> Result<Option<vw_shared::task::Task>> {
crates/vw-agent/src/agent_scheduler/task_board.rs:105:) -> Result<Vec<vw_shared::task::Task>> {
crates/vw-agent/src/agent_scheduler/task_board.rs:135:fn native_task_title(task: &vw_shared::task::Task) -> String {
crates/vw-agent/src/agent_scheduler/task_board.rs:143:fn native_task_metadata(task: &vw_shared::task::Task) -> Result<Value> {
crates/vw-agent/src/agent_scheduler/task_board.rs:174:) -> Result<vw_shared::task::Task> {
crates/vw-agent/src/agent_scheduler/task_board.rs:184:        .map(|log| vw_shared::task::TaskLogEntry {
crates/vw-agent/src/agent_scheduler/task_board.rs:189:                .and_then(vw_shared::task::TaskStatus::parse_key),
crates/vw-agent/src/agent_scheduler/task_board.rs:193:                .and_then(vw_shared::task::TaskStatus::parse_key),
crates/vw-agent/src/agent_scheduler/task_board.rs:197:    Ok(vw_shared::task::Task {
crates/vw-agent/src/agent_scheduler/task_board.rs:204:            .and_then(vw_shared::task::TaskExecutorBackend::from_id)
crates/vw-agent/src/agent_scheduler/task_board.rs:205:            .unwrap_or(vw_shared::task::TaskExecutorBackend::Internal),
crates/vw-agent/src/agent_scheduler/task_board.rs:210:        status: vw_shared::task::TaskStatus::parse_key(status_key)
crates/vw-agent/src/agent_scheduler/task_board.rs:211:            .unwrap_or(vw_shared::task::TaskStatus::Pool),
crates/vw-agent/src/agent_scheduler/task_board.rs:226:            .and_then(vw_shared::task::TaskSplitMode::parse_key)
crates/vw-agent/src/agent/mod.rs:341:) -> Option<vw_shared::message::types::ModelRef> {
crates/vw-agent/src/agent/mod.rs:345:        return Some(vw_shared::message::types::ModelRef {
crates/vw-agent/src/agent/mod.rs:357:    Some(vw_shared::message::types::ModelRef {
crates/vw-desktop/src/desktop_update_helper.rs:224:        if ready.kind() != vw_shared::update::InstallerKind::Dmg {
crates/vw-desktop/src/desktop_update_helper.rs:233:        if ready.kind() != vw_shared::update::InstallerKind::Msi {
crates/vw-agent/src/update.rs:4:pub use vw_shared::update::*;
crates/vw-agent/src/model_relay/messages_responses_result_tests.rs:56:        super::protocol_bridge::response(vw_shared::provider::ModelProtocol::Responses, &body)
crates/vw-agent/src/agent/loop_/parsing/json.rs:432:pub use vw_shared::json::extract_json_values;
crates/vw-agent/src/model_relay/protocol_bridge.rs:5:use vw_shared::provider::ModelProtocol;
crates/vw-desktop/src/main.rs:79:    let runtime_log_dir = vw_shared::runtime_log::default_runtime_log_dir();
crates/vw-desktop/src/main.rs:80:    if let Err(err) = vw_shared::runtime_log::ensure_runtime_log_dir(&runtime_log_dir) {
crates/vw-desktop/src/main.rs:87:        .with_writer(vw_shared::runtime_log::RuntimeLogMakeWriter::new(runtime_log_dir))
crates/vw-desktop/src/main.rs:97:    if let Err(error) = native_crash::install(&vw_shared::runtime_log::default_runtime_log_dir()) {
crates/vw-agent/src/model_relay/protocol_bridge_stream.rs:10:use vw_shared::provider::ModelProtocol as P;
crates/vw-agent/src/model_relay/messages_gemini_result.rs:4:use vw_shared::provider::ModelProtocol;
crates/vw-agent/src/model_relay/messages_request_core_tests.rs:93:    let mut capabilities = vw_shared::provider::state::default_model_capabilities();
crates/vw-agent/src/model_relay/messages_thinking_config.rs:6:    use vw_shared::provider::reasoning::{ReasoningFormat, reasoning_profile};
crates/vw-agent/src/model_relay/messages_thinking_config.rs:17:    use vw_shared::provider::reasoning::{ReasoningFormat, reasoning_profile};
crates/vw-agent/src/model_relay/messages_thinking_config.rs:81:    use vw_shared::provider::reasoning::{ReasoningFormat, reasoning_levels, reasoning_profile};
crates/vw-agent/src/model_relay/messages_thinking_config.rs:99:            vw_shared::provider::reasoning::ReasoningOption::Toggle => true,
crates/vw-agent/src/model_relay/messages_thinking_config.rs:100:            vw_shared::provider::reasoning::ReasoningOption::Effort { values } => {
crates/vw-agent/src/session/child_agent_tests.rs:625:                info.permission = Some(vec![vw_shared::permission::Rule {
crates/vw-agent/src/session/child_agent_tests.rs:628:                    action: vw_shared::permission::Action::Deny,
crates/vw-agent/src/session/child_agent_tests.rs:684:    assert_eq!(rules[0].action, vw_shared::permission::Action::Deny);
crates/vw-desktop/src/desktop_update_linux.rs:6:use vw_shared::update::InstallerKind;
crates/vw-desktop/src/desktop_update_linux.rs:12:    let kind = vw_shared::update::desktop_release::linux_installer_kind()?;
crates/vw-agent/src/model_relay/messages_gemini_request.rs:5:use vw_shared::provider::ModelProtocol;
crates/vw-agent/src/model_relay/responses_tests.rs:158:    let mut capabilities = vw_shared::provider::state::default_model_capabilities();
crates/vw-agent/src/provider/model_catalog.rs:12:use vw_shared::provider::reasoning::ReasoningOption;
crates/vw-agent/src/provider/model_catalog.rs:13:use vw_shared::provider::state::{DEFAULT_MODEL_CONTEXT_LIMIT, DEFAULT_MODEL_OUTPUT_LIMIT};
crates/vw-agent/src/provider/model_catalog.rs:14:use vw_shared::provider::types::CatalogExecutionCapabilities;
crates/vw-desktop/src/app/session/active_tests.rs:7:use vw_shared::session::info::{Info, TimeInfo};
crates/vw-desktop/src/app/session/active_tests.rs:129:    let content = vw_shared::session::session_utils::encode_task_session_source_marker(
crates/vw-agent/src/model_relay/responses_codex_chat.rs:55:    use vw_shared::provider::reasoning::{ReasoningFormat, reasoning_profile};
crates/vw-desktop/src/app/session/active.rs:44:                if vw_shared::session::session_utils::parse_task_session_source_marker(
crates/vw-desktop/src/app/session/active.rs:76:            if vw_shared::session::session_utils::parse_task_session_source_marker(&message.content)
crates/vw-desktop/src/app/session/active.rs:97:                    vw_shared::session::session_utils::strip_task_session_source_marker(
crates/vw-desktop/src/app/session/active.rs:255:                        vw_shared::session::session_utils::strip_task_session_source_marker(
crates/vw-agent/src/model_relay/routing.rs:9:use vw_shared::provider::protocols::{
crates/vw-desktop/src/app/session/ui_projection.rs:126:                        vw_shared::session::ui_store::set_thread_pinned(
crates/vw-desktop/src/app/session/ui_projection.rs:257:    vw_shared::session::ui_store::pinned_threads_scope(
crates/vw-desktop/src/app/session/ui_projection.rs:269:        let ids = vw_shared::session::ui_store::load_pinned_threads(&data_dir, &scope)
crates/vw-desktop/src/app/session/loading_tests.rs:45:fn session_info(id: &str) -> vw_shared::session::info::Info {
crates/vw-desktop/src/app/session/loading_tests.rs:46:    vw_shared::session::info::Info {
crates/vw-desktop/src/app/session/loading_tests.rs:56:        time: vw_shared::session::info::TimeInfo {
crates/vw-desktop/src/app/session/loading_tests.rs:251:use vw_shared::session::ui_types::SessionMessageRecord;
crates/vw-desktop/src/app/session/loading_tests.rs:271:            role: vw_shared::session::ui_types::ChatRole::User,
crates/vw-agent/src/provider/provider_tests.rs:242:    let expected = vw_shared::provider::state::resolve_model_limit(None, None, None);
crates/vw-agent/src/provider/provider_tests.rs:279:    let expected = vw_shared::provider::state::resolve_model_limit(None, None, None);
crates/vw-agent/src/permission/next.rs:43:pub use vw_shared::permission::{Action, Rule, Ruleset};
crates/vw-agent/src/worktree/cleanup_tests.rs:319:                &vw_shared::project::Info {
crates/vw-agent/src/worktree/cleanup_tests.rs:326:                    time: vw_shared::project::TimeInfo {
crates/vw-desktop/src/app/session/loading.rs:111:        vw_shared::session::ui_types::SessionMessageRecord,
crates/vw-desktop/src/app/session/loading.rs:161:        vw_shared::session::ui_types::SessionMessageRecord,
crates/vw-desktop/src/app/session/loading.rs:235:            let line = vw_shared::session::session_utils::initial_title_from_content(&content)?;
crates/vw-desktop/src/app/session/loading.rs:325:                        .session_list::<Vec<vw_shared::session::info::Info>>(None)
crates/vw-desktop/src/app/session/loading.rs:334:                    Vec<vw_shared::session::info::Info>,
crates/vw-agent/src/provider/provider.rs:43:pub use vw_shared::provider::state::{
crates/vw-agent/src/provider/provider.rs:47:pub use vw_shared::provider::types::{
crates/vw-agent/src/provider/provider.rs:52:pub use vw_shared::provider::types::{parse_model, sort};
crates/vw-agent/src/provider/provider.rs:447:    let reasoning_model_ids = vw_shared::provider::state::catalog_reasoning_model_ids(&database);
crates/vw-agent/src/provider/provider.rs:595:                let limit = vw_shared::provider::state::resolve_model_limit_with_base(
crates/vw-agent/src/provider/provider.rs:641:            vw_shared::provider::state::apply_api_overrides(&mut model.api, provider, configured);
crates/vw-cli/src/tests.rs:7:use vw_shared::task::Task;
crates/vw-desktop/src/app/session/agent_children.rs:5:use vw_shared::session::info::Info;
crates/vw-desktop/src/app/session/chat_ui.rs:66:            vw_shared::session::session_utils::strip_task_session_source_marker(&message.content)
crates/vw-desktop/src/app/session/agent_children_tests.rs:5:use vw_shared::session::info::{Info, TimeInfo};
crates/vw-cli/src/handlers/api/session_content.rs:11:use vw_shared::session::ui_types::ChatSession;
crates/vw-desktop/src/app/session_gateway.rs:43:use vw_shared::session::ui_types::{ChatSession, ChatSessionMeta, SessionMessageDelta};
crates/vw-desktop/src/app/session_gateway.rs:323:            vw_shared::session::ui_types::SessionMessageRecord,
crates/vw-desktop/src/app/session_gateway.rs:348:        vw_shared::session::ui_types::SessionMessageRecord,
crates/vw-desktop/src/app/session_gateway.rs:580:pub async fn gateway_question_list_async() -> Result<Vec<vw_shared::question::Request>, String> {
crates/vw-desktop/src/app/session_gateway.rs:592:) -> Result<Vec<vw_shared::todo::Todo>, String> {
crates/vw-desktop/src/app/session_gateway.rs:647:    info: &vw_shared::session::info::Info,
crates/vw-desktop/src/app/session_gateway.rs:1063:pub fn gateway_question_list() -> Vec<vw_shared::question::Request> {
crates/vw-desktop/src/app/session_gateway.rs:1113:    info: &vw_shared::session::info::Info,
crates/vw-agent/src/model_relay/tool_media_request.rs:82:    target: vw_shared::provider::ModelProtocol,
crates/vw-agent/src/model_relay/tool_media_request.rs:86:    use vw_shared::provider::ModelProtocol;
crates/vw-agent/src/id/mod.rs:3://! 当前模块复用 `vw_shared` 中定义的 ID 类型，避免代理层与共享层维护两套标识符约定。
crates/vw-agent/src/id/mod.rs:6:pub use vw_shared::id::*;
crates/vw-agent/src/model_relay/messages_request_tests.rs:9:    use vw_shared::provider::protocols::ModelProtocol as P;
crates/vw-agent/src/model_relay/messages_responses_request.rs:4:use vw_shared::provider::ModelProtocol;
crates/vw-agent/src/acp/mounts_tests.rs:288:            .with_delegated_skill_rules(vec![vw_shared::permission::Rule {
crates/vw-agent/src/acp/mounts_tests.rs:291:                action: vw_shared::permission::Action::Deny,
crates/vw-desktop/src/app/config_desktop_tests.rs:624:    use vw_shared::update::UpdateSource;
crates/vw-desktop/src/app/config_desktop_tests.rs:657:    use vw_shared::update::UpdateSource;
crates/vw-agent/src/model_relay/responses_anthropic_request.rs:6:use vw_shared::provider::ModelProtocol;
crates/vw-agent/src/model_relay/messages_gemini_result_tests.rs:74:        super::protocol_bridge::response(vw_shared::provider::ModelProtocol::Gemini, &body)
crates/vw-agent/src/model_relay/responses.rs:91:    let profile = vw_shared::provider::reasoning::reasoning_profile(
crates/vw-cli/src/cli/tui/app.rs:31:use vw_shared::session::ui_types::{ChatMessage, ChatRole, ChatSession, ChatSessionMeta};
crates/vw-cli/src/cli/tui/app.rs:3735:    use vw_shared::id::{Prefix, descending};
crates/vw-agent/src/model_relay/messages_thinking_core_tests.rs:133:    use vw_shared::provider::types::{ApiInfo, Model, ModelCost, ModelCostCache, ModelLimit};
crates/vw-agent/src/model_relay/messages_thinking_core_tests.rs:145:        capabilities: vw_shared::provider::state::default_model_capabilities(),
crates/vw-agent/src/model_relay/messages_responses_result.rs:4:use vw_shared::provider::ModelProtocol;
crates/vw-cli/src/cli/tui/session_command_tests.rs:9:use vw_shared::session::ui_types::{ChatMessage, ChatRole, ChatSession};
crates/vw-agent/src/model_relay/transport.rs:278:                vw_shared::provider::protocols::ProtocolAuthentication::None {}
crates/vw-agent/src/model_relay/transport.rs:322:            vw_shared::provider::protocols::ProtocolAuthentication::None {}
crates/vw-agent/src/gateway/api/handlers/session/ui_ops_tests.rs:17:use vw_shared::session::info::{Info as AgentSessionInfo, TimeInfo as AgentSessionTimeInfo};
crates/vw-agent/src/gateway/api/handlers/session/ui_ops_tests.rs:479:        .join(vw_shared::session::path::session_scope_key(scope))
crates/vw-agent/src/gateway/agent_apps_diagnostic_executors/model_check/target.rs:176:        use vw_shared::provider::protocols::{
crates/vw-cli/src/cli/tui/model/ui_message.rs:6:use vw_shared::session::ui_types as session_ui;
crates/vw-desktop/src/app/views/task_board/control/helpers.rs:33:    let value = vw_shared::util::format_duration(secs);
crates/vw-agent/src/model_relay/generation.rs:14:pub(crate) use vw_shared::provider::ModelProtocol as GenerationProtocol;
crates/vw-agent/src/model_relay/generation.rs:84:    endpoint: &vw_shared::provider::protocols::ProtocolEndpoint,
crates/vw-agent/src/model_relay/generation.rs:87:    use vw_shared::provider::protocols::ProtocolAuthentication as A;
crates/vw-agent/src/gateway/api/handlers/session/fork_acceptance_tests.rs:17:use vw_shared::session::ui_types::{ChatRole, SessionMessageRecord};
crates/vw-agent/src/model_relay/messages_thinking_tests.rs:11:    use vw_shared::provider::protocols::ModelProtocol as P;
crates/vw-cli/src/cli/tui/model/overlay.rs:8:use vw_shared::question;
crates/vw-cli/src/cli/tui/model/overlay.rs:9:use vw_shared::todo::Todo;
crates/vw-agent/src/gateway/api/handlers/session/stream.rs:1491:            let mut processing_timer = vw_shared::time::ActiveTimer::default();
crates/vw-agent/src/gateway/api/handlers/session/stream.rs:1501:            let mut llm_timer = vw_shared::time::ActiveTimer::default();
crates/vw-desktop/src/app/views/task_board/board_tests.rs:22:    assert_eq!(super::format_duration_ms(1_250), vw_shared::util::format_duration(1));
crates/vw-desktop/src/app/views/task_board/board.rs:43:    let value = vw_shared::util::format_duration(secs);
crates/vw-agent/src/model_relay/transport_tests.rs:96:    use vw_shared::provider::reasoning::ReasoningOption;
crates/vw-agent/src/gateway/api/handlers/auth_tests.rs:36:    vw_shared::auth::store::resolve_filepath(&paths.home, &paths.data)
crates/vw-cli/src/cli/tui/commands.rs:5:use vw_shared::session::ui_types::ChatSessionMeta;
crates/vw-cli/src/cli/tui/commands.rs:325:                vw_shared::session::session_utils::initial_title_from_content(last_content)
crates/vw-agent/src/model_relay/responses_anthropic_result.rs:5:use vw_shared::provider::ModelProtocol;
crates/vw-cli/src/cli/tui/state/mod.rs:21:use vw_shared::session::ui_types::{
crates/vw-agent/src/model_relay/protocol_bridge_tests.rs:3:use vw_shared::provider::ModelProtocol as P;
crates/vw-agent/src/gateway/api/handlers/global.rs:225:    vw_shared::shell::resolve_executable(command).is_some()
crates/vw-cli/src/cli/tui/state/reducer.rs:10:use vw_shared::session::ui_types::ChatSession;
crates/vw-desktop/src/app/views/task_board/panel/mod.rs:109:    let value = vw_shared::util::format_duration(secs);
crates/vw-cli/src/cli/tui/runtime/capabilities.rs:154:fn model_efforts(model: &vw_shared::provider::types::Model) -> Vec<ReasoningEffort> {
crates/vw-cli/src/cli/tui/runtime/capabilities.rs:157:        for level in vw_shared::provider::reasoning::reasoning_levels(
crates/vw-cli/src/cli/tui/runtime/capabilities_tests.rs:31:        let mut model: vw_shared::provider::types::Model = serde_json::from_value(json!({
crates/vw-cli/src/cli/tui/runtime/capabilities_tests.rs:42:            Some(vec![vw_shared::provider::reasoning::ReasoningOption::Effort {
crates/vw-agent/src/gateway/api/handlers/agent_apps_auth_read.rs:111:                let path = vw_shared::local_apps::auth_policy_path(
crates/vw-cli/src/cli/tui/runtime/session_store.rs:13:use vw_shared::session::info::Info;
crates/vw-cli/src/cli/tui/runtime/session_store.rs:14:use vw_shared::session::ui_types::{ChatSession, ChatSessionMeta};
crates/vw-agent/src/gateway/api/handlers/agent_apps/local_access_tests.rs:8:    let credential = vw_shared::local_apps::initialize(home.path(), "p", "c").unwrap();
crates/vw-agent/src/gateway/api/handlers/agent_apps/local_access_tests.rs:9:    let policy = vw_shared::local_apps::policy_path(home.path(), "p", "c");
crates/vw-agent/src/gateway/api/handlers/agent_apps/local_access_tests.rs:13:    let host = vw_shared::local_apps::HOST_ID;
crates/vw-agent/src/gateway/api/handlers/agent_apps/local_access_tests.rs:63:            vw_shared::local_apps::initialize(std::path::Path::new(&home), "p", "c").unwrap();
crates/vw-agent/src/gateway/api/handlers/agent_apps/local_access_tests.rs:95:    let credential = vw_shared::local_apps::initialize_auth(home, "p", "c").unwrap();
crates/vw-agent/src/gateway/api/handlers/agent_apps/local_access_tests.rs:111:    let host = vw_shared::local_apps::HOST_ID;
crates/vw-agent/src/gateway/api/handlers/agent_apps/local_access_tests.rs:133:    let other = vw_shared::local_apps::initialize_auth(home, "p", "other").unwrap();
crates/vw-cli/src/cli/tui/runtime/interaction.rs:7:use vw_shared::{question, todo::Todo};
crates/vw-agent/src/gateway/api/handlers/app_routing/outbound_probe.rs:28:use vw_shared::update::desktop_release::SHIJU_MANIFEST;
crates/vw-cli/src/cli/tui/runtime/session_diff.rs:5:use vw_shared::snapshot::{DiffStatus, FileDiff};
crates/vw-agent/src/gateway/api/handlers/agent_apps/access.rs:962:    vw_shared::local_apps::policy_path(
crates/vw-agent/src/gateway/api/handlers/agent_apps/access.rs:2447:            let Some(host) = policy.hosts.iter().find(|host| host.id == grant.host_id && host.id == vw_shared::local_apps::HOST_ID
crates/vw-desktop/src/app/session_gateway_tests.rs:275:    let session = vw_shared::session::ui_types::ChatSession {
crates/vw-desktop/src/app/session_gateway_tests.rs:281:        steps: vec![vw_shared::session::ui_types::ChatSessionStep {
crates/vw-desktop/src/app/session_gateway_tests.rs:288:            usage: vw_shared::session::ui_types::TokenUsage::default(),
crates/vw-desktop/src/app/session_gateway_tests.rs:308:    use vw_shared::session::ui_types::{
crates/vw-desktop/src/app/session_gateway_tests.rs:330:    let full_session = vw_shared::session::ui_types::ChatSession {
crates/vw-desktop/src/app/session_gateway_tests.rs:508:        let delta = vw_shared::session::ui_types::SessionMessageDelta {
crates/vw-desktop/src/app/session_gateway_tests.rs:547:        let delta = vw_shared::session::ui_types::SessionMessageDelta {
crates/vw-desktop/src/app/session_gateway_tests.rs:576:    let delta = vw_shared::session::ui_types::SessionMessageDelta {
crates/vw-cli/src/cli/tui/session_restore_tests.rs:10:use vw_shared::session::ui_types::{ChatMessage, ChatRole, ChatSession};
crates/vw-cli/src/cli/tui/bridge.rs:62:                    .and_then(vw_shared::session::session_utils::initial_title_from_content)
crates/vw-cli/src/cli/tui/bridge.rs:66:                                vw_shared::session::session_utils::initial_title_from_content(
crates/vw-cli/src/cli/tui/bridge_tests.rs:13:use vw_shared::session::ui_types::{ChatMessage, ChatRole, ChatSession};
crates/vw-desktop/src/app/web_install/installer.rs:9:use vw_shared::update::UpdateProgress;
crates/vw-desktop/src/app/web_install/installer.rs:10:use vw_shared::update::{UpdateSource, desktop_release::SHIJU_MANIFEST};
crates/vw-desktop/src/app/web_install/mod.rs:11:use vw_shared::update::UpdateProgress;
crates/vw-desktop/src/app/web_install/mod.rs:131:    source: Option<vw_shared::update::UpdateSource>,
crates/vw-desktop/src/app/installer_update.rs:8:use vw_shared::update::{DesktopInstaller, DesktopRelease, InstallerKind, UpdateProgress};
crates/vw-desktop/src/app/installer_update.rs:81:    let path = vw_shared::update::download_desktop_installer(
crates/vw-desktop/src/app/installer_update.rs:269:        "linux" => vw_shared::update::desktop_release::linux_installer_kind(),
crates/vw-agent/src/channels/runtime_command/session_commands.rs:270:pub(super) fn effort_levels(model: &vw_shared::provider::types::Model) -> Vec<String> {
crates/vw-agent/src/channels/runtime_command/session_commands.rs:274:    vw_shared::provider::reasoning::reasoning_levels(
crates/vw-cli/src/lib.rs:90:use vw_shared::task::{SubTask, Task, TaskExecutorBackend, TaskSplitMode, TaskStatus};
crates/vw-cli/src/lib.rs:106:    pub(crate) use vw_shared::session::ui_types;
crates/vw-cli/src/lib.rs:852:    let runtime_log_dir = vw_shared::runtime_log::default_runtime_log_dir();
crates/vw-cli/src/lib.rs:853:    if let Err(err) = vw_shared::runtime_log::ensure_runtime_log_dir(&runtime_log_dir) {
crates/vw-cli/src/lib.rs:865:            .with_writer(vw_shared::runtime_log::RuntimeLogMakeWriter::new(runtime_log_dir))
crates/vw-cli/src/lib.rs:878:            .with_writer(vw_shared::runtime_log::RuntimeLogMakeWriter::new(runtime_log_dir))
crates/vw-cli/src/lib.rs:890:            .with_writer(vw_shared::runtime_log::RuntimeLogMakeWriter::new(runtime_log_dir))
crates/vw-desktop/src/app/desktop_integration_47_tests.rs:5:use vw_shared::update::{DesktopRelease, UpdateSource};
crates/vw-desktop/src/app/models.rs:7:pub use vw_shared::session::ui_types::{
crates/vw-desktop/src/app/provider.rs:13:    pub use vw_shared::provider::types::{
crates/vw-desktop/src/app/provider.rs:19:    pub use vw_shared::provider::types::{parse_model, sort};
crates/vw-desktop/src/app/config_cli_bootstrap.rs:106:    let cli_build_id = vw_shared::build_identity::executable_build_id(&cli)
crates/vw-desktop/src/app/views/project/components/new_session.rs:29:use vw_shared::util::truncate;
crates/vw-desktop/src/app/mod.rs:210:pub use vw_shared::time;
crates/vw-desktop/src/app/views/project/components/sessions_panel.rs:20:use vw_shared::session::info as session;
crates/vw-desktop/src/app/message/git/shared_tests.rs:59:    app.sessions = vec![vw_shared::session::info::Info {
crates/vw-desktop/src/app/message/git/shared_tests.rs:69:        time: vw_shared::session::info::TimeInfo {
crates/vw-desktop/src/app/views/project/components/sessions_panel_tests.rs:11:use vw_shared::session::info::{Info, Summary, TimeInfo};
crates/vw-desktop/src/app/views/project/components/projects_list.rs:32:use vw_shared::session::info as session;
crates/vw-desktop/src/app/message/chat/processing_timing.rs:4:use vw_shared::session::ui_types::ProcessingTiming;
crates/vw-desktop/src/app/views/project/components/projects_list_tests.rs:7:use vw_shared::session::info::{Info, TimeInfo};
crates/vw-desktop/src/app/message/chat/processing_timing_tests.rs:25:        Some(vw_shared::session::ui_types::ProcessingTiming {
crates/vw-desktop/src/app/message/chat/processing_timing_tests.rs:47:        Some(vw_shared::session::ui_types::ProcessingTiming {
crates/vw-desktop/src/app/views/project/components/session_row.rs:10:use vw_shared::session::info::{Info, TimeInfo};
crates/vw-desktop/src/app/views/project/components/session_row.rs:97:                vw_shared::session::session_utils::initial_title_from_content(&request.query)
crates/vw-desktop/src/app/message/chat/mod.rs:223:    pub(crate) info: vw_shared::session::info::Info,
crates/vw-desktop/src/app/message/chat/mod.rs:537:    VisibleTaskSessionLoaded(Result<vw_shared::session::info::Info, String>),
crates/vw-desktop/src/app/message/chat/mod.rs:682:    AgentSessionDiffSummary(u64, String, vw_shared::session::info::Summary),
crates/vw-desktop/src/app/message/chat/mod.rs:683:    SessionSummaryReloaded(Result<vw_shared::session::info::Info, String>),
crates/vw-desktop/src/app/message/chat/mod.rs:723:    PublishedSessionTitleLoaded(vw_shared::session::info::Info),
crates/vw-desktop/src/app/message/chat/mod.rs:746:    QuestionListLoaded(Result<Vec<vw_shared::question::Request>, String>),
crates/vw-desktop/src/app/message/chat/mod.rs:752:    InputPanelTodosLoaded(String, Result<Vec<vw_shared::todo::Todo>, String>),
crates/vw-desktop/src/app/message/chat/mod.rs:1053:        result: Result<vw_shared::session::info::Info, String>,
crates/vw-desktop/src/app/desktop_integration_49_tests.rs:6:    use vw_shared::update::{DesktopRelease, UpdateSource};
crates/vw-desktop/src/app/impls/app_update_tests.rs:705:    use vw_shared::update::UpdateSource;
crates/vw-desktop/src/app/impls/app_update_tests.rs:719:    use vw_shared::update::{DesktopRelease, UpdateSource};
crates/vw-desktop/src/app/desktop_integration_48_tests.rs:6:    use vw_shared::update::{DesktopRelease, UpdateProgress, UpdateSource};
crates/vw-desktop/src/app/message/chat/input_tests.rs:219:    app.sessions.push(vw_shared::session::info::Info {
crates/vw-desktop/src/app/message/chat/input_tests.rs:229:        time: vw_shared::session::info::TimeInfo {
crates/vw-desktop/src/app/message/view/mod.rs:160:    CheckCliToolUpdateFinished(u64, vw_shared::update::UpdateSource, Result<String, String>),
crates/vw-desktop/src/app/message/view/mod.rs:166:        vw_shared::update::UpdateSource,
crates/vw-desktop/src/app/message/view/mod.rs:167:        Result<vw_shared::update::DesktopRelease, String>,
crates/vw-desktop/src/app/message/view/mod.rs:173:        vw_shared::update::UpdateSource,
crates/vw-desktop/src/app/message/view/mod.rs:174:        Result<vw_shared::update::DesktopRelease, String>,
crates/vw-desktop/src/app/message/view/mod.rs:176:    AppUpdateSourceSelected(vw_shared::update::UpdateSource),
crates/vw-desktop/src/app/message/chat/session_tests.rs:63:fn session_info(id: &str, title: &str, directory: &str) -> vw_shared::session::info::Info {
crates/vw-desktop/src/app/message/chat/session_tests.rs:64:    vw_shared::session::info::Info {
crates/vw-desktop/src/app/message/chat/session_tests.rs:74:        time: vw_shared::session::info::TimeInfo {
crates/vw-desktop/src/app/message/chat/session_tests.rs:86:fn persisted_user_message(id: &str) -> vw_shared::message::types::WithParts {
crates/vw-desktop/src/app/message/chat/session_tests.rs:87:    use vw_shared::message::types as message;
crates/vw-desktop/src/app/message/chat/session_tests.rs:1567:    child.slug = vw_shared::session::info::Info::plan_run_slug(&child.slug);
crates/vw-desktop/src/app/message/chat/session_tests.rs:1779:    app.sessions[0].slug = vw_shared::session::info::Info::plan_run_slug(&app.sessions[0].slug);
crates/vw-desktop/src/app/message/chat/session_tests.rs:2254:                vw_shared::session::ui_types::ChatSessionMeta {
crates/vw-desktop/src/app/message/view/panel/settings_tests.rs:475:    use vw_shared::update::{DesktopRelease, UpdateSource};
crates/vw-desktop/src/app/message/view/panel/settings_tests.rs:512:    let first = vw_shared::update::UpdateProgress {
crates/vw-desktop/src/app/message/view/panel/settings_tests.rs:756:    use vw_shared::update::UpdateSource;
crates/vw-desktop/src/app/message/view/panel/settings_tests.rs:767:            Ok(vw_shared::update::DesktopRelease {
crates/vw-desktop/src/app/message/view/panel/settings_tests.rs:782:    use vw_shared::update::{DesktopRelease, UpdateSource};
crates/vw-desktop/src/app/message/view/panel/settings_tests.rs:809:    use vw_shared::update::UpdateSource;
crates/vw-desktop/src/app/message/view/panel/settings_tests.rs:827:    use vw_shared::update::UpdateSource;
crates/vw-desktop/src/app/message/view/panel/settings_tests.rs:845:    use vw_shared::update::{DesktopRelease, UpdateSource};
crates/vw-desktop/src/app/message/view/panel/settings_tests.rs:873:    use vw_shared::update::UpdateSource;
crates/vw-desktop/src/app/message/chat/stream/permission_tests.rs:35:fn session_info(id: &str, title: &str, directory: &str) -> vw_shared::session::info::Info {
crates/vw-desktop/src/app/message/chat/stream/permission_tests.rs:36:    vw_shared::session::info::Info {
crates/vw-desktop/src/app/message/chat/stream/permission_tests.rs:46:        time: vw_shared::session::info::TimeInfo {
crates/vw-desktop/src/app/impls/app_view_modals/permission_modal_tests.rs:291:    let question = vw_shared::question::Request {
crates/vw-desktop/src/app/impls/app_view_modals/permission_modal_tests.rs:294:        questions: vec![vw_shared::question::Info {
crates/vw-desktop/src/app/plugin_compat_t25_tests.rs:8:fn form() -> vw_shared::question::McpForm {
crates/vw-desktop/src/app/plugin_compat_t25_tests.rs:11:fn request(id: &str) -> vw_shared::question::Request {
crates/vw-desktop/src/app/message/view/panel/settings.rs:20:use vw_shared::update;
crates/vw-desktop/src/app/message/view/panel/settings.rs:1116:    _source: vw_shared::update::UpdateSource,
crates/vw-desktop/src/app/message/view/panel/settings.rs:1142:    _source: vw_shared::update::UpdateSource,
crates/vw-desktop/src/app/message/view/panel/settings.rs:1143:) -> Result<vw_shared::update::DesktopRelease, String> {
crates/vw-desktop/src/app/message/chat/stream/tests.rs:17:fn session_info(id: &str, title: &str, directory: &str) -> vw_shared::session::info::Info {
crates/vw-desktop/src/app/message/chat/stream/tests.rs:18:    vw_shared::session::info::Info {
crates/vw-desktop/src/app/message/chat/stream/tests.rs:28:        time: vw_shared::session::info::TimeInfo {
crates/vw-desktop/src/app/message/chat/stream/tests.rs:72:    info.summary = Some(vw_shared::session::info::Summary {
crates/vw-desktop/src/app/message/chat/stream/tests.rs:95:        vw_shared::session::info::Summary { additions: 2, deletions: 1, files: 1, diffs: None };
crates/vw-desktop/src/app/message/chat/stream/tests.rs:392:    visible.slug = vw_shared::session::info::Info::visible_task_slug("visible-task");
crates/vw-desktop/src/app/message/chat/stream/tests.rs:665:            preview: Some(vw_shared::session::ui_types::ChatSessionMeta {
crates/vw-desktop/src/app/i18n/chat.rs:122:                .__(vw_shared::session::session_utils::TASK_SESSION_SOURCE_DISPLAY_LABEL)
crates/vw-desktop/src/app/message/agent_apps_auth.rs:77:        result: Result<vw_shared::local_apps::Credentials, String>,
crates/vw-desktop/src/app/message/chat/stream/stream_events.rs:18:use vw_shared::message::types as message;
crates/vw-desktop/src/app/message/chat/stream/stream_events.rs:19:use vw_shared::session::info as session_info;
crates/vw-desktop/src/app/message/chat/stream/stream_events.rs:612:    let record = vw_shared::session::ui_types::SessionMessageRecord { seq_no, message_id, message };
crates/vw-desktop/src/app/message/chat/stream/stream_events.rs:655:    let delta = vw_shared::session::ui_types::SessionMessageDelta {
crates/vw-desktop/src/app/impls/app_view_modals/rename_tests.rs:10:fn session(id: &str, title: &str) -> vw_shared::session::info::Info {
crates/vw-desktop/src/app/impls/app_view_modals/rename_tests.rs:11:    vw_shared::session::info::Info {
crates/vw-desktop/src/app/impls/app_view_modals/rename_tests.rs:21:        time: vw_shared::session::info::TimeInfo {
crates/vw-desktop/src/app/message/app_providers/paths.rs:99:        client.endpoint().is_loopback() && scope.host == vw_shared::local_apps::HOST_ID;
crates/vw-desktop/src/app/views/project/layout/work_sidebar_tests.rs:6:use vw_shared::session::info::{Info, TimeInfo};
crates/vw-desktop/src/app/views/project/layout/work_sidebar_tests.rs:23:    vw_shared::session::ui_store::set_thread_pinned(temp.path(), project, "older", true).unwrap();
crates/vw-desktop/src/app/views/project/layout/work_sidebar_tests.rs:24:    let pinned = vw_shared::session::ui_store::load_pinned_threads(temp.path(), project).unwrap();
crates/vw-desktop/src/app/views/project/layout/work_sidebar_tests.rs:47:    vw_shared::session::ui_store::set_thread_pinned(temp.path(), project, "older", false).unwrap();
crates/vw-desktop/src/app/views/project/layout/work_sidebar_tests.rs:48:    let unpinned = vw_shared::session::ui_store::load_pinned_threads(temp.path(), project).unwrap();
crates/vw-desktop/src/app/views/project/layout/work_sidebar_tests.rs:345:    use vw_shared::session::info::Summary;
crates/vw-desktop/src/app/views/project/layout/work_sidebar_tests.rs:621:    let now = vw_shared::time::now_ms();
crates/vw-desktop/src/app/views/project/layout/work_sidebar_tests.rs:656:    let now = vw_shared::time::now_ms();
crates/vw-desktop/src/app/message/chat/stream/mod.rs:464:                                .session_get::<vw_shared::session::info::Info>(
crates/vw-desktop/src/app/message/chat/stream/permissions_tests.rs:23:fn session_info(id: &str, directory: &str) -> vw_shared::session::info::Info {
crates/vw-desktop/src/app/message/chat/stream/permissions_tests.rs:24:    vw_shared::session::info::Info {
crates/vw-desktop/src/app/message/chat/stream/permissions_tests.rs:34:        time: vw_shared::session::info::TimeInfo {
crates/vw-desktop/src/app/components/git_panel/ops_tests.rs:13:use vw_shared::session::info::{Info, TimeInfo};
crates/vw-desktop/src/app/impls/app_view_modals/chat_fork_ui_refresh_32_tests.rs:281:fn session_info(id: &str, directory: &str) -> vw_shared::session::info::Info {
crates/vw-desktop/src/app/message/chat/stream/stream_events_tests.rs:381:) -> vw_shared::session::info::Info {
crates/vw-desktop/src/app/message/chat/stream/stream_events_tests.rs:382:    vw_shared::session::info::Info {
crates/vw-desktop/src/app/message/chat/stream/stream_events_tests.rs:392:        time: vw_shared::session::info::TimeInfo {
crates/vw-desktop/src/app/message/chat/stream/stream_events_tests.rs:1593:    active_assistant.processing_timing = Some(vw_shared::session::ui_types::ProcessingTiming {
crates/vw-desktop/src/app/message/chat/stream/stream_events_tests.rs:1615:    background_assistant.processing_timing = Some(vw_shared::session::ui_types::ProcessingTiming {
crates/vw-desktop/src/app/message/chat/stream/stream_events_tests.rs:1716:    active_assistant.processing_timing = Some(vw_shared::session::ui_types::ProcessingTiming {
crates/vw-desktop/src/app/message/chat/stream/stream_events_tests.rs:1734:    assistant.processing_timing = Some(vw_shared::session::ui_types::ProcessingTiming {
crates/vw-desktop/src/app/message/chat/stream/stream_events_tests.rs:1764:    assistant.processing_timing = Some(vw_shared::session::ui_types::ProcessingTiming {
crates/vw-desktop/src/app/views/usage/data_tests.rs:6:use vw_shared::session::info::{Info, TimeInfo};
crates/vw-desktop/src/app/views/project/layout/work_sidebar.rs:125:type SessionRef<'a> = (&'a str, &'a vw_shared::session::info::Info);
crates/vw-desktop/src/app/views/project/layout/work_sidebar.rs:128:    sessions: impl Iterator<Item = (&'a str, &'a vw_shared::session::info::Info)>,
crates/vw-desktop/src/app/views/project/layout/work_sidebar.rs:216:    let cutoff = vw_shared::time::now_ms().saturating_sub(days * MILLIS_PER_DAY);
crates/vw-desktop/src/app/message/settings/models_tests.rs:9:use vw_shared::provider::types::{
crates/vw-desktop/src/app/message/view/panel/apps.rs:153:                        input_limit: vw_shared::provider::state::effective_input_window(&m.limit),
crates/vw-desktop/src/app/message/view/panel/apps.rs:155:                        limit_is_default: vw_shared::provider::state::is_default_model_limit(
crates/vw-desktop/src/app/message/view/panel/update_notifications_tests.rs:3:use vw_shared::update::{DesktopRelease, UpdateSource};
crates/vw-desktop/src/app/message/chat/stream/draft.rs:8:use vw_shared::session::info::Info;
crates/vw-desktop/src/app/message/chat/stream/draft.rs:9:use vw_shared::session::ui_types::ChatSessionMeta;
crates/vw-desktop/src/app/mcp_form.rs:2:pub(crate) fn valid(form: &vw_shared::question::McpForm, answers: &[Vec<String>]) -> bool {
crates/vw-desktop/src/app/impls/app_init_tests.rs:262:    use vw_shared::update::UpdateSource;
crates/vw-desktop/src/app/message/chat/stream/todos_tests.rs:8:fn todo(id: &str, status: &str) -> vw_shared::todo::Todo {
crates/vw-desktop/src/app/message/chat/stream/todos_tests.rs:9:    vw_shared::todo::Todo {
crates/vw-desktop/src/app/message/project/session/lifecycle.rs:13:use vw_shared::message::types as agent_message;
crates/vw-desktop/src/app/message/project/session/lifecycle.rs:17:pub(crate) fn cache_external_session(app: &mut App, info: vw_shared::session::info::Info) {
crates/vw-desktop/src/app/message/project/session/lifecycle.rs:24:    info: vw_shared::session::info::Info,
crates/vw-desktop/src/app/message/project/session/lifecycle.rs:48:    processing_timing: &mut Option<vw_shared::session::ui_types::ProcessingTiming>,
crates/vw-desktop/src/app/message/project/session/lifecycle.rs:72:    processing_timing: &mut Option<vw_shared::session::ui_types::ProcessingTiming>,
crates/vw-desktop/src/app/message/project/session/lifecycle.rs:285:) -> Option<vw_shared::session::ui_types::ProcessingTiming> {
crates/vw-desktop/src/app/message/project/session/lifecycle.rs:290:    Some(vw_shared::session::ui_types::ProcessingTiming {
crates/vw-desktop/src/app/message/project/session/lifecycle.rs:860:                        .session_update::<vw_shared::session::info::Info>(
crates/vw-desktop/src/app/message/project/session/lifecycle.rs:917:                                .session_get::<vw_shared::session::info::Info>(&next_id, None)
crates/vw-desktop/src/app/message/project/session/lifecycle.rs:958:                            .session_update::<vw_shared::session::info::Info>(
crates/vw-desktop/src/app/message/project/session/lifecycle.rs:964:                                        archived: Some(vw_shared::time::now_ms()),
crates/vw-desktop/src/app/message/project/session/lifecycle.rs:1073:                        .session_fork::<vw_shared::session::info::Info>(
crates/vw-desktop/src/app/task/executor/git_tests.rs:12:    let output = vw_shared::shell::git_std_command()
crates/vw-desktop/src/app/task/executor/git_tests.rs:441:    let merge = vw_shared::shell::git_std_command()
crates/vw-desktop/src/app/message/chat/stream/todos.rs:46:    res: Result<Vec<vw_shared::todo::Todo>, String>,
crates/vw-desktop/src/app/message/chat/stream/todos.rs:131:    res: Result<Vec<vw_shared::todo::Todo>, String>,
crates/vw-desktop/src/app/impls/app_view_modals/question_modal_tests.rs:13:fn option(label: &str, preview: Option<&str>) -> vw_shared::question::OptionInfo {
crates/vw-desktop/src/app/impls/app_view_modals/question_modal_tests.rs:14:    vw_shared::question::OptionInfo {
crates/vw-desktop/src/app/impls/app_view_modals/question_modal_tests.rs:23:    options: Vec<vw_shared::question::OptionInfo>,
crates/vw-desktop/src/app/impls/app_view_modals/question_modal_tests.rs:26:) -> vw_shared::question::Info {
crates/vw-desktop/src/app/impls/app_view_modals/question_modal_tests.rs:27:    vw_shared::question::Info {
crates/vw-desktop/src/app/impls/app_view_modals/question_modal_tests.rs:36:fn request(questions: Vec<vw_shared::question::Info>) -> vw_shared::question::Request {
crates/vw-desktop/src/app/impls/app_view_modals/question_modal_tests.rs:37:    vw_shared::question::Request {
crates/vw-desktop/src/app/message/project/session/lifecycle_tests.rs:10:use vw_shared::message::types as agent_message;
crates/vw-desktop/src/app/message/project/session/lifecycle_tests.rs:53:fn session_info(id: &str, title: &str, directory: &str) -> vw_shared::session::info::Info {
crates/vw-desktop/src/app/message/project/session/lifecycle_tests.rs:54:    vw_shared::session::info::Info {
crates/vw-desktop/src/app/message/project/session/lifecycle_tests.rs:64:        time: vw_shared::session::info::TimeInfo {
crates/vw-desktop/src/app/message/project/session/lifecycle_tests.rs:556:        Some(vw_shared::session::ui_types::ProcessingTiming {
crates/vw-desktop/src/app/task/executor/worktree_admin_tests.rs:48:    let output = vw_shared::shell::git_std_command()
crates/vw-desktop/src/app/i18n/gettext_area_15_tests.rs:36:                source: vw_shared::update::UpdateSource::Shiju,
crates/vw-desktop/src/app/task/executor/mod.rs:20:use vw_shared::session::session_utils::is_default_title;
crates/vw-desktop/src/app/task/executor/mod.rs:21:use vw_shared::shell::{resolve_executable, shell_profile_env_var, std_command};
crates/vw-desktop/src/app/message/project/session/open.rs:15:use vw_shared::session::info::Info;
crates/vw-desktop/src/app/message/project/session/open.rs:19:    existing: Option<Vec<vw_shared::session::info::Info>>,
crates/vw-desktop/src/app/message/project/session/open.rs:20:    incoming: Vec<vw_shared::session::info::Info>,
crates/vw-desktop/src/app/message/project/session/open.rs:21:) -> Vec<vw_shared::session::info::Info> {
crates/vw-desktop/src/app/message/project/session/open.rs:450:                app.project_is_git_repo = matches!(info.vcs, Some(vw_shared::project::Vcs::Git));
crates/vw-desktop/src/app/message/chat/stream/questions_tests.rs:18:fn option(label: &str) -> vw_shared::question::OptionInfo {
crates/vw-desktop/src/app/message/chat/stream/questions_tests.rs:19:    vw_shared::question::OptionInfo {
crates/vw-desktop/src/app/message/chat/stream/questions_tests.rs:31:) -> vw_shared::question::Info {
crates/vw-desktop/src/app/message/chat/stream/questions_tests.rs:32:    vw_shared::question::Info {
crates/vw-desktop/src/app/message/chat/stream/questions_tests.rs:41:fn request(id: &str, questions: Vec<vw_shared::question::Info>) -> vw_shared::question::Request {
crates/vw-desktop/src/app/message/chat/stream/questions_tests.rs:42:    vw_shared::question::Request {
crates/vw-desktop/src/app/task/executor/tests.rs:27:    let output = vw_shared::shell::git_std_command()
crates/vw-desktop/src/app/message/chat/stream/questions.rs:16:fn apply_question_request(app: &mut App, requests: Vec<vw_shared::question::Request>) {
crates/vw-desktop/src/app/message/chat/stream/questions.rs:66:    res: Result<Vec<vw_shared::question::Request>, String>,
crates/vw-desktop/src/app/message/settings/sessions_tests.rs:6:use vw_shared::session::info::{Info, TimeInfo};
crates/vw-desktop/src/app/message/project/session/open_tests.rs:85:        info: vw_shared::project::Info {
crates/vw-desktop/src/app/message/project/session/open_tests.rs:88:            vcs: Some(vw_shared::project::Vcs::Git),
crates/vw-desktop/src/app/message/project/session/open_tests.rs:92:            time: vw_shared::project::TimeInfo { created: 1, updated, initialized: None },
crates/vw-desktop/src/app/message/project/session/open_tests.rs:99:fn session_info(id: &str, additions: i64, deletions: i64) -> vw_shared::session::info::Info {
crates/vw-desktop/src/app/message/project/session/open_tests.rs:100:    vw_shared::session::info::Info {
crates/vw-desktop/src/app/message/project/session/open_tests.rs:106:        summary: Some(vw_shared::session::info::Summary {
crates/vw-desktop/src/app/message/project/session/open_tests.rs:115:        time: vw_shared::session::info::TimeInfo {
crates/vw-desktop/src/app/message/project/session/open_tests.rs:1362:    loaded.info.icon = Some(vw_shared::project::Icon {
crates/vw-desktop/src/app/message/project/session/open_tests.rs:1368:        Some(vw_shared::project::Commands { start: Some("cargo run".to_string()) });
crates/vw-desktop/src/app/message/project/session/open_tests.rs:1450:    use vw_shared::session::ui_types::SessionMessageRecord;
crates/vw-desktop/src/app/task/executor/git.rs:198:    let output = vw_shared::shell::git_std_command()
crates/vw-desktop/src/app/components/archived_chats_window_tests.rs:11:use vw_shared::session::info::Info;
crates/vw-desktop/src/app/message/chat/input/clipboard.rs:32:            match vw_shared::pasted_text::cache_large_pasted_text(&content) {
crates/vw-desktop/src/app/desktop_update.rs:2:use vw_shared::update::{DesktopRelease, UpdateSource};
crates/vw-desktop/src/app/message/project/mod.rs:12:use vw_shared::message::types as agent_message;
crates/vw-desktop/src/app/message/project/mod.rs:35:    pub info: vw_shared::project::Info,
crates/vw-desktop/src/app/message/project/mod.rs:79:        destination: Option<(String, Box<vw_shared::session::info::Info>)>,
crates/vw-desktop/src/app/message/project/mod.rs:282:    SessionCopied(vw_shared::session::info::Info),
crates/vw-desktop/src/app/message/project/mod.rs:283:    SessionCreated(vw_shared::session::info::Info),
crates/vw-desktop/src/app/message/project/mod.rs:284:    SessionsLoaded(Result<Vec<vw_shared::session::info::Info>, String>),
crates/vw-desktop/src/app/message/project/mod.rs:287:        result: Result<Vec<vw_shared::session::info::Info>, String>,
crates/vw-desktop/src/app/message/project/mod.rs:291:    ProjectSessionsLoaded(String, Result<Vec<vw_shared::session::info::Info>, String>),
crates/vw-desktop/src/app/state/agent_apps_auth.rs:45:    pub fn local(credentials: vw_shared::local_apps::Credentials) -> Self {
crates/vw-desktop/src/app/state/agent_apps_auth.rs:47:            host: vw_shared::local_apps::HOST_ID.into(),
crates/vw-desktop/src/app/state/agent_apps_auth.rs:55:            || self.host == vw_shared::local_apps::HOST_ID
crates/vw-desktop/src/app/message/settings/skills/storage_modal.rs:101:            if !client.endpoint().is_loopback() || scope.target_host_id != vw_shared::local_apps::HOST_ID {
crates/vw-desktop/src/app/message/project/helpers.rs:575:fn map_gateway_project(project: ProjectDto) -> vw_shared::project::Info {
crates/vw-desktop/src/app/message/project/helpers.rs:576:    vw_shared::project::Info {
crates/vw-desktop/src/app/message/project/helpers.rs:579:        vcs: project.git.is_repo.then_some(vw_shared::project::Vcs::Git),
crates/vw-desktop/src/app/message/project/helpers.rs:583:        time: vw_shared::project::TimeInfo {
crates/vw-desktop/src/app/message/project/helpers.rs:666:) -> Result<Vec<vw_shared::session::info::Info>, String> {
crates/vw-desktop/src/app/message/project/helpers.rs:670:    let mut sessions: Vec<vw_shared::session::info::Info> =
crates/vw-desktop/src/app/message/project/helpers.rs:679:    info: &vw_shared::session::info::Info,
crates/vw-desktop/src/app/state/agent.rs:462:    pub(crate) processing_timer: vw_shared::time::ActiveTimer,
crates/vw-desktop/src/app/state/agent.rs:464:    pub(crate) llm_timer: vw_shared::time::ActiveTimer,
crates/vw-desktop/src/app/state/agent.rs:489:    pub(crate) stream_save_dirty_records: Vec<vw_shared::session::ui_types::SessionMessageRecord>,
crates/vw-desktop/src/app/state/agent.rs:711:            processing_timer: vw_shared::time::ActiveTimer::default(),
crates/vw-desktop/src/app/state/agent.rs:712:            llm_timer: vw_shared::time::ActiveTimer::default(),
crates/vw-desktop/src/app/state/pet_tests.rs:58:fn session_info(id: &str, title: &str, directory: &str) -> vw_shared::session::info::Info {
crates/vw-desktop/src/app/state/pet_tests.rs:59:    vw_shared::session::info::Info {
crates/vw-desktop/src/app/state/pet_tests.rs:69:        time: vw_shared::session::info::TimeInfo {
crates/vw-desktop/src/app/state/app_state.rs:280:    pub(crate) chat_todo_items: Vec<vw_shared::todo::Todo>,
crates/vw-desktop/src/app/state/app_state.rs:401:    pub(crate) app_update_progress: Option<vw_shared::update::UpdateProgress>,
crates/vw-desktop/src/app/state/app_state.rs:404:        Option<std::sync::mpsc::Receiver<vw_shared::update::UpdateProgress>>,
crates/vw-desktop/src/app/state/app_state.rs:408:    pub(crate) question_modal_request: Option<vw_shared::question::Request>,
crates/vw-desktop/src/app/state/app_state.rs:516:    pub(crate) sessions: Vec<vw_shared::session::info::Info>,
crates/vw-desktop/src/app/state/app_state.rs:540:    pub(crate) project_sessions: HashMap<String, Vec<vw_shared::session::info::Info>>,
crates/vw-desktop/src/app/message/project/mod_tests.rs:33:    let info = vw_shared::project::Info {
crates/vw-desktop/src/app/message/project/mod_tests.rs:36:        vcs: Some(vw_shared::project::Vcs::Git),
crates/vw-desktop/src/app/message/project/mod_tests.rs:40:        time: vw_shared::project::TimeInfo { created: 1, updated: 2, initialized: None },
crates/vw-desktop/src/app/components/system_settings_apps/launch_tests.rs:235:        vw_shared::local_apps::initialize(home.path(), "fixture-platform", "fixture-company")
crates/vw-desktop/src/app/components/system_settings_apps/launch_tests.rs:262:            vw_shared::local_apps::HOST_ID,
crates/vw-desktop/src/app/components/chat_panel/message_view.rs:62:use vw_shared::session::ui_types::ProcessingTiming;
crates/vw-desktop/src/app/components/system_settings_apps/window_tests.rs:24:            AppsCatalogState { host: vw_shared::local_apps::HOST_ID.into(), ..Default::default() };
crates/vw-desktop/src/app/components/system_settings_apps/window_tests.rs:46:                            target_host_id: vw_shared::local_apps::HOST_ID.into(),
crates/vw-desktop/src/app/state/settings/archived_chats.rs:6:use vw_shared::session::info::Info;
crates/vw-desktop/src/app/message/chat/session.rs:21:use vw_shared::id;
crates/vw-desktop/src/app/message/chat/session.rs:22:use vw_shared::message::types as agent_message;
crates/vw-desktop/src/app/message/chat/session.rs:23:use vw_shared::session::info as session;
crates/vw-desktop/src/app/message/chat/session.rs:442:            let delta = vw_shared::session::ui_types::SessionMessageDelta {
crates/vw-desktop/src/app/message/chat/session.rs:863:            .is_some_and(vw_shared::session::info::Info::is_plan_run)
crates/vw-desktop/src/app/message/project/helpers_tests.rs:671:    assert!(matches!(info.vcs, Some(vw_shared::project::Vcs::Git)));
crates/vw-desktop/src/app/message/settings/archived_chats.rs:13:use vw_shared::session::info::Info;
crates/vw-desktop/src/app/components/system_settings_apps/launch_view.rs:121:            text(if catalog.host == vw_shared::local_apps::HOST_ID {
crates/vw-desktop/src/app/message/search/tests.rs:22:fn session_info(id: &str, directory: &str) -> vw_shared::session::info::Info {
crates/vw-desktop/src/app/message/search/tests.rs:23:    vw_shared::session::info::Info {
crates/vw-desktop/src/app/message/search/tests.rs:33:        time: vw_shared::session::info::TimeInfo {
crates/vw-desktop/src/app/state/app_state/core_fields.rs:102:    pub(crate) chat_todo_items: Vec<vw_shared::todo::Todo>,
crates/vw-desktop/src/app/components/chat_panel/message_view/text.rs:93:    vw_shared::session::session_utils::strip_task_session_source_marker(raw)
crates/vw-desktop/src/app/components/system_settings_apps/list.rs:58:    let location = if state.host == vw_shared::local_apps::HOST_ID {
crates/vw-desktop/src/app/components/system_settings_apps/list.rs:401:        && state.host == vw_shared::local_apps::HOST_ID
crates/vw-desktop/src/app/components/system_settings_apps/catalog_tests.rs:246:                result: Ok(vw_shared::local_apps::Credentials {
crates/vw-desktop/src/app/components/system_settings_apps/catalog_tests.rs:257:            result: Ok(vw_shared::local_apps::Credentials {
crates/vw-desktop/src/app/components/system_settings_apps/catalog_tests.rs:262:    assert_eq!(app.apps_catalog.host, vw_shared::local_apps::HOST_ID);
crates/vw-desktop/src/app/state/settings/archived_chats_tests.rs:2:use vw_shared::session::info::TimeInfo;
crates/vw-desktop/src/app/state/app_state/chrome_fields.rs:43:    pub(crate) app_update_progress: Option<vw_shared::update::UpdateProgress>,
crates/vw-desktop/src/app/state/app_state/chrome_fields.rs:46:        Option<std::sync::mpsc::Receiver<vw_shared::update::UpdateProgress>>,
crates/vw-desktop/src/app/state/app_state/chrome_fields.rs:50:    pub(crate) question_modal_request: Option<vw_shared::question::Request>,
crates/vw-desktop/src/app/state/app_state/chrome_fields.rs:151:    pub(crate) sessions: Vec<vw_shared::session::info::Info>,
crates/vw-desktop/src/app/state/app_state/chrome_fields.rs:170:    pub(crate) project_sessions: HashMap<String, Vec<vw_shared::session::info::Info>>,
crates/vw-desktop/src/app/message/settings/prompts/transfer.rs:68:    if matches!(event, Event::Reveal) && (!client.endpoint().is_loopback() || scope.target_host_id != vw_shared::local_apps::HOST_ID) {
crates/vw-desktop/src/app/components/system_settings_apps/local.rs:10:) -> Result<vw_shared::local_apps::Credentials, String> {
crates/vw-desktop/src/app/components/system_settings_apps/local.rs:17:) -> Result<vw_shared::local_apps::Credentials, String> {
crates/vw-desktop/src/app/components/system_settings_apps/local.rs:24:) -> Result<vw_shared::local_apps::Credentials, String> {
crates/vw-desktop/src/app/components/system_settings_apps/local.rs:36:                vw_shared::local_apps::initialize_auth
crates/vw-desktop/src/app/components/system_settings_apps/local.rs:38:                vw_shared::local_apps::initialize
crates/vw-desktop/src/app/components/system_settings_apps/local.rs:43:                vw_shared::local_apps::initialize
crates/vw-desktop/src/app/components/system_settings_apps/local.rs:69:    if host != vw_shared::local_apps::HOST_ID || !client.endpoint().is_loopback() {
crates/vw-desktop/src/app/components/system_settings_apps/local.rs:91:    let credentials = vw_shared::local_apps::restore(
crates/vw-desktop/src/app/components/system_settings_apps/launch_window_tests.rs:65:            self.state.host = vw_shared::local_apps::HOST_ID.into();
crates/vw-desktop/src/app/state/app_state/fields_navigation.rs:102:    pub(crate) chat_todo_items: Vec<vw_shared::todo::Todo>,
crates/vw-desktop/src/app/state/app_state/fields_navigation.rs:183:    pub(crate) app_update_progress: Option<vw_shared::update::UpdateProgress>,
crates/vw-desktop/src/app/state/app_state/fields_navigation.rs:186:        Option<std::sync::mpsc::Receiver<vw_shared::update::UpdateProgress>>,
crates/vw-desktop/src/app/state/app_state/fields_navigation.rs:190:    pub(crate) question_modal_request: Option<vw_shared::question::Request>,
crates/vw-desktop/src/app/state/app_state/fields_navigation.rs:293:    pub(crate) sessions: Vec<vw_shared::session::info::Info>,
crates/vw-desktop/src/app/state/app_state/fields_navigation.rs:312:    pub(crate) project_sessions: HashMap<String, Vec<vw_shared::session::info::Info>>,
crates/vw-desktop/src/app/components/chat_panel/message_view/tool_summaries.rs:551:            if let Ok(todos) = serde_json::from_str::<Vec<vw_shared::todo::Todo>>(output) {
crates/vw-desktop/src/app/components/chat_panel/question_view_tests.rs:6:fn request_with_message_id(message_id: &str) -> vw_shared::question::Request {
crates/vw-desktop/src/app/components/chat_panel/question_view_tests.rs:7:    vw_shared::question::Request {
crates/vw-desktop/src/app/components/chat_panel/question_view_tests.rs:12:        tool: Some(vw_shared::question::ToolMeta {
crates/vw-desktop/src/app/components/chat_panel/question_view_tests.rs:29:    let request = vw_shared::question::Request {
crates/vw-desktop/src/app/components/chat_panel/header.rs:169:) -> Option<&'a vw_shared::session::info::Info> {
crates/vw-desktop/src/app/components/chat_panel/header_tests.rs:7:use vw_shared::session::info::{Info, TimeInfo};
crates/vw-desktop/src/app/components/chat_panel/message_view_tests.rs:25:use vw_shared::session::ui_types::ProcessingTiming;
crates/vw-desktop/src/app/components/search_panel_tests.rs:11:fn session(id: &str, title: &str, directory: &str) -> vw_shared::session::info::Info {
crates/vw-desktop/src/app/components/search_panel_tests.rs:12:    vw_shared::session::info::Info {
crates/vw-desktop/src/app/components/search_panel_tests.rs:22:        time: vw_shared::session::info::TimeInfo {
crates/vw-desktop/src/app/components/search_panel_tests.rs:126:    workflow.slug = vw_shared::session::info::Info::plan_run_slug(&workflow.slug);
crates/vw-desktop/src/app/components/search_panel_tests.rs:131:    task.slug = vw_shared::session::info::Info::visible_task_slug(&task.slug);
crates/vw-desktop/src/app/components/chat_panel/mod_tests.rs:21:use vw_shared::todo::Todo;
crates/vw-desktop/src/app/components/chat_panel/message_view/message_view_tests.rs:12:use vw_shared::session::ui_types::ProcessingTiming;
crates/vw-desktop/src/app/message/settings/agents_tests.rs:21:use vw_shared::provider::types::{
crates/vw-desktop/src/app/components/install_web_modal.rs:39:        Some(vw_shared::update::UpdateSource::Shiju) => Some(i18n.__("shiju（默认）")),
crates/vw-desktop/src/app/components/install_web_modal.rs:40:        Some(vw_shared::update::UpdateSource::Github) => Some("github.com"),
crates/vw-desktop/src/app/components/install_web_modal.rs:49:                    vw_shared::update::UpdateSource::Github
crates/vw-desktop/src/app/components/install_web_modal.rs:51:                    vw_shared::update::UpdateSource::Shiju
crates/vw-desktop/src/app/components/chat_panel/trajectory/projection.rs:6:use vw_shared::session::ui_types::{ChatSessionStep, TokenUsage};
crates/vw-desktop/src/app/components/chat_panel/message_view/text_tests.rs:111:    let encoded = vw_shared::session::session_utils::encode_task_session_source_marker(
crates/vw-desktop/src/app/components/agent_apps/hermes_dashboard.rs:69:    let local = client.endpoint().is_loopback() && app.apps_catalog.host == vw_shared::local_apps::HOST_ID;
crates/vw-desktop/src/app/components/about_modal_ui_refresh_41_tests.rs:301:        app.desktop_update.source = Some(vw_shared::update::UpdateSource::Shiju);
crates/vw-desktop/src/app/components/about_modal_ui_refresh_41_tests.rs:349:                    vw_shared::update::UpdateSource::Github
crates/vw-desktop/src/app/components/about_modal_ui_refresh_41_tests.rs:550:    app.desktop_update.source = Some(vw_shared::update::UpdateSource::Github);
crates/vw-desktop/src/app/components/chat_panel/utils/app_state_tests.rs:10:fn session_info(id: &str, title: &str) -> vw_shared::session::info::Info {
crates/vw-desktop/src/app/components/chat_panel/utils/app_state_tests.rs:11:    vw_shared::session::info::Info {
crates/vw-desktop/src/app/components/chat_panel/utils/app_state_tests.rs:21:        time: vw_shared::session::info::TimeInfo {
crates/vw-desktop/src/app/components/agent_apps/external_sessions/paths.rs:89:    let local = cfg!(target_os = "macos") && session.identity.target_host == vw_shared::local_apps::HOST_ID
crates/vw-desktop/src/app/components/toast_ui_refresh_33_tests.rs:496:fn question_option(label: &str, preview: Option<&str>) -> vw_shared::question::OptionInfo {
crates/vw-desktop/src/app/components/toast_ui_refresh_33_tests.rs:497:    vw_shared::question::OptionInfo {
crates/vw-desktop/src/app/components/toast_ui_refresh_33_tests.rs:518:    let request = vw_shared::question::Request {
crates/vw-desktop/src/app/components/toast_ui_refresh_33_tests.rs:522:            vw_shared::question::Info {
crates/vw-desktop/src/app/components/toast_ui_refresh_33_tests.rs:532:            vw_shared::question::Info {
crates/vw-desktop/src/app/components/install_cli_modal_tests.rs:22:) -> vw_shared::update::UpdateProgress {
crates/vw-desktop/src/app/components/install_cli_modal_tests.rs:23:    vw_shared::update::UpdateProgress {
crates/vw-desktop/src/app/components/install_cli_modal_tests.rs:253:    app.desktop_update.release = Some(vw_shared::update::DesktopRelease {
crates/vw-desktop/src/app/components/install_cli_modal_tests.rs:254:        source: vw_shared::update::UpdateSource::Shiju,
crates/vw-desktop/src/app/components/chat_panel/trajectory/projection_tests.rs:7:use vw_shared::session::ui_types::{ChatSessionStep, TokenUsage, ToolTiming};
crates/vw-desktop/src/app/message/settings/sessions.rs:5:use vw_shared::message::types as message;
crates/vw-desktop/src/app/message/settings/sessions.rs:6:use vw_shared::session::info;
crates/vw-desktop/src/app/components/chat_panel/tools/apply_patch_preview.rs:7:use vw_shared::patch::{Hunk, parse_patch};
crates/vw-desktop/src/app/components/chat_panel/tools/todo_views_tests.rs:9:use vw_shared::todo::Todo;
crates/vw-desktop/src/app/components/archived_chats_window/gateway.rs:8:use vw_shared::session::info::Info;
crates/vw-desktop/src/app/components/chat_panel/tools/question_view_tests.rs:11:use vw_shared::question::{Info, Request, ToolMeta};
crates/vw-desktop/src/app/components/archived_chats_window/native.rs:42:    let _: vw_shared::session::info::Info = client
crates/vw-desktop/src/app/components/install_cli_modal.rs:144:    update_progress: Option<&'a vw_shared::update::UpdateProgress>,
crates/vw-desktop/src/app/components/install_cli_modal.rs:200:    update_progress: Option<&'a vw_shared::update::UpdateProgress>,
crates/vw-desktop/src/app/components/install_cli_modal.rs:397:            Some(vw_shared::update::UpdateSource::Shiju) => Some(i18n.__("shiju（默认）")),
crates/vw-desktop/src/app/components/install_cli_modal.rs:398:            Some(vw_shared::update::UpdateSource::Github) => Some("github.com"),
crates/vw-desktop/src/app/components/install_cli_modal.rs:407:                        vw_shared::update::UpdateSource::Github
crates/vw-desktop/src/app/components/install_cli_modal.rs:409:                        vw_shared::update::UpdateSource::Shiju
crates/vw-desktop/src/app/components/install_cli_modal.rs:494:    progress: &'a vw_shared::update::UpdateProgress,
crates/vw-desktop/src/app/components/install_cli_modal.rs:649:    progress: &'a vw_shared::update::UpdateProgress,
crates/vw-desktop/src/app/message/settings/models.rs:12:use vw_shared::provider::types as provider_types;
crates/vw-desktop/src/app/message/settings/models.rs:13:use vw_shared::provider::types::Info;
crates/vw-desktop/src/app/components/system_settings_auth.rs:121:        let host = if connection.host == vw_shared::local_apps::HOST_ID {
crates/vw-desktop/src/app/message/settings/agents.rs:20:use vw_shared::provider::types as provider_types;
crates/vw-desktop/src/app/message/settings/agents.rs:21:use vw_shared::provider::types::Info;
crates/vw-desktop/src/app/message/settings/providers/models_tests.rs:677:    let resolved = vw_shared::provider::state::resolve_model_limit(None, None, None);
crates/vw-desktop/src/app/message/settings/providers/models_tests.rs:678:    assert_eq!(vw_shared::provider::state::effective_input_window(&resolved), 200_000);
crates/vw-desktop/src/app/components/input_panel/agent_sessions_tests.rs:15:use vw_shared::session::info::{Info, TimeInfo};
crates/vw-desktop/src/app/message/settings/util.rs:3:use vw_shared::provider::types::Info;
crates/vw-desktop/src/app/message/settings/util.rs:22:pub fn is_provider_connected(provider: &vw_shared::provider::types::Info) -> bool {
crates/vw-desktop/src/app/message/settings/util.rs:28:            vw_shared::provider::types::ProviderSource::Api
crates/vw-desktop/src/app/message/settings/util.rs:29:                | vw_shared::provider::types::ProviderSource::Env
crates/vw-desktop/src/app/components/input_panel/reasoning_selector_tests.rs:222:    let raw: vw_shared::provider::models::Provider =
crates/vw-desktop/src/app/components/input_panel/reasoning_selector_tests.rs:232:                let model = vw_shared::provider::state::from_models_dev_model(&raw, raw_model);
crates/vw-desktop/src/app/components/input_panel/todo_panel.rs:29:use vw_shared::todo::Todo;
crates/vw-desktop/src/app/message/settings/util_tests.rs:5:use vw_shared::provider::types::{Info, ProviderSource};
crates/vw-desktop/src/app/message/settings/types.rs:3:use vw_shared::provider::types as model_provider;
crates/vw-desktop/src/app/message/settings/providers/managed_shiju.rs:11:use vw_shared::auth;
crates/vw-desktop/src/app/components/chat_panel/tools/question_view.rs:45:    request: Option<&vw_shared::question::Request>,
crates/vw-desktop/src/app/components/chat_panel/tools/question_view.rs:95:pub(super) fn parse_questions(input: &str) -> Vec<vw_shared::question::Info> {
crates/vw-desktop/src/app/components/chat_panel/tools/question_view.rs:108:    serde_json::from_value::<Vec<vw_shared::question::Info>>(questions).ok().unwrap_or_default()
crates/vw-desktop/src/app/components/chat_panel/tools/question_view.rs:165:    questions: &[vw_shared::question::Info],
crates/vw-desktop/src/app/components/input_panel/tests.rs:21:use vw_shared::todo::Todo;
crates/vw-desktop/src/app/components/input_panel/agent_sessions.rs:17:use vw_shared::session::info::Info;
crates/vw-desktop/src/app/message/settings/providers/helpers_tests.rs:6:use vw_shared::provider::types::{
crates/vw-desktop/src/app/message/settings/providers/helpers.rs:7:use vw_shared::provider::models as raw_model_provider;
crates/vw-desktop/src/app/message/settings/providers/helpers.rs:8:use vw_shared::provider::types as model_provider;
crates/vw-desktop/src/app/message/settings/apps_catalog.rs:33:        result: Result<vw_shared::local_apps::Credentials, String>,
crates/vw-desktop/src/app/message/settings/apps_catalog.rs:218:                    app.apps_catalog.host = vw_shared::local_apps::HOST_ID.into();
crates/vw-desktop/src/app/message/settings/apps_catalog.rs:334:            let local = client.endpoint().is_loopback() && host == vw_shared::local_apps::HOST_ID;
crates/vw-desktop/src/app/components/chat_panel/tools/todo_views.rs:17:use vw_shared::todo::Todo;
crates/vw-desktop/src/app/components/input_panel/mod.rs:660:    let todo_items: &[vw_shared::todo::Todo] = if app.chat_todo_session_id == app.active_session_id
crates/vw-desktop/src/app/message/settings/data_management/locations_ui_tests.rs:15:    assert!(!can_reveal(false, vw_shared::local_apps::HOST_ID));
crates/vw-desktop/src/app/message/settings/data_management/locations_ui_tests.rs:16:    assert!(can_reveal(true, vw_shared::local_apps::HOST_ID));
crates/vw-desktop/src/app/message/settings/data_management/locations_ui.rs:8:    loopback && host == vw_shared::local_apps::HOST_ID
crates/vw-desktop/src/app/components/input_panel/reasoning_selector.rs:63:        let declared: Option<Vec<vw_shared::provider::reasoning::ReasoningOption>> = detail
crates/vw-desktop/src/app/components/input_panel/reasoning_selector.rs:66:        let levels = vw_shared::provider::reasoning::reasoning_levels(
crates/vw-desktop/src/app/message/settings/providers/models.rs:10:use vw_shared::auth::{self as shared_auth};
crates/vw-desktop/src/app/components/input_panel/todo_panel_tests.rs:11:use vw_shared::todo::Todo;
```
