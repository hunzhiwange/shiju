# 来源复核010：agent-gateway/src/host_acceptance_tests/pairing

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】；依赖：无。

状态：未开始

目的：复核 `crates/libs/agent-gateway/src/host_acceptance_tests/pairing.rs` 的具体测试归属与真实宿主依赖。来源：任务133未关联候选，不是已确认遗漏实现。一个实际源模块；混合独立行为时先补窄实现卡。

## 源码和声明证据

- 当前候选：`crates/libs/agent-gateway/src/host_acceptance_tests/pairing.rs`。
- 快照 SHA256：`400cb1bdeb296f9fb53c1710ce31e9fe6a6b63fd6f97b38ed0a5e8b91dca3738`。源码变化后重新阅读。
- 快照词法属性 14 个，不是当前 cfg 展开后的有效测试数。

| crates/vw-agent/src/gateway/tests/mod.rs:442 | crates/libs/agent-gateway/src/host_acceptance_tests/pairing.rs | mod pairing | cfg(not(target_arch="wasm32")); cfg(test) | agent-gateway（物理所有者；生产声明见该库 src/lib.rs） | 未匹配→133 | 未发现 |

## 身份线索

- `persist_pairing_tokens_writes_config_tokens`
- `handle_pair_returns_200_with_token_on_valid_code`
- `handle_pair_token_authenticates_after_pairing`
- `handle_pair_code_returns_loopback_bootstrap_code_when_unpaired`
- `handle_pair_code_rejects_non_loopback_clients`
- `handle_pair_returns_403_on_wrong_code`
- `handle_pair_returns_403_when_no_code_header`
- `handle_pair_returns_403_on_empty_code`
- `handle_pair_returns_403_when_pairing_mode_disabled`
- `handle_pair_returns_429_when_rate_limit_exceeded`
- `handle_pair_returns_429_after_brute_force_lockout`
- `handle_pair_accepts_pre_seeded_token_immediately`
- `handle_pair_success_body_has_paired_true_and_token`
- `handle_pair_failure_body_has_error_field`

## 步骤与验收

从当前真实父声明追踪 cfg/path/include 和包目标；核对全部原断言、共享 fixture、环境锁、进程、网络及资源释放。与 ../审计记录、实施提交和所属库现有测试逐项核对，不以文件名或同名属性作等价证明。按生产符号所有者判定每项迁移或保留理由；纯转发不自动构成真实 Host 必要性。需要迁移时先按真实行为补独立实现卡并保留来源证据，不在本审计任务吞并多个行为。

本轮不执行 Cargo 或测试。审计证据需包含完整旧→新身份、每项原断言的归属、真实父接线、cfg、fixture 和源码行数去重口径；未能确认项明确列出。为以后统一验证记录实际包/目标和完整精确过滤名，按有效 cfg 核对预期数，不将上述词法值当通过证据。无新运行证据，不声称来源覆盖或验收通过。
