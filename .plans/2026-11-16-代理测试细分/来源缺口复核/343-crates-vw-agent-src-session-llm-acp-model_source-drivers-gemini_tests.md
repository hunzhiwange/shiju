# 来源复核343：src/session/llm/acp/model_source/drivers/gemini_tests

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】；依赖：无。

状态：未开始

目的：复核 `crates/vw-agent/src/session/llm/acp/model_source/drivers/gemini_tests.rs` 的具体测试归属与真实宿主依赖。来源：任务133未关联候选，不是已确认遗漏实现。一个实际源模块；混合独立行为时先补窄实现卡。

## 源码和声明证据

- 当前候选：`crates/vw-agent/src/session/llm/acp/model_source/drivers/gemini_tests.rs`。
- 快照 SHA256：`4b782749d5000e13ad1a563f96a73361d48290ecfff4c4b2ae7b903e3488a9f5`。源码变化后重新阅读。
- 快照词法属性 6 个，不是当前 cfg 展开后的有效测试数。

| crates/vw-agent/src/session/llm/acp/model_source/drivers/gemini.rs:13 | crates/vw-agent/src/session/llm/acp/model_source/drivers/gemini_tests.rs | mod tests | cfg(not(target_arch="wasm32")); cfg(all(test,unix)) | axum | 未匹配→133 | 未发现 |

## 身份线索

- `acp_model_source_gemini_private_configuration_routes_all_models`
- `acp_model_source_gemini_rejects_remote_endpoint`
- `acp_model_source_gemini_subprocess_sends_private_key_to_bound_model`
- `acp_model_source_gemini_rejects_configuration_conflicts_before_publication`
- `acp_model_source_gemini_project_cli_override_reaches_actual_subprocess`
- `model_protocol_013_gemini_startup_accepts_native_and_each_converted_protocol`

## 步骤与验收

从当前真实父声明追踪 cfg/path/include 和包目标；核对全部原断言、共享 fixture、环境锁、进程、网络及资源释放。与 ../审计记录、实施提交和所属库现有测试逐项核对，不以文件名或同名属性作等价证明。按生产符号所有者判定每项迁移或保留理由；纯转发不自动构成真实 Host 必要性。需要迁移时先按真实行为补独立实现卡并保留来源证据，不在本审计任务吞并多个行为。

本轮不执行 Cargo 或测试。审计证据需包含完整旧→新身份、每项原断言的归属、真实父接线、cfg、fixture 和源码行数去重口径；未能确认项明确列出。为以后统一验证记录实际包/目标和完整精确过滤名，按有效 cfg 核对预期数，不将上述词法值当通过证据。无新运行证据，不声称来源覆盖或验收通过。
