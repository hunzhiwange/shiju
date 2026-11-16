# 来源复核285：src/model_relay/server_tests

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】；依赖：无。

状态：未开始

目的：复核 `crates/vw-agent/src/model_relay/server_tests.rs` 的具体测试归属与真实宿主依赖。来源：任务133未关联候选，不是已确认遗漏实现。一个实际源模块；混合独立行为时先补窄实现卡。

## 源码和声明证据

- 当前候选：`crates/vw-agent/src/model_relay/server_tests.rs`。
- 快照 SHA256：`1a6db98a2c28750922b4d2a2f4269899e7d29d7934bf542b5afe6c4f92435a77`。源码变化后重新阅读。
- 快照词法属性 4 个，不是当前 cfg 展开后的有效测试数。

| crates/vw-agent/src/model_relay/mod.rs:10 | crates/vw-agent/src/model_relay/server_tests.rs | mod server_tests | cfg(not(target_arch="wasm32")); cfg(test) | axum, tokio_util, sha2, futures_util, tokio | 未匹配→133 | 未发现 |

## 身份线索

- `kiro_protocol_flag_is_local_but_catalog_still_requires_selected_model`
- `acp_model_source_relay_binding_http_isolation_limits_and_lifecycle`
- `acp_model_source_relay_expiry_cancels_an_established_http_stream`
- `acp_model_source_relay_binding_upstream_redirect_is_not_followed`

## 步骤与验收

从当前真实父声明追踪 cfg/path/include 和包目标；核对全部原断言、共享 fixture、环境锁、进程、网络及资源释放。与 ../审计记录、实施提交和所属库现有测试逐项核对，不以文件名或同名属性作等价证明。按生产符号所有者判定每项迁移或保留理由；纯转发不自动构成真实 Host 必要性。需要迁移时先按真实行为补独立实现卡并保留来源证据，不在本审计任务吞并多个行为。

本轮不执行 Cargo 或测试。审计证据需包含完整旧→新身份、每项原断言的归属、真实父接线、cfg、fixture 和源码行数去重口径；未能确认项明确列出。为以后统一验证记录实际包/目标和完整精确过滤名，按有效 cfg 核对预期数，不将上述词法值当通过证据。无新运行证据，不声称来源覆盖或验收通过。
