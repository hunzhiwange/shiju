# 余额、套餐额度与查询脚本来源覆盖

固定来源为 CC Switch 4.0.25，提交 `ca58c0313360eae7c851fbd7553c1614c7af03e7`。六组证据合计 44 个文件，含原始清单中归属母任务 7 的全部 40 个文件，以及第 6 组补查的 4 个跨归属文件。每组保留源码行号、输入输出、失败分支及实现和验收归属。下表是后续补填实现与测试证据的入口，查询边界见[查询合同](contract.md)。[原始清单](../../../../.plans/2026-11-05-CC能力完整融合/来源清单.json) · [来源覆盖索引](../../../../.plans/2026-11-05-余额套餐额度与查询脚本/来源覆盖索引.md)

| 批次 | 文件数 | 主要证据 | 实现／测试交接 |
| --- | ---: | --- | --- |
| [1](coverage/source-1.md) | 7 | 余额和套餐命令、官方订阅命令、AGY 额度、Codex 额度保护、封面资源 | 任务 11–16 保留币种及钱包语义，26–32 消费账号额度，44–48 提供 API 和 SDK。封面由 59 归档。 |
| [2](coverage/source-2.md) | 7 | Coding Plan 分派、官方订阅与 AGY/Grok、QuickJS、账号页脚 | 任务 10、17–30 补原生适配和响应夹具，33–37 补脚本模板、资源、网络、结果和取消测试。 |
| [3](coverage/source-3.md) | 7 | Codex、Copilot、Kimi、xAI 与通用订阅卡，账号和明细展示 | 任务 9 确认账号解析，27、29、31–32 补适配测试，41–43、49–50、53 验证投影和共享刷新。 |
| [4](coverage/source-4.md) | 7 | 多窗口、Credits、重置、余额与失败投影，显示偏好、批量刷新和套餐检测回归 | 任务 10、38–43、49–53 补测试和 UI 交互，58 验收实际 macOS 界面。 |
| [5](coverage/source-5.md) | 7 | TS 命令及结果合同、托管绑定、缓存与调度、套餐检测及组件回归 | 任务 8–10 固化类型和身份，39–40 迁移缓存与调度，44–48、53 验证接口、SDK 和联动。 |
| [6](coverage/source-6.md) | 9 | 5 个组件/绑定回归文件，另有脚本弹窗、短期成功缓存、TS 和 Rust 的配置与结果合同 | 任务 9、33–43、45–46、49–53 按来源断言补实现与测试。 |

第 6 组的交叉证据包含 `src/types.ts` 和 `src-tauri/src/provider.rs`。源快照将计划中的 `src/types/index.ts` 放在 `src/types.ts`。本批对六组的精确来源重新校验 SHA-256，并核对索引追加的以下 4 个文件，合计 48 个独立来源。

| 交叉入口 | 行为及所有者 |
| --- | --- |
| [Copilot 额度命令](../../../../crates/vw-agent-apps/upstream/src-tauri/src/commands/copilot.rs#L201-L221) | 任务 31 以所选账号获取额度，登录及令牌派生归母任务 6。 |
| [Codex 额度命令](../../../../crates/vw-agent-apps/upstream/src-tauri/src/commands/codex_oauth.rs#L28-L96) | 任务 27 按托管账号和工作区查询，任务 9 处理账号选择，凭据轮换归母任务 6。 |
| [xAI 额度命令](../../../../crates/vw-agent-apps/upstream/src-tauri/src/commands/xai_oauth.rs#L29-L75) | 任务 29 保留 xAI 账号与额度失败语义，账号授权和刷新归母任务 6。 |
| [Kimi 账号服务](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/kimi_oauth/account.rs#L379-L447) | 任务 32 消费分区账号凭据及用量响应，默认账号和原生槽归母任务 6。 |

后续在对应来源条目下补实现文件、测试入口和结果。任务 55–58 分别验收余额、Coding Plan、官方订阅及 macOS 界面，任务 59 核对交接。Codex 额度保护的阈值拦截归母任务 8，系统设置归母任务 20，账号删除与绑定引用归母任务 6。

本批验证记录为 48/48 个来源 SHA-256 匹配，8 份文档中的 331 处链接均可定位（318 处仓内路径、13 处固定上游提交，含 281 处行号引用）。上游链接用固定提交的 Git 对象核验。计划及来源索引保持只读，限定范围的 `git diff --check` 退出码为 0。

## 任务 8 查询结果类型

| 来源 | 实现和测试 |
| --- | --- |
| [订阅及凭据状态](../../../../crates/vw-agent-apps/upstream/src/types/subscription.ts#L1-L55) | `UsageQueryResult` 分开表达来源、可空查询时间、凭据五态和成功或失败。`UsageSubscription` 保留订阅档位、多窗口、超额用量、Credits 余额和逐次重置的到期时间。 |
| [钱包查询](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/balance.rs#L116-L153) | `UsageWallet` 逐钱包保留币种、余额、赠金和充值额。零、负值及缺失分别表达。 |
| [多套餐结果](../../../../crates/vw-agent-apps/upstream/src-tauri/src/provider.rs#L340-L372) | `UsageQueryData` 按类别承载余额、官方订阅、编程套餐或脚本结果。编程套餐可返回多窗口，脚本可返回多个带单位及有效状态的套餐。 |

`UsageNumber` 在构造和反序列化时拒绝非有限数值。`serde_json` 的 `float_roundtrip` 保留有限 `f64` 的 JSON 往返精度。[查询类型](../../../../crates/vw-api-types/src/apps_usage.rs) · [独立行为测试](../../../../crates/vw-api-types/src/apps_usage_tests.rs)

验证环境为 macOS 27.0 (26A428)、arm64，`vw-api-types` 0.6.0。

| 阶段 | 槽位 | 命令 | 结果 |
| --- | --- | --- | --- |
| 实现前 | 5 | `cargo test -p vw-api-types usage_types_ -- --nocapture` | 退出 101，`usage_types_returns_expected_result` 和 `usage_types_rejects_invalid_input` 因查询模块尚未定义而遇到 E0432，执行 0 项。 |
| 格式化 | 6 | `cargo fmt --all` | 退出 0，在独立源码副本运行，仅同步任务 8 的 Rust 文件。 |
| 精度回归 | 6 | `cargo test -p vw-api-types usage_types_ -- --nocapture` | 退出 101，2 项通过，`usage_types_preserves_finite_number_precision` 检出默认 JSON 解析偏差 1 ULP。 |
| 实现后 | 6 | `cargo test -p vw-api-types usage_types_ -- --nocapture` | 退出 0，3 项通过，0 项失败。 |
| 工作区检查 | 6 | `cargo clippy --all-targets --all-features -- -D warnings` | 退出 0。 |

槽位 5 的 `CARGO_TARGET_DIR` 为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-5`，槽位 6 为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-6`。初轮沿用了仓库既有 `build.build-dir`。最终验证将 `CARGO_BUILD_BUILD_DIR` 指向同一槽位，聚焦测试从槽位 6 的二进制执行。

限定提交范围的 `git diff --cached --check` 退出 0。行为测试使用 DTO 夹具，真实账号验收分别由任务 55–57 承担。

## 任务 9 查询账号与凭据定位

| 来源 | 实现和测试 |
| --- | --- |
| [显式绑定](../../../../crates/vw-agent-apps/upstream/src/lib/authBinding.ts#L3-L20) · [默认账号规则](../../../../crates/vw-agent-apps/upstream/src/lib/managedAccountUsage.ts#L46-L109) · [上游行为测试](../../../../crates/vw-agent-apps/upstream/tests/lib/managedAccountUsage.test.ts) | `resolve_usage_credential` 先核对租户、主体、主机与服务，再使用授权供应商记录的绑定或明确指定的账号。仅适配器允许时读取同作用域的托管默认账号。绑定账号失效或与指定账号冲突时返回类型化错误。 |
| [Codex 本机登录](../../../../crates/vw-agent-apps/upstream/src/lib/managedAccountUsage.ts#L50-L66) · [AGY 显式绑定](../../../../crates/vw-agent-apps/upstream/src/components/providers/ProviderCard.tsx#L293-L306) | Codex 卡未绑定时可定位至授权主体独占的 CLI 登录槽。AGY 供应商额度要求显式绑定。托管账号定位与本机槽定位均返回宿主内部引用。 |

解析使用现有 `AuthAccountScope` 和 `AuthErrorCode`。执行端在调用前核验项目、供应商与凭据使用授权，适配器通过宿主服务取用凭据。网关连线由任务 44–46 处理。[解析实现](../../../../crates/vw-agent-apps/src/usage/binding.rs) · [独立行为测试](../../../../crates/vw-agent-apps/src/usage/binding_tests.rs)

验证环境为 macOS 27.0 (26A428)、arm64，`vw-agent-apps` 0.6.0。实现前后均领取槽位 2，`CARGO_TARGET_DIR` 和 `CARGO_BUILD_BUILD_DIR` 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。

| 阶段 | 命令 | 结果 |
| --- | --- | --- |
| 实现前 | `cargo test -p vw-agent-apps usage_binding_ -- --nocapture` | 退出 101，`usage_binding_returns_expected_result` 和 `usage_binding_rejects_invalid_input` 因模块 `super::binding` 缺失遇到 E0432，执行 0 项。 |
| 格式化 | `cargo fmt --all` | 退出 0，在独立源码副本执行，仅同步本任务的 Rust 文件。 |
| 实现后 | `cargo test -p vw-agent-apps usage_binding_ -- --nocapture` | 退出 0，3 项通过，0 项失败。跨租户用例使用现有凭据文件存储和临时夹具。 |
| 工作区检查 | `cargo clippy --all-targets --all-features -- -D warnings` | 退出 0。 |

限定提交范围的 `git diff --cached --check` 退出 0。槽位锁已释放，编译缓存保留。真实账号验收见任务 55–57。

## 任务 10 原生查询能力识别

| 来源 | 实现和测试 |
| --- | --- |
| [余额模板检测](../../../../crates/vw-agent-apps/upstream/src/components/UsageScriptModal.tsx#L184-L200) · [余额分派](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/balance.rs#L27-L44) | `detect_native_usage` 用解析后的主机识别五家余额供应商，保留 SiliconFlow 境内与国际分支。 |
| [套餐检测与启用规则](../../../../crates/vw-agent-apps/upstream/src/config/codingPlanProviders.ts#L25-L113) · [套餐分派](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/coding_plan.rs#L12-L82) · [Kimi 预设](../../../../crates/vw-agent-apps/upstream/src/config/claudeProviderPresets.ts#L173-L181) | 依实际主机和大小写敏感的完整路径段识别套餐。MiniMax 保留真实子域匹配，Kimi 同时识别 `api.kimi.com` 与已交付预设使用的 `api.kimi.ai`。火山仅识别方舟地区主机的两类套餐入口，`enabled_by_default` 返回关闭。 |
| [智谱团队显式分派](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/coding_plan.rs#L1858-L1877) · [上游检测测试](../../../../crates/vw-agent-apps/upstream/src/config/codingPlanProviders.test.ts#L20-L62) | `ZhipuPlanSelection::Team` 要求显式选择及 `open.bigmodel.cn` 主机。伪造主机、出现在 URL 其他部位的域名、路径前缀混淆及按量入口均有拒绝用例。 |

识别结果由 `NativeUsageCapability` 区分余额与套餐。无匹配能力、非法地址与团队配置冲突分别返回无结果或类型化错误。验证覆盖纯识别逻辑，真实账号查询验收见任务 55–57。[识别实现](../../../../crates/vw-agent-apps/src/usage/detect.rs) · [独立行为测试](../../../../crates/vw-agent-apps/src/usage/detect_tests.rs)

验证环境为 macOS 27.0 (26A428)、arm64，`vw-agent-apps` 0.6.0。
两阶段均领取槽位 3，`CARGO_TARGET_DIR` 与 `CARGO_BUILD_BUILD_DIR` 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-3`。

| 阶段 | 命令 | 结果 |
| --- | --- | --- |
| 实现前 | `cargo test -p vw-agent-apps usage_detect_ -- --nocapture` | 退出 101，测试导入未实现的 `super::detect` 时遇到 E0432，执行 0 项。 |
| 格式化 | `cargo fmt --all` | 退出 0，在独立源码副本执行，仅同步本任务的三个 Rust 文件。 |
| 实现后 | `cargo test -p vw-agent-apps usage_detect_ -- --nocapture` | 退出 0，3 项通过，0 项失败。 |
| 工作区检查 | `cargo clippy --all-targets --all-features -- -D warnings` | 退出 0。 |

三项用例分别为 `usage_detect_returns_expected_result`、`usage_detect_rejects_invalid_input`、`usage_detect_requires_explicit_team_selection_and_disables_volcengine_by_default`。限定提交范围的 `git diff --cached --check` 退出 0。槽位锁已释放，编译缓存保留。

## 任务 11 DeepSeek 钱包赠金

| 来源 | 实现和测试 |
| --- | --- |
| [余额模板检测](../../../../crates/vw-agent-apps/upstream/src/components/UsageScriptModal.tsx#L184-L200) · [DeepSeek 余额查询](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/balance.rs#L71-L151) | `query_deepseek` 使用固定的 `GET https://api.deepseek.com/user/balance` 和已授权的 Bearer 凭据。保留所有币种，包括余额为零和欠费的钱包。`total_balance` 是当前合计余额，映射到 `UsageWallet.remaining`，赠金和充值余额分别保留。 |
| [上游钱包与可用性测试](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/balance.rs#L642-L693) | `DeepSeekBalance` 单独保留 `is_available`。HTTP 401/403 返回 `UpstreamUnauthorized`。本机 HTTP 夹具核对方法、路径、请求头，并覆盖多币种、鉴权拒绝、无效响应及重定向。 |

[适配器](../../../../crates/vw-agent-apps/src/usage/balances/deepseek.rs) · [独立行为测试](../../../../crates/vw-agent-apps/src/usage/balances/deepseek_tests.rs)

验证环境为 macOS 27.0 (26A428)、arm64，`vw-agent-apps` 0.6.0。两阶段均领取槽位 2，`CARGO_TARGET_DIR` 和 `CARGO_BUILD_BUILD_DIR` 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。

| 阶段 | 命令 | 结果 |
| --- | --- | --- |
| 实现前 | `cargo test -p vw-agent-apps usage_balances_deepseek_ -- --nocapture` | 退出 101。`usage_balances_deepseek_returns_expected_result`、`usage_balances_deepseek_rejects_invalid_input` 等用例引用尚未实现的 `client_builder`、`query_at`，触发 E0432，执行 0 项。 |
| 格式化 | `cargo fmt --all` | 退出 0，在独立源码副本执行，仅同步任务 11 的 Rust 文件。 |
| 实现后 | `cargo test -p vw-agent-apps usage_balances_deepseek_ -- --nocapture` | 退出 0，4 项通过，0 项失败。 |
| 工作区检查 | `cargo clippy --all-targets --all-features -- -D warnings` | 退出 0。 |

限定提交范围的 `git diff --cached --check` 退出 0。槽位锁已释放，编译缓存保留。真实账号验收见任务 55。

## 任务 12 StepFun 余额

| 来源 | 实现和测试 |
| --- | --- |
| [StepFun 余额查询](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/balance.rs#L153-L209) · [金额解析与余额分派](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/balance.rs#L534-L572) | `query_stepfun` 使用固定的 `GET https://api.stepfun.com/v1/accounts` 和已授权的 Bearer 凭据。将数字或字符串形式的有限 `balance` 映射为 CNY 钱包余额，保留零值和负值。缺失或非法金额返回 `InvalidResponse`。空密钥在发请求前返回 `CredentialUnavailable`。 |
| [余额模板检测](../../../../crates/vw-agent-apps/upstream/src/components/UsageScriptModal.tsx#L184-L200) · [余额模板调用](../../../../crates/vw-agent-apps/upstream/src/components/UsageScriptModal.tsx#L622-L627) | 已有 `detect_native_usage` 识别 StepFun 主机。本机 HTTP 夹具验证请求方法、路径、请求头和余额映射，以及无效密钥、无效响应、鉴权拒绝、限流、服务不可用及重定向。 |

[适配器](../../../../crates/vw-agent-apps/src/usage/balances/stepfun.rs) · [独立行为测试](../../../../crates/vw-agent-apps/src/usage/balances/stepfun_tests.rs)

验证环境为 macOS 27.0 (26A428)、arm64，`vw-agent-apps` 0.6.0。两阶段均领取槽位 1，`CARGO_TARGET_DIR` 和 `CARGO_BUILD_BUILD_DIR` 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。

| 阶段 | 命令 | 结果 |
| --- | --- | --- |
| 实现前 | `cargo test -p vw-agent-apps usage_balances_stepfun_ -- --nocapture` | 退出 101。`usage_balances_stepfun_returns_expected_result`、`usage_balances_stepfun_rejects_invalid_input` 等用例导入未实现的 `client_builder`、`query_at`、`query_stepfun` 时触发 E0432，执行 0 项。 |
| 格式化 | `cargo fmt --all` | 退出 0，在独立源码副本执行，仅同步任务 12 的三个 Rust 文件。 |
| 实现后 | `cargo test -p vw-agent-apps usage_balances_stepfun_ -- --nocapture` | 退出 0，4 项通过，0 项失败。 |
| 工作区检查 | `cargo clippy --all-targets --all-features -- -D warnings` | 退出 0。 |

限定提交范围的 `git diff --cached --check` 退出 0。槽位锁已释放，编译缓存保留。真实账号验收见任务 55。

## 任务 13 SiliconFlow 余额

| 来源 | 实现和测试 |
| --- | --- |
| [SiliconFlow 余额查询](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/balance.rs#L211-L285) · [余额分派](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/balance.rs#L550-L572) · [官方响应示例](https://siliconflow.readme.io/reference/user-info) | `query_siliconflow` 使用已识别的地区选择固定的 `GET https://api.siliconflow.cn/v1/user/info`（CNY）或 `GET https://api.siliconflow.com/v1/user/info`（USD）。只向选定端点发送经授权的 Bearer 凭据，拒绝所有重定向。 |
| [余额模板检测](../../../../crates/vw-agent-apps/upstream/src/components/UsageScriptModal.tsx#L184-L200) · [余额模板调用](../../../../crates/vw-agent-apps/upstream/src/components/UsageScriptModal.tsx#L622-L627) | `totalBalance` 是当前合计余额。按官方示例的分项关系，将 `balance` 和 `chargeBalance` 分别保留为赠金余额和充值余额。接受有限数字及数字字符串，区分缺失、零和负值。夹具检查真实 HTTP 请求以及无效凭据、上游拒绝、无效响应和跨域重定向。 |

[适配器](../../../../crates/vw-agent-apps/src/usage/balances/siliconflow.rs) · [独立行为测试](../../../../crates/vw-agent-apps/src/usage/balances/siliconflow_tests.rs)

验证环境为 macOS 27.0 (26A428)、arm64，`vw-agent-apps` 0.6.0。各阶段均领取槽位 1，`CARGO_TARGET_DIR` 和 `CARGO_BUILD_BUILD_DIR` 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。共享工作区首次测试遇到其他改动中的 `vw-acp/src/access_policy/claude.rs` E0277（`Sha256` 未实现 `std::io::Write`）。随后在批次 12 提交 `c9eff0088` 的独立源码副本上验证。

| 阶段 | 命令 | 结果 |
| --- | --- | --- |
| 实现前 | `cargo test -p vw-agent-apps usage_balances_siliconflow_ -- --nocapture` | 退出 101，执行 0 项。独立用例 `usage_balances_siliconflow_returns_expected_result` 和 `usage_balances_siliconflow_rejects_invalid_input` 等引用尚未实现的 `client_builder`、`endpoint`、`query_at`、`query_siliconflow`，触发 E0432。 |
| 格式化 | `cargo fmt --all` | 退出 0，在源码副本执行，仅将本任务三个 Rust 文件同步到共享工作区。 |
| 实现后 | `cargo test -p vw-agent-apps usage_balances_siliconflow_ -- --nocapture` | 退出 0，4 项通过，0 项失败。 |
| 本包检查 | `cargo clippy -p vw-agent-apps --all-targets --all-features -- -D warnings` | 共享工作区退出 0。 |
| 全仓检查 | `cargo clippy --all-targets --all-features -- -D warnings` | 源码副本退出 101，`vw-agent/src/session/store/postgres_tests.rs:81` 的 `load_session_meta` 缺少 `include_pending` 参数（E0061）。共享工作区重跑退出 101，`vw-desktop/src/app/message/chat/stream/permissions.rs:240` 缺少 `session_set_full_access` 方法，`vw-agent/src/gateway/api/handlers/session/ui_ops.rs:110` 等调用处缺少 `GatewaySessionUiSummary.full_access` 字段（E0599、E0609、E0560）。 |

限定提交范围的 `git diff --cached --check` 退出 0。槽位锁已释放，编译缓存保留。真实账号验收见任务 55。

## 任务 14 OpenRouter 余额

| 来源 | 实现和测试 |
| --- | --- |
| [余额查询与解析](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/balance.rs#L288-L362) · [余额分派](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/balance.rs#L554-L572) · [OpenRouter Credits API](https://openrouter.ai/docs/api/api-reference/credits/get-credits) | `query_openrouter` 使用经执行主机授权的管理密钥请求固定的 `GET https://openrouter.ai/api/v1/credits`，禁用重定向。401 和 403 归为上游鉴权拒绝。 |
| [上游行为测试](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/balance.rs#L580-L639) · [余额模板识别](../../../../crates/vw-agent-apps/upstream/src/components/UsageScriptModal.tsx#L184-L200) · [余额模板调用](../../../../crates/vw-agent-apps/upstream/src/components/UsageScriptModal.tsx#L622-L627) | 接受有 `data` 包装和无包装的响应，以及有限数字和数字字符串。保留美元总额、已用额和两者之差，包括零和负余额。缺失、无效金额及非有限差值返回 `InvalidResponse`。独立测试通过本地 HTTP 服务检查请求路径、请求头、金额边界及上游错误分类。 |

[适配器](../../../../crates/vw-agent-apps/src/usage/balances/openrouter.rs) · [独立行为测试](../../../../crates/vw-agent-apps/src/usage/balances/openrouter_tests.rs)

验证环境为 macOS 27.0 (26A428)、arm64，`vw-agent-apps` 0.6.0。各阶段均领取槽位 2，`CARGO_TARGET_DIR` 和 `CARGO_BUILD_BUILD_DIR` 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。格式化、聚焦测试和本包 Clippy 在批次 13 提交 `36270520f` 的独立源码副本执行。

| 阶段 | 命令 | 结果 |
| --- | --- | --- |
| 实现前 | `cargo test -p vw-agent-apps usage_balances_openrouter_ -- --nocapture` | 退出 101，执行 0 项。独立用例 `usage_balances_openrouter_returns_expected_result`、`usage_balances_openrouter_rejects_invalid_input` 和 `usage_balances_openrouter_classifies_upstream_rejections` 引用待实现符号，触发 E0432。 |
| 格式化 | `cargo fmt --all` | 两次退出 0，仅将本批三个 Rust 文件同步到共享工作区。 |
| 实现后 | `cargo test -p vw-agent-apps usage_balances_openrouter_ -- --nocapture` | 退出 0，3 项通过，0 项失败。 |
| 本包检查 | `cargo clippy -p vw-agent-apps --all-targets --all-features -- -D warnings` | 退出 0。 |
| 全仓检查 | `cargo clippy --all-targets --all-features -- -D warnings` | 共享工作区退出 0。 |

限定提交范围的 `git diff --cached --check` 退出 0。槽位锁已释放，编译缓存保留。真实账号验收见任务 55。

## 任务 15 Novita 余额

| 来源 | 实现和测试 |
| --- | --- |
| [余额查询与解析](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/balance.rs#L364-L433) · [余额分派](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/balance.rs#L565-L572) · [Novita 余额 API](https://docs.novita.ai/api-reference/basic-get-user-balance) | `query_novita` 使用经执行主机授权的 API 密钥请求 Novita 当前文档中的 `GET https://api.novita.ai/openapi/v1/billing/balance/detail`，禁用重定向。 |
| [上游行为测试](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/balance.rs#L691-L721) · [余额模板识别](../../../../crates/vw-agent-apps/upstream/src/components/UsageScriptModal.tsx#L184-L199) · [余额模板调用](../../../../crates/vw-agent-apps/upstream/src/components/UsageScriptModal.tsx#L622-L627) | 将 `availableBalance` 和可选的 `cashBalance` 按万分之一美元换算，分别保留为可用余额和充值剩余。保留零、负数和缺失的区别。上游缺少独立赠金字段，因此赠金为未知。授信上限表示可欠款额度。通过本地 HTTP 服务核对请求方法、路径和头部，并区分非法 JSON、截断响应和鉴权拒绝。 |

[适配器](../../../../crates/vw-agent-apps/src/usage/balances/novita.rs) · [独立行为测试](../../../../crates/vw-agent-apps/src/usage/balances/novita_tests.rs)

验证环境为 macOS 27.0 (26A428)、arm64，`vw-agent-apps` 0.6.0。各阶段的 `CARGO_BUILD_BUILD_DIR` 与 `CARGO_TARGET_DIR` 相同。实现前使用槽位 2，路径为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。实现后使用槽位 1，路径为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。

| 阶段 | 命令 | 结果 |
| --- | --- | --- |
| 实现前 | `cargo test -p vw-agent-apps usage_balances_novita_ -- --nocapture` | 退出 101，执行 0 项。独立用例 `usage_balances_novita_returns_expected_result`、`usage_balances_novita_rejects_invalid_input` 和 `usage_balances_novita_distinguishes_transport_and_upstream_status` 引用待实现符号，触发 E0432。 |
| 格式化 | `cargo fmt --all` | 在批次 14 提交 `4dc0cf131` 的隔离源码副本运行两次，均退出 0。只将任务 15 的三个 Rust 文件同步到共享工作区。 |
| 实现后 | `cargo test -p vw-agent-apps usage_balances_novita_ -- --nocapture` | 共享工作区退出 0，3 项通过，0 项失败。 |
| 全仓检查 | `cargo clippy --all-targets --all-features -- -D warnings` | 共享工作区退出 0。 |

限定提交范围的 `git diff --cached --check` 退出 0。槽位锁已释放，编译缓存保留。真实账号验收见任务 55。

## 任务 16 智谱回落余额

| 来源 | 实现和测试 |
| --- | --- |
| [个人套餐检测](../../../../crates/vw-agent-apps/upstream/src/config/codingPlanProviders.ts#L32-L50) · [套餐查询与无套餐识别](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/coding_plan.rs#L376-L495) · [用量分派](../../../../crates/vw-agent-apps/upstream/src-tauri/src/commands/provider.rs#L713-L751) | `query_zhipu_balance` 接收已授权的智谱个人套餐类型和 API 密钥，仅向对应官方站点发送请求。只有 HTTP 成功、业务码 500、明确失败且完整文案匹配已知无套餐响应时才查询余额。上游复用业务码 500，完整匹配缩小了上游模糊关键词识别的范围。鉴权、限流、传输中断、其他业务失败及格式错误使用类型化错误。已有套餐返回 `Ok(None)`，多时间窗解析归任务 18。 |
| [余额请求与解析](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/balance.rs#L434-L530) · [套餐判别夹具](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/coding_plan.rs#L3371-L3458) · [余额夹具](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/balance.rs#L724-L774) | 本地 HTTP 测试核对套餐和余额的 GET 路径、原样鉴权头、语言及 JSON 请求头，并验证套餐查询失败时只请求套餐。国内站按 CNY，国际站按上游推定记为 USD。余额优先取 `availableBalance`，缺少可解析值时使用 `balance`，保留零和负数。 |

[适配器](../../../../crates/vw-agent-apps/src/usage/balances/zhipu.rs) · [独立行为测试](../../../../crates/vw-agent-apps/src/usage/balances/zhipu_tests.rs)

验证环境为 macOS 27.0 (26A428)、arm64，`vw-agent-apps` 0.6.0。实现前后均动态领取槽位 2。`CARGO_TARGET_DIR` 和 `CARGO_BUILD_BUILD_DIR` 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。

| 阶段 | 命令 | 结果 |
| --- | --- | --- |
| 实现前 | `cargo test -p vw-agent-apps usage_balances_zhipu_ -- --nocapture` | 退出 101，执行 0 项。`usage_balances_zhipu_returns_expected_result`、`usage_balances_zhipu_rejects_invalid_input` 等测试依赖尚未实现的 `client_builder`、`query_at` 和 `query_zhipu_balance`，编译报 E0432。 |
| 格式化 | `cargo fmt --all` | 在批次 15 提交 `448833e9d` 的隔离源码副本运行两次，均退出 0。只同步任务 16 的三个 Rust 文件。 |
| 实现后 | `cargo test -p vw-agent-apps usage_balances_zhipu_ -- --nocapture` | 共享工作区退出 0，4 项通过，0 项失败。 |
| 全仓检查 | `cargo clippy --all-targets --all-features -- -D warnings` | 共享工作区退出 0。 |

限定提交范围的 `git diff --cached --check` 退出 0。槽位锁已释放，编译缓存保留。真实账号验收见任务 55 和 56。

## 任务 17 Kimi 编程套餐

| 来源 | 实现和测试 |
| --- | --- |
| [TS 套餐检测](../../../../crates/vw-agent-apps/upstream/src/config/codingPlanProviders.ts#L32-L34) · [Rust 套餐分派](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/coding_plan.rs#L1902-L1907) · [区域套餐查询](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/coding_plan.rs#L152-L266) | `query_kimi` 接收执行主机已授权的凭据、区域、查询时间和关联 ID。只请求对应官方站点的 `GET /coding/v1/usages`，使用 Bearer 鉴权、JSON Accept、15 秒超时和禁用重定向。HTTP 401/403 标记凭据过期，空窗口列表表示查询成功但无窗口。状态码、传输及响应格式失败分别归类，错误文本不包含凭据或响应。 |
| [上游时间及数字解析](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/coding_plan.rs#L95-L119) · [每周额度夹具](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/kimi_oauth/mod.rs#L1212-L1214) | 保留 `limits[].detail` 的 5 小时窗口顺序，再附上 `usage` 的每周窗口。接受 JSON 数字和数字字符串，已用量取总量减剩余量并以零为下限。缺失值保持为空，零上限保留零利用率。重置时间接受 RFC 3339、正整数 Unix 秒或毫秒，非正时间戳表示无重置时间。无效的已知字段返回格式错误。 |

[适配器](../../../../crates/vw-agent-apps/src/usage/coding/kimi.rs) · [独立行为测试](../../../../crates/vw-agent-apps/src/usage/coding/kimi_tests.rs)

验证环境为 macOS 27.0 (26A428)、arm64，`vw-agent-apps` 0.6.0。实现前后动态领取槽位 1。`CARGO_TARGET_DIR` 和 `CARGO_BUILD_BUILD_DIR` 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。

| 阶段 | 命令 | 结果 |
| --- | --- | --- |
| 实现前 | `cargo test -p vw-agent-apps usage_coding_kimi_ -- --nocapture` | 退出 101，执行 0 项。`usage_coding_kimi_returns_expected_result` 和 `usage_coding_kimi_rejects_invalid_input` 所在测试模块引用尚未实现的 `api_root`、`client_builder`、`query_at` 和 `query_kimi`，编译报 E0432。 |
| 格式化 | `cargo fmt --all` | 在 `07cbda65b` 和 `534765738` 的隔离源码副本各运行一次，均退出 0。只同步本任务的 Rust 文件。 |
| 实现后 | `cargo test -p vw-agent-apps usage_coding_kimi_ -- --nocapture` | 退出 0，5 项通过，0 项失败。本地 HTTP 夹具核对实际请求并重放额度、空套餐、凭据过期、格式错误和传输失败响应。 |
| 全仓检查 | `cargo clippy --all-targets --all-features -- -D warnings` | 退出 0。 |

限定提交范围的 `git diff --cached --check` 退出 0。槽位锁已释放，编译缓存保留。真实账号验收见任务 56。

## 任务 18 智谱个人套餐

| 来源 | 实现和测试 |
| --- | --- |
| [TS 个人版检测](../../../../crates/vw-agent-apps/upstream/src/config/codingPlanProviders.ts#L35-L50) · [Rust 套餐查询](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/coding_plan.rs#L368-L436) | `query_zhipu` 只接受国内或国际个人版供应商类型，向对应官方站点发送 GET 请求，使用原样鉴权头、15 秒超时和禁用重定向。HTTP、传输和响应错误返回稳定错误码及脱敏说明。 |
| [窗口分类与档位](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/coding_plan.rs#L274-L367) · [档位字段](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/coding_plan.rs#L515-L533) · [窗口夹具](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/coding_plan.rs#L2444-L2661) | 按 `unit` 优先识别 5 小时和每周窗口。旧数据先放无重置时间的窗口，再按重置时间升序填空位。识别 `TOKENS_LIMIT` 和 `CREDIT_LIMIT`，保留套餐档位、有限百分比和毫秒级重置时间。缺失的用量保留为空，畸形用量字段返回 `InvalidResponse`。 |
| [余额回落调用方](../../../../crates/vw-agent-apps/upstream/src-tauri/src/commands/provider.rs#L713-L751) · [无套餐夹具](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/coding_plan.rs#L3371-L3458) | 套餐查询和任务 16 的余额回落共用原有 HTTP 和无套餐判定。明确无套餐返回 `None`，有效套餐可返回空窗口，鉴权和业务失败保留为类型化错误。 |

[适配器](../../../../crates/vw-agent-apps/src/usage/coding/zhipu.rs) · [独立行为测试](../../../../crates/vw-agent-apps/src/usage/coding/zhipu_tests.rs)

验证环境为 macOS 27.0 (26A428)、arm64，`vw-agent-apps` 0.6.0。动态领取槽位 2，`CARGO_TARGET_DIR` 和 `CARGO_BUILD_BUILD_DIR` 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。

| 阶段 | 命令 | 结果 |
| --- | --- | --- |
| 实现前 | `cargo test -p vw-agent-apps usage_coding_zhipu_ -- --nocapture` | 退出 101，执行 0 项。`usage_coding_zhipu_returns_expected_result` 和 `usage_coding_zhipu_rejects_invalid_input` 所在测试模块因适配器尚未实现报 E0432。 |
| 格式化 | `cargo fmt --all` | 在隔离源码副本执行，退出 0。仅同步本任务 Rust 文件。 |
| 实现后 | `cargo test -p vw-agent-apps usage_coding_zhipu_ -- --nocapture` | 退出 0，4 项通过，0 项失败。本地 HTTP 夹具核对请求方法、路径和请求头，并验证窗口顺序、重置时间、无套餐与失败的区别。 |
| 余额回归 | `cargo test -p vw-agent-apps usage_balances_zhipu_ -- --nocapture` | 退出 0，4 项通过，0 项失败。 |
| 全仓检查 | `cargo clippy --all-targets --all-features -- -D warnings` | 退出 0。 |

限定提交范围的 `git diff --cached --check` 退出 0。槽位锁已释放，编译缓存保留。真实账号验收见任务 56。

## 任务 19 智谱团队套餐

| 来源 | 实现和测试 |
| --- | --- |
| [TS 显式团队配置](../../../../crates/vw-agent-apps/upstream/src/config/codingPlanProviders.ts#L35-L50) · [Rust 分派](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/coding_plan.rs#L1858-L1877) | `query_zhipu_team` 仅接受显式团队检测产生的 `CodingPlanProvider::ZhipuTeam`。`ZhipuTeamCredentials` 缺组织或项目、字段为空或请求头非法时，在发请求前返回 `MissingConfiguration`。密钥缺失或非法返回 `CredentialUnavailable`。 |
| [团队 HTTP 请求](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/coding_plan.rs#L1747-L1814) · [请求与凭据夹具](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/coding_plan.rs#L3270-L3369) | 对国内官方额度端点发送 GET，携带 `type=2`、原样鉴权头和团队组织、项目请求头。复用智谱 HTTP 错误分类、响应校验和窗口解析。无套餐响应返回 `UpstreamRejected`，结果保持团队来源；鉴权拒绝标记凭据过期。 |

[团队适配器](../../../../crates/vw-agent-apps/src/usage/coding/zhipu_team.rs) · [独立行为测试](../../../../crates/vw-agent-apps/src/usage/coding/zhipu_team_tests.rs)

验证环境为 macOS 27.0 (26A428)、arm64，`vw-agent-apps` 0.6.0。实现前动态领取槽位 2，`CARGO_TARGET_DIR` 为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。实现后动态领取槽位 1，`CARGO_TARGET_DIR` 为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。各阶段的 `CARGO_BUILD_BUILD_DIR` 与对应目标目录一致。

| 阶段 | 命令 | 结果 |
| --- | --- | --- |
| 实现前 | `cargo test -p vw-agent-apps usage_coding_zhipu_team_ -- --nocapture` | 退出 101，执行 0 项。`usage_coding_zhipu_team_returns_expected_result` 和 `usage_coding_zhipu_team_rejects_invalid_input` 所在模块因团队适配器尚未实现报 E0432。 |
| 格式化 | `cargo fmt --all` | 在隔离源码副本执行，退出 0。同步本任务的 Rust 文件。 |
| 实现后 | `cargo test -p vw-agent-apps usage_coding_zhipu_team_ -- --nocapture` | 退出 0，3 项通过，0 项失败。成功夹具核对方法、路径、请求头和双窗口；失败夹具核对发送前拒绝、无套餐和鉴权失败。 |
| 套餐回归 | `cargo test -p vw-agent-apps usage_coding_zhipu_ -- --nocapture` | 退出 0，7 项通过，0 项失败，含 3 项团队用例。 |
| 余额回归 | `cargo test -p vw-agent-apps usage_balances_zhipu_ -- --nocapture` | 退出 0，4 项通过，0 项失败。 |
| 检测回归 | `cargo test -p vw-agent-apps usage_detect_ -- --nocapture` | 退出 0，3 项通过，0 项失败。 |
| 全仓检查 | `cargo clippy --all-targets --all-features -- -D warnings` | 退出 0。 |

限定提交范围的 `git diff --cached --check` 退出 0。槽位锁已释放，编译缓存保留。HTTP 证据来自本地夹具，真实账号验收见任务 56。

## 任务 20 MiniMax 套餐

| 来源 | 实现和测试 |
| --- | --- |
| [TS 套餐检测](../../../../crates/vw-agent-apps/upstream/src/config/codingPlanProviders.ts#L51-L57) · [Rust 套餐分派](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/coding_plan.rs#L1907-L1908) · [区域请求](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/coding_plan.rs#L539-L612) | `query_minimax` 复用区域检测，将两个国内推理域名映射到 `api.minimaxi.com`，国际入口映射到 `api.minimax.io`。本机 HTTP 夹具核对 GET 路径、Bearer 鉴权和 JSON Content-Type。请求限时 15 秒并禁止重定向。HTTP 401/403 标记凭据过期，业务拒绝与响应格式错误分别返回稳定错误码。 |
| [窗口解析](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/coding_plan.rs#L770-L839) · [上游夹具](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/coding_plan.rs#L2661-L2832) · [上游计数语义与字段切换](https://github.com/farion1231/cc-switch/commit/43ae1e5f2c808a28d55815a5da877afd9acaa6bb) | 读取 `general`，使用 100 减剩余百分比得到已用比例，保留超界的有限百分比与毫秒重置时间。周状态 1 和 2 表示有效窗口，状态 3 不展示周窗口。上游同时提供正总量和剩余计数时，按总量减 `usage_count` 得到已用请求数，零占位和缺失计数不填为额度。缺失百分比的窗口跳过，整份响应没有可用窗口时返回 `InvalidResponse`。 |

[MiniMax 适配器](../../../../crates/vw-agent-apps/src/usage/coding/minimax.rs#L26) · [独立行为测试](../../../../crates/vw-agent-apps/src/usage/coding/minimax_tests.rs#L83)

验证环境为 macOS 27.0 (26A428)、arm64，`vw-agent-apps` 0.6.0。实现前后分别动态领取槽位 2，`CARGO_TARGET_DIR` 和 `CARGO_BUILD_BUILD_DIR` 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。

| 阶段 | 命令 | 结果 |
| --- | --- | --- |
| 实现前 | `cargo test -p vw-agent-apps usage_coding_minimax_ -- --nocapture` | 退出 101，执行 0 项。测试模块报 E0432 和 E0425，缺少 `query_minimax`、`minimax_origin`、`query_at` 和 HTTP 客户端构造入口。 |
| 格式化 | `cargo fmt --all` | 在隔离源码副本执行，退出 0。仅同步任务范围内的 Rust 文件。 |
| 实现后 | `cargo test -p vw-agent-apps usage_coding_minimax_ -- --nocapture` | 退出 0，3 项通过，0 项失败。覆盖双窗口、缺失字段、无周限额和错误分类。 |
| 全仓检查 | `cargo clippy --all-targets --all-features -- -D warnings` | 退出 0。 |

首次试跑设置了目标目录，随后发现仓库另设中间产物目录。表内采用两个目录均指向槽位的复跑结果。限定提交范围的 `git diff --cached --check` 退出 0。槽位锁已释放，编译缓存保留。HTTP 证据来自本机夹具，真实账号验收见任务 56。

## 任务 21 ZenMux 套餐

| 来源 | 实现和测试 |
| --- | --- |
| [TS 套餐识别](../../../../crates/vw-agent-apps/upstream/src/config/codingPlanProviders.ts#L58-L62) · [凭据选择](../../../../crates/vw-agent-apps/upstream/src-tauri/src/commands/provider.rs#L622-L654) · [Rust 请求](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/coding_plan.rs#L630-L679) | `query_zenmux` 由执行端授权并提供完整查询地址和已解析的凭据。依照已有检测范围，只向 `zenmux.ai` 默认 HTTPS 端口发送请求，拒绝嵌入的凭据和片段，保留路径和查询参数。GET 使用 Bearer 鉴权、JSON Accept 和 15 秒超时，禁用重定向。 |
| [窗口解析](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/coding_plan.rs#L681-L767) · [用量投影](../../../../crates/vw-agent-apps/upstream/src-tauri/src/commands/provider.rs#L758-L796) | 依序保留五小时和七天窗口。数字及数字字符串均可解析，使用率由比例乘以 100 得到，已用额与总额保留 USD，两者齐备时计算剩余金额。保留原样 RFC 3339 重置时间、套餐档位和账号状态。缺失字段保留为空，没有可识别的窗口、窗口字段畸形或数值非有限时返回 `InvalidResponse`。 |
| [瞬时及确定性失败夹具](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/coding_plan.rs#L3052-L3190) | 本机 HTTP 夹具核对方法、地址和请求头，并验证鉴权拒绝、限流、服务不可用、重定向、传输中断、业务拒绝和未知结构分别归类。对外错误携带稳定码及关联 ID。 |

[ZenMux 适配器](../../../../crates/vw-agent-apps/src/usage/coding/zenmux.rs#L24) · [独立行为测试](../../../../crates/vw-agent-apps/src/usage/coding/zenmux_tests.rs)

验证环境为 macOS 27.0 (26A428)、arm64，`vw-agent-apps` 0.6.0。在批次 20 提交 `4199e2867` 的隔离源码副本中叠加本任务文件执行验证。实现前动态领取槽位 1，`CARGO_TARGET_DIR` 为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。实现后领取槽位 2，目录为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。各阶段的 `CARGO_BUILD_BUILD_DIR` 与目标目录一致。

| 阶段 | 命令 | 结果 |
| --- | --- | --- |
| 实现前 | `cargo test -p vw-agent-apps usage_coding_zenmux_ -- --nocapture` | 退出 101，执行 0 项。`usage_coding_zenmux_returns_expected_result`、`usage_coding_zenmux_rejects_invalid_input` 和 `usage_coding_zenmux_classifies_http_and_transport_failures` 所在模块报 E0432，缺少 `query_zenmux` 和 `query_at`。 |
| 格式化 | `cargo fmt --all` | 退出 0，只同步本任务的三个 Rust 文件。 |
| 实现后 | `cargo test -p vw-agent-apps usage_coding_zenmux_ -- --nocapture` | 退出 0，3 项通过，0 项失败。 |
| 全仓检查 | `cargo clippy --all-targets --all-features -- -D warnings` | 首轮退出 101。基线的 `vw-agent/src/session/store/postgres_tests.rs:81` 缺少 `load_session_meta` 的第三个参数，报 E0061。将共享工作区已有的一行修正同步到隔离副本后，在槽位 2 复跑退出 0。 |
| 本包检查 | `cargo clippy -p vw-agent-apps --all-targets --all-features -- -D warnings` | 槽位 2 退出 0。 |

仅审查并暂存本任务的适配器、模块注册、独立测试和覆盖记录，`git diff --cached --check` 退出 0。各阶段均释放所持槽位锁，保留编译缓存。HTTP 证据来自本机夹具，真实账号验收见任务 56。

## 任务 22 火山方舟请求签名

| 来源 | 实现和测试 |
| --- | --- |
| [TS 启用规则](../../../../crates/vw-agent-apps/upstream/src/config/codingPlanProviders.ts#L103-L113) · [AK/SK 分派](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/coding_plan.rs#L1883-L1894) | 签名入口接收执行端解析出的账号 AK/SK。凭据缺失或无法安全写入鉴权头时返回 `CredentialUnavailable`，非法动作或区域配置返回 `MissingConfiguration`。错误不携带输入，鉴权头标记为敏感。 |
| [签名与请求构造](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/coding_plan.rs#L1342-L1473) · [固定时间夹具](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/coding_plan.rs#L2983-L3028) | `sign_volcengine_request` 构造固定控制面地址的 POST，查询按 Action、Region、Version 顺序排列，签名和请求共用查询与请求体字节。固定时间夹具核对完整 Authorization、请求日期和 SHA-256，预期签名按上游规则用 Python 标准库独立计算。额外夹具覆盖保留字符、UTF-8、非空请求体与脱敏错误。 |

[签名入口](../../../../crates/vw-agent-apps/src/usage/coding/volcengine_sign.rs#L21) · [独立行为测试](../../../../crates/vw-agent-apps/src/usage/coding/volcengine_sign_tests.rs#L12)

验证环境为 macOS 27.0 (26A428)、arm64，`vw-agent-apps` 0.6.0。使用批次 21 提交 `b1eb7d5a0` 的隔离源码副本。两阶段分别动态领取槽位 2，`CARGO_TARGET_DIR` 和 `CARGO_BUILD_BUILD_DIR` 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。

| 阶段 | 命令 | 结果 |
| --- | --- | --- |
| 格式化 | `cargo fmt --all` | 两阶段均退出 0，仅同步本任务的 Rust 文件。 |
| 实现前 | `cargo test -p vw-agent-apps usage_coding_volcengine_sign_ -- --nocapture` | 退出 101，执行 0 项。三个用例所在模块报 E0432，无法导入尚未实现的 `super::volcengine_sign`。 |
| 实现后 | `cargo test -p vw-agent-apps usage_coding_volcengine_sign_ -- --nocapture` | 退出 0，3 项通过，0 项失败。用例为 `usage_coding_volcengine_sign_returns_expected_result`、`usage_coding_volcengine_sign_binds_encoded_query_and_payload` 和 `usage_coding_volcengine_sign_rejects_invalid_input`。 |
| 全仓检查 | `cargo clippy --all-targets --all-features -- -D warnings` | 退出 0。隔离副本同步了共享工作区已有的 `postgres_tests.rs` 第三参数修正，以通过基线测试编译。 |

限定提交范围的 `git diff --cached --check` 退出 0。两阶段均释放槽位锁并保留缓存。夹具验证签名与请求构造，真实账号验收见任务 56。

## 任务 23 火山双套餐查询

| 来源 | 实现和测试 |
| --- | --- |
| [上游 AK/SK 分派](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/coding_plan.rs#L1883-L1894) · [上游探测顺序](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/coding_plan.rs#L1654-L1726) · [TS 启用规则](../../../../crates/vw-agent-apps/upstream/src/config/codingPlanProviders.ts#L103-L113) | `query_volcengine` 复用批次 22 的签名入口，在固定控制面先发 `GetAFPUsage`，遇到空额度或非鉴权响应失败时再发 `GetCodingPlanUsage`。仅从方舟地区域名提取区域，其他输入使用 `cn-beijing`。探测成功即返回，两个响应均无窗口时返回脱敏的无有效套餐错误。 |
| [上游错误信封与 HTTP 处理](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/coding_plan.rs#L1301-L1336) · [上游错误与回退](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/coding_plan.rs#L1474-L1529) · [火山公共错误码](https://docs.volcengine.com/docs/Contentmanagementplatform/Commonerrorcode-4?lang=zh) | HTTP 401/403 和 OpenAPI 信封内的已知鉴权码立即停止探测，返回 `UpstreamUnauthorized`。网络、超时和读体中断也立即返回。普通业务错误进入下一探测，限流与服务不可用在没有可用额度时保留重试分类。公开错误使用固定说明。 |
| [上游窗口解析](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/coding_plan.rs#L1531-L1622) · [上游窗口夹具](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/coding_plan.rs#L2874-L2963) · [火山 AFP 接口字段](https://docs.volcengine.com/docs/ark/get-afp-usage-api?lang=zh) | Agent Plan 保留 5 小时、周、月窗口的 AFP 已用量、总额、剩余及百分比，跳过日窗口和非正额度。已知档位映射为固定标签。Coding Plan 保留窗口百分比，未知窗口跳过。秒和毫秒统一转为 ISO 8601，零或负时间戳表示无重置时间。 |

[查询入口](../../../../crates/vw-agent-apps/src/usage/coding/volcengine.rs#L24) · [独立 HTTP 夹具](../../../../crates/vw-agent-apps/src/usage/coding/volcengine_tests.rs#L27)

验证环境为 macOS 27.0 (26A428)、arm64，`vw-agent-apps` 0.6.0。使用批次 22 提交 `8e9f72a8f` 的隔离源码副本，绿阶段同步工作区已有的 `postgres_tests.rs` 第三参数修正。动态领取槽位 1，`CARGO_TARGET_DIR` 与 `CARGO_BUILD_BUILD_DIR` 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。

| 阶段 | 命令 | 结果 |
| --- | --- | --- |
| 格式化 | `cargo fmt --all` | 退出 0，仅同步任务 23 的三个 Rust 文件。 |
| 实现前 | `cargo test -p vw-agent-apps usage_coding_volcengine_ -- --nocapture` | 退出 101，执行 0 项。测试模块报 E0432，无法导入尚未实现的 `super::volcengine`。 |
| 实现后 | `cargo test -p vw-agent-apps usage_coding_volcengine_ -- --nocapture` | 退出 0，7 项通过，0 项失败，含本任务 4 项及批次 22 签名回归 3 项。 |
| 全仓检查 | `cargo clippy --all-targets --all-features -- -D warnings` | 退出 0。 |

本任务用例为 `usage_coding_volcengine_returns_expected_result`、`usage_coding_volcengine_rejects_invalid_input`、`usage_coding_volcengine_falls_back_and_preserves_window_times` 和 `usage_coding_volcengine_distinguishes_empty_plans_from_transport_failure`。夹具核对实际 HTTP POST 路径、签名头、请求顺序、脱敏错误和时间解析。真实账号验收留待任务 56。各阶段释放槽位锁并保留缓存，限定提交范围的差异检查退出 0。

## 任务 24 OpenCode Go 套餐查询

`query_opencode_go` 复用已交付的能力识别规则，接受 `/inference/go` 和 `/zen/go` 两类推理入口。两类入口均以 Bearer 凭据访问固定的 `GET https://opencode.ai/zen/go/v1/usage`，按量 Zen 直接返回 `Unsupported`。HTTP 401 标记凭据失效，403 返回缺少套餐的业务错误。读取 JSON 复用已有的响应大小限制和 HTTP 错误分类。 [上游 TS 规则](../../../../crates/vw-agent-apps/upstream/src/config/codingPlanProviders.ts#L71-L82) · [上游查询](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/coding_plan.rs#L889-L962) · [适配器](../../../../crates/vw-agent-apps/src/usage/coding/opencode_go.rs#L28) · [共用响应读取](../../../../crates/vw-agent-apps/src/usage/coding/zhipu.rs#L270)

依次保留 5 小时、周、月三个美元额度窗口和上游报告的使用百分比。接口只返回百分比，金额字段留空。有效的 ISO 8601 重置时间原样保留，零使用量时忽略占位时间。单个畸形窗口不影响其他有效窗口，没有可识别窗口时返回 `InvalidResponse`。 [上游解析](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/coding_plan.rs#L842-L887) · [上游窗口夹具](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/coding_plan.rs#L2378-L2441) · [本机 HTTP 夹具](../../../../crates/vw-agent-apps/src/usage/coding/opencode_go_tests.rs#L73)

验证环境为 macOS 27.0 (26A428)、arm64，`vw-agent-apps` 0.6.0。基于批次 23 提交 `95691bebd` 在隔离源码副本验证，全仓检查沿用工作区已有的 `postgres_tests.rs` 第三参数修正。两阶段均动态领取槽位 1，`CARGO_TARGET_DIR` 与 `CARGO_BUILD_BUILD_DIR` 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。

| 阶段 | 命令 | 结果 |
| --- | --- | --- |
| 实现前 | `cargo test -p vw-agent-apps usage_coding_opencode_go_ -- --nocapture` | 退出 101，执行 0 项。测试模块报 E0432，缺少 `super::opencode_go`。 |
| 格式化 | `cargo fmt --all` | 退出 0，仅同步本任务的四个 Rust 文件。 |
| 实现后 | `cargo test -p vw-agent-apps usage_coding_opencode_go_ -- --nocapture` | 退出 0，4 项通过，0 项失败。 |
| 全仓检查 | `cargo clippy --all-targets --all-features -- -D warnings` | 退出 0。 |

四项测试覆盖两类入口的实际 HTTP 方法、路径与请求头，按量 Zen 和无效凭据的拒绝，401 与 403 的区分，以及缺失窗口和占位时间。真实账号验收留待任务 56。各阶段释放槽位锁并保留缓存，提交范围差异检查退出 0。

## 任务 25 CommandCode 套餐查询

`query_command_code` 复用能力识别，Claude 的 `/provider` 和 Codex 的 `/provider/v1` 共用固定的 `https://api.commandcode.ai` 控制面。使用 Bearer 凭据依次读取身份、余额、订阅与用量摘要。按身份响应中的组织限定后三条请求，并在已知订阅周期起点时为摘要传入 `since`。共用 HTTP 客户端禁用重定向，并限制响应体大小。 [上游识别规则](../../../../crates/vw-agent-apps/upstream/src/config/codingPlanProviders.ts#L83-L89) · [上游查询顺序](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/coding_plan.rs#L1157-L1247) · [本地适配器](../../../../crates/vw-agent-apps/src/usage/coding/command_code.rs#L39)

`CommandCodeUsage` 分别返回余额、套餐结果及补充错误。余额合计上游已报告的月度、充值与赠送额度，保留负值。五小时和周窗口读取独立上限，月档沿用上游公式，以总花费除以总花费与三类非负剩余额度之和。按量账户仍可得到余额及月档。订阅查询失败时省略月档，避免查询未经确认的周期。摘要或单个窗口失败时保留其他有效窗口。任一请求收到 401 时，两类结果均标记凭据失效。错误只包含稳定码、安全说明和关联 ID。 [上游解析](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/coding_plan.rs#L1045-L1154) · [上游夹具](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/coding_plan.rs#L1966-L2153) · [本机 HTTP 夹具](../../../../crates/vw-agent-apps/src/usage/coding/command_code_tests.rs)

验证环境为 macOS 27.0 (26A428)、arm64，`vw-agent-apps` 0.6.0。实现前后测试、格式化和共享工作区 Clippy 动态领取槽位 1，`CARGO_TARGET_DIR` 与 `CARGO_BUILD_BUILD_DIR` 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。隔离 Clippy 动态领取槽位 2，两项环境变量均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。格式化在隔离源码副本执行，仅同步本任务三个 Rust 文件。

| 阶段 | 命令 | 结果 |
| --- | --- | --- |
| 实现前 | `cargo test -p vw-agent-apps usage_coding_command_code_ -- --nocapture` | 退出 101，执行 0 项。包含 `usage_coding_command_code_returns_expected_result` 和 `usage_coding_command_code_rejects_invalid_input` 的测试模块报 E0432，缺少 `super::command_code`。 |
| 格式化 | `cargo fmt --all` | 退出 0。 |
| 实现后 | `cargo test -p vw-agent-apps usage_coding_command_code_ -- --nocapture` | 退出 0，7 项通过，0 项失败。 |
| 共享工作区 | `cargo clippy --all-targets --all-features -- -D warnings` | 退出 101。并行变更中的 ACP `PermissionConfirmation` 导入报 E0432，桌面调用 `session_set_full_access` 报 E0599。 |
| 隔离复核 | `cargo clippy --all-targets --all-features -- -D warnings` | 退出 0。基于批次 24 提交 `f853e61b1`、本次三个 Rust 文件及批次 24 已记录的 PostgreSQL 测试第三参数修正。 |

七项测试核对两类入口、Bearer GET 请求顺序、组织和周期参数、按量余额、部分错误和迟发 401。真实账号验收留待任务 56。提交范围差异检查通过，各阶段均释放自己的锁并保留缓存。

## 任务 26 Claude 官方额度

`query_claude` 从已授权的账号引用读取登录快照。绑定账号默认读取受管目录，仅当执行主机、租户、主体和账号都匹配活跃槽位记录时读取默认槽位，并核对外部身份摘要。未绑定的 CLI 默认查询按授权 HOME 读取；已有目录覆盖时返回不支持。读取使用现有账号租约与登录锁。令牌和套餐取自同一份凭据，钥匙串条目仅含 MCP 凭据时读取该槽位的文件。 [上游绑定查询](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/subscription.rs#L1621) · [上游默认套餐](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/subscription.rs#L203) · [上游按账号查询键](../../../../crates/vw-agent-apps/upstream/src/lib/query/subscription.ts#L283) · [上游卡片](../../../../crates/vw-agent-apps/upstream/src/components/ClaudeAccountQuotaFooter.tsx#L29) · [适配器](../../../../crates/vw-agent-apps/src/usage/subscriptions/claude.rs#L57)

查询使用固定的 Anthropic OAuth 端点、Bearer 与 OAuth beta 头，共用禁用重定向及限制响应大小的客户端。保留旧额度窗口、模型专属周限额、零值、重置时间、超额用量及凭据里的套餐。已知过期的令牌在请求前返回失效或待刷新状态，401 和 403 返回上游鉴权拒绝。授权服务负责刷新，查询器只返回凭据状态。 [上游 HTTP 查询](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/subscription.rs#L445) · [上游额度解析](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/subscription.rs#L502) · [本地解析](../../../../crates/vw-agent-apps/src/usage/subscriptions/claude.rs#L299)

环境为 macOS 27.0 (26A428)、arm64，`vw-agent-apps` 0.6.0。实现前后测试、格式化与共享工作区 Clippy 动态领取槽位 1，`CARGO_TARGET_DIR` 与 `CARGO_BUILD_BUILD_DIR` 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。隔离 Clippy 动态领取槽位 2，两项变量均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。全仓格式化在源码副本执行，只同步本任务的 Rust 文件。

| 阶段 | 命令 | 结果 |
| --- | --- | --- |
| 实现前 | `cargo test -p vw-agent-apps usage_subscriptions_claude_ -- --nocapture` | 退出 101，执行 0 项。`usage_subscriptions_claude_returns_expected_result` 和 `usage_subscriptions_claude_rejects_invalid_input` 所在模块报 E0432，缺少 `super::claude`。 |
| 格式化 | `cargo fmt --all` | 退出 0。 |
| 实现后 | `cargo test -p vw-agent-apps usage_subscriptions_claude_ -- --nocapture` | 退出 0，5 项通过，0 项失败。 |
| 共享工作区 | `cargo clippy --all-targets --all-features -- -D warnings` | 退出 101，其他工作区变更缺少 `vw_acp::PermissionConfirmation`、`GatewayClient::session_set_full_access` 及 `GatewaySessionUiSummary.full_access`（E0432、E0599、E0609）。 |
| 隔离复核 | `cargo clippy --all-targets --all-features -- -D warnings` | 退出 0。基于批次 25 提交 `ab57e81bf`、本任务 Rust 文件及工作区已有的 `postgres_tests.rs` 第三参数修正。 |

本机 HTTP 夹具核对实际请求方法、路径与请求头，覆盖绑定账号和 CLI 默认账号、活跃账号令牌轮换、身份不匹配、钥匙串与文件选取、过期时阻止出站请求及 401/403。真实账号验收留待任务 56。任务差异检查通过，各阶段均释放槽位锁并保留缓存。 [测试](../../../../crates/vw-agent-apps/src/usage/subscriptions/claude_tests.rs#L258)

## 任务 27 Codex 官方额度

`query_codex` 消费已授权的账号引用。CLI 分支读取授权 HOME 的默认 `.codex` 登录槽，按配置选择文件或 macOS 钥匙串。托管分支按租户、主体、主机及绑定账号读取现有凭据存储。访问令牌和 ChatGPT 工作区 ID 取自同一份凭据快照。凭据续期由授权流程处理。 [上游 CLI 查询](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/subscription.rs#L1745) · [上游托管查询](../../../../crates/vw-agent-apps/upstream/src-tauri/src/commands/codex_oauth.rs#L28) · [上游账号选择](../../../../crates/vw-agent-apps/upstream/src/utils/providerCapabilities.ts#L46) · [适配器](../../../../crates/vw-agent-apps/src/usage/subscriptions/codex.rs#L38)

两种来源共用固定 ChatGPT 端点和解析器。主查询保留 5 小时、周及其他窗口，省略零用量的占位重置时间，并保留正数 Credits。附带查询按状态和到期时间筛选可用重置次数，最早到期的排在前面。附带查询失败时仍保留主额度；主请求的 401 和 403 标为凭据过期。查询禁用重定向，限制响应体大小，错误只返回类型与脱敏说明。 [上游协议](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/subscription.rs#L972) · [本地解析](../../../../crates/vw-agent-apps/src/usage/subscriptions/codex.rs#L272)

验证环境为 macOS 27.0 (26A428)、arm64，`vw-agent-apps` 0.6.0。实现前后测试、格式化和共享工作区 Clippy 动态领取槽位 1，`CARGO_TARGET_DIR` 与 `CARGO_BUILD_BUILD_DIR` 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。全仓格式化在批次 26 提交 `aa1dff319` 的源码副本执行，只同步本任务 Rust 文件。隔离 Clippy 动态领取槽位 2，两项变量均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。

| 阶段 | 命令 | 结果 |
| --- | --- | --- |
| 实现前 | `cargo test -p vw-agent-apps usage_subscriptions_codex_ -- --nocapture` | 退出 101，执行 0 项。包含 `usage_subscriptions_codex_returns_expected_result` 和 `usage_subscriptions_codex_rejects_invalid_input` 的模块报 E0432，缺少 `super::codex`。 |
| 格式化 | `cargo fmt --all` | 退出 0。 |
| 实现后 | `cargo test -p vw-agent-apps usage_subscriptions_codex_ -- --nocapture` | 退出 0，5 项通过，0 项失败。 |
| 共享工作区 | `cargo clippy --all-targets --all-features -- -D warnings` | 退出 101。并行改动中的 `vw-agent/src/session/llm/acp/approval.rs` 缺少 `PermissionConfirmation`，`vw-desktop/src/app/message/chat/stream/permissions.rs` 缺少 `session_set_full_access`，多处缺少 `GatewaySessionUiSummary.full_access`。 |
| 隔离复核 | `cargo clippy --all-targets --all-features -- -D warnings` | 退出 0。基于批次 26 提交 `aa1dff319`、本任务三个 Rust 文件和工作区已有的 `postgres_tests.rs` 第三参数修正。 |

本机 HTTP 夹具核对两种凭据来源的请求方法、路径、Bearer、工作区和 beta 头，覆盖重置过期边界、缺少绑定凭据、输入拒绝及主请求与附带请求的错误隔离。差异检查通过，槽位锁已释放，缓存已保留。真实账号与系统钥匙串留待任务 57 验收。 [测试](../../../../crates/vw-agent-apps/src/usage/subscriptions/codex_tests.rs#L187)

## 任务 28 Gemini 官方额度

`query_gemini` 由执行端在确认独占本机 CLI 登录槽后调用。适配器只读授权 HOME 下的默认凭据槽。当前进程 HOME 匹配时优先读取 macOS 钥匙串，条目缺失时读取 `.gemini/oauth_creds.json`。兼容嵌套及扁平格式。访问令牌到期时，从进程环境变量 `SHIJU_GEMINI_OAUTH_CLIENT_ID` 和 `SHIJU_GEMINI_OAUTH_CLIENT_SECRET` 读取与该登录匹配的 Gemini CLI OAuth 客户端配置，只为本次查询续期。配置缺失时返回待刷新，有效访问令牌可直接用于查询。 [上游凭据读取](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/subscription.rs#L1138) · [上游续期](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/subscription.rs#L1321) · [Gemini CLI OAuth 客户端](https://github.com/google-gemini/gemini-cli/blob/main/packages/core/src/code_assist/oauth2.ts) · [本地入口](../../../../crates/vw-agent-apps/src/usage/subscriptions/gemini.rs)

查询依次向固定 Code Assist 端点发送 `loadCodeAssist` 和 `retrieveUserQuota`，沿用 Pro、Flash、Flash Lite 分档及卡片顺序。每档返回已报告的最高用量及该模型的重置时间，缺失字段保留为空。缺凭据、凭据解析失败、上游响应无法解析和显式用尽均有独立结果。 [上游官方卡片条件](../../../../crates/vw-agent-apps/upstream/src/components/providers/ProviderCard.tsx#L148) · [上游协议](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/subscription.rs#L1406) · [上游分档标签](../../../../crates/vw-agent-apps/upstream/src/components/SubscriptionQuotaFooter.tsx#L56) · [本地分档](../../../../crates/vw-agent-apps/src/usage/subscriptions/gemini.rs#L285)

验证环境为 macOS 27.0 (26A428)、arm64，`vw-agent-apps` 0.6.0。实现前后聚焦测试、格式化及共享工作区 Clippy 动态领取槽位 1，`CARGO_TARGET_DIR` 和 `CARGO_BUILD_BUILD_DIR` 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。隔离 Clippy 动态领取槽位 2，两项变量均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。全仓格式化在源码副本执行，仅同步本任务 Rust 文件。

| 阶段 | 命令 | 结果 |
| --- | --- | --- |
| 实现前 | `cargo test -p vw-agent-apps usage_subscriptions_gemini_ -- --nocapture` | 退出 101，执行 0 项。包含 `usage_subscriptions_gemini_returns_expected_result` 和 `usage_subscriptions_gemini_rejects_invalid_input` 的模块报 E0432，缺少 `super::gemini`。 |
| 格式化 | `cargo fmt --all` | 退出 0。 |
| 实现后 | `cargo test -p vw-agent-apps usage_subscriptions_gemini_ -- --nocapture` | 退出 0，5 项通过，0 项失败。 |
| 共享工作区 | `cargo clippy --all-targets --all-features -- -D warnings` | 退出 101。并行的 ACP 权限改动在 `vw-agent/src/session/llm/acp/approval.rs` 缺少 `PermissionConfirmation`，在 `vw-desktop/src/app/message/chat/stream/permissions.rs` 缺少 `session_set_full_access`，多处缺少 `GatewaySessionUiSummary.full_access`。 |
| 隔离复核 | `cargo clippy --all-targets --all-features -- -D warnings` | 退出 0。源码基于批次 27 提交 `2105e1089`、本任务的三个 Rust 文件及工作区已有的 `postgres_tests.rs` 第三参数修正。 |

本机 HTTP 夹具核对续期表单、两步 POST 的请求体和 Bearer，覆盖已用尽的零剩余值、缺失比例、重置时间、缺少凭据、解析失败和上游拒绝。差异检查通过，两个槽位锁已释放，缓存已保留。真实账号与系统钥匙串留待任务 57 验收。 [聚焦测试](../../../../crates/vw-agent-apps/src/usage/subscriptions/gemini_tests.rs#L144)

## 任务 29 Grok 与 xAI 官方额度

`query_grok` 使用执行端已授权的账号引用。本机入口从授权 HOME 下的 `.grok/auth.json` 读取 OIDC 或旧版登录凭据。已到期且仍可由 CLI 刷新的凭据返回待刷新。托管入口用现有 `XaiRefresh` 为已选账号取短期令牌。两个入口复用固定账单地址及解析。 [本地入口](../../../../crates/vw-agent-apps/src/usage/subscriptions/grok.rs#L51) · [Grok CLI 来源](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/subscription_grok.rs#L702) · [xAI 托管来源](../../../../crates/vw-agent-apps/upstream/src-tauri/src/commands/xai_oauth.rs#L29) · [TS 绑定与查询规则](../../../../crates/vw-agent-apps/upstream/src/lib/query/subscription.ts#L186)

固定来源的 Grok Web gRPC 查询提供一个用量窗口。Grok CLI 官方账单 JSON 补充当前周期、旧版月度额度、预付钱包及按需用量。当前周期与不同的月度窗口分别保留。钱包和按需金额按美分转换为 USD，保留显式零值。钱包复用 `UsageWallet`，与官方订阅结果分开。 [固定来源 RPC](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/subscription_grok.rs#L570) · [xAI CLI 账单字段与请求](https://github.com/xai-org/grok-build/blob/2bdd1d6a6369de0e8c68132ea4539e9abd9e14a8/crates/codegen/xai-grok-shell/src/extensions/billing.rs#L60) · [本地投影](../../../../crates/vw-agent-apps/src/usage/subscriptions/grok.rs#L357)

两个账单接口各自返回类型化错误。单路失败时保留另一路已取得的数据和失败接口的安全说明。gRPC 状态码来自响应头或 trailer，响应正文及错误消息原文不进入结果。 [本地查询](../../../../crates/vw-agent-apps/src/usage/subscriptions/grok.rs#L113) · [gRPC 状态与帧解析](../../../../crates/vw-agent-apps/src/usage/subscriptions/grok/grpc.rs#L25)

验证环境为 macOS 27.0 (26A428)、arm64，`vw-agent-apps` 0.6.0。实现前后的聚焦测试、格式化及共享工作区 Clippy 动态领取槽位 1，`CARGO_TARGET_DIR` 为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。隔离 Clippy 动态领取槽位 2，`CARGO_TARGET_DIR` 为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。各阶段的 `CARGO_BUILD_BUILD_DIR` 与相应槽位一致。

| 阶段 | 命令 | 结果 |
| --- | --- | --- |
| 实现前 | `cargo test -p vw-agent-apps usage_subscriptions_grok_ -- --nocapture` | 退出 101，执行 0 项。包含 `usage_subscriptions_grok_returns_expected_result` 和 `usage_subscriptions_grok_rejects_invalid_input` 的测试模块报 E0432，缺少 `super::grok`。 |
| 格式化 | `cargo fmt --all` | 退出 0。在源码副本运行，仅同步本任务 Rust 文件。 |
| 实现后 | `cargo test -p vw-agent-apps usage_subscriptions_grok_ -- --nocapture` | 退出 0，4 项通过，0 项失败。 |
| 共享工作区 | `cargo clippy --all-targets --all-features -- -D warnings` | 退出 101。已有 ACP 权限改动引用缺失的 `PermissionConfirmation`、`GatewayClient::session_set_full_access` 和 `GatewaySessionUiSummary.full_access`。 |
| 隔离复核 | `cargo clippy --all-targets --all-features -- -D warnings` | 退出 0。源码基于批次 28 提交 `518e91821`，叠加本任务 Rust 文件和工作区已有的 `postgres_tests.rs` 第三参数修正。 |

本机 HTTP 夹具核对 CLI 与托管令牌、请求方法、路径、请求头及 gRPC 空帧。测试覆盖双窗口、预付钱包零值、按需金额、无效或到期凭据、双向部分失败及完全拒绝。差异检查通过，槽位锁已释放并保留缓存。真实账号留待任务 57 验收。 [聚焦测试](../../../../crates/vw-agent-apps/src/usage/subscriptions/grok_tests.rs#L205)

## 任务 30 AGY 官方额度

`query_agy_account` 使用调用方已授权的明确账号及其作用域。供应商调用先解析保存的绑定。托管入口从钥匙串读取该账号的有效令牌并直接 POST `retrieveUserQuotaSummary`。令牌过期时复用已有刷新流程，HTTP 401 时强制刷新一次再试。查询接受已探测的 CLI 版本，缺失时使用来源快照的 1.4.6。托管查询失败也保留其来源。 [账号入口](../../../../crates/vw-agent-apps/src/usage/subscriptions/agy.rs#L48) · [账号查询来源](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/agy_account/mod.rs#L389) · [HTTP 来源](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/agy_account/quota.rs#L149) · [TS 账号规则](../../../../crates/vw-agent-apps/upstream/src/lib/query/subscription.ts#L217)

`query_agy_cli` 使用目标主机独占的原生登录槽，持有读锁并执行固定的 `agy --print /usage --output-format json`。进程环境由主机提供，排除外部 API Key 及自定义地址。令牌刷新在阻塞线程等待同一物理锁。两种响应共享 Gemini、Claude/GPT 两组各自的五小时和每周窗口。同一窗口重复时取较高用量及对应重置时间。未知桶跳过。HTTP 403 通过 `loadCodeAssist` 区分资格限制和其他访问拒绝，返回固定安全说明。 [CLI 入口](../../../../crates/vw-agent-apps/src/usage/subscriptions/agy.rs#L74) · [刷新锁](../../../../crates/vw-agent-apps/src/auth/agy_refresh.rs#L111) · [窗口投影](../../../../crates/vw-agent-apps/src/usage/subscriptions/agy.rs#L327) · [CLI 来源](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/subscription_agy.rs#L115) · [资格检查来源](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/agy_account/quota.rs#L169)

验证环境为 macOS 27.0 (26A428)、arm64，`vw-agent-apps` 0.6.0。各阶段动态领取槽位 1，`CARGO_TARGET_DIR` 和隔离验证的 `CARGO_BUILD_BUILD_DIR` 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。隔离验证基于批次 29 提交 `58e9143ae`，叠加本任务 Rust 文件和工作区已有的 `postgres_tests.rs` 第三参数修正。格式化后仅同步本任务文件。

| 阶段 | 命令 | 结果 |
| --- | --- | --- |
| 实现前 | `cargo test -p vw-agent-apps usage_subscriptions_agy_ -- --nocapture` | 退出 101，执行 0 项。包含 `usage_subscriptions_agy_returns_expected_result` 和 `usage_subscriptions_agy_rejects_invalid_input` 的模块报 E0432，缺少 `super::agy`。 |
| 格式化 | `cargo fmt --all` | 退出 0。 |
| 实现后 | `cargo test -p vw-agent-apps usage_subscriptions_agy_ -- --nocapture` | 退出 0，4 项通过，0 项失败。 |
| 授权刷新 | `cargo test -p vw-agent-apps auth6_agy_refresh_ -- --nocapture --test-threads=1` | 退出 0，2 项通过，0 项失败。 |
| 隔离复核 | `cargo clippy --all-targets --all-features -- -D warnings` | 退出 0。 |

HTTP 与 CLI 夹具核对请求、来源分离、额度归并、重置时间、拒绝和脱敏。差异检查通过，槽位锁已释放，缓存已保留。真实账号留待任务 57 验收。 [聚焦测试](../../../../crates/vw-agent-apps/src/usage/subscriptions/agy_tests.rs#L126)

## 任务 31 Copilot 额度

`query_copilot` 查询额度解析器选中的显式、绑定或托管默认账号。默认账号由同一租户、主体、主机和服务范围内的已保存选择提供。找不到所选账号或它的凭据时直接失败。执行主机从所选账号的授权记录读取 GitHub 域名，并在授权后使用该账号的 OAuth 凭据向 GitHub 或企业实例发送固定的 `GET /copilot_internal/user` 请求。请求沿用母任务 6 的请求头及禁止重定向策略。 [账号解析](../../../../crates/vw-agent-apps/src/usage/binding.rs#L49) · [额度入口](../../../../crates/vw-agent-apps/src/usage/subscriptions/copilot.rs#L29) · [请求构造](../../../../crates/vw-agent-apps/src/auth/copilot.rs#L288) · [上游命令](../../../../crates/vw-agent-apps/upstream/src-tauri/src/commands/copilot.rs#L201-L221) · [上游 HTTP](../../../../crates/vw-agent-apps/upstream/src-tauri/src/proxy/providers/copilot_auth.rs#L1107)

结果保留套餐、Premium 请求数和原始重置日期。总量大于零时以已用量除以总量计算百分比，总量不大于零时返回零。HTTP 401 表示登录过期，HTTP 403 表示上游拒绝。额度查询只读凭据，失败说明不包含上游响应或凭据。未设置托管默认号时返回未找到。 [额度转换](../../../../crates/vw-agent-apps/src/usage/subscriptions/copilot.rs#L145) · [TS 查询规则](../../../../crates/vw-agent-apps/upstream/src/lib/query/copilot.ts#L28) · [页脚账号选择](../../../../crates/vw-agent-apps/upstream/src/components/CopilotQuotaFooter.tsx#L25)

验证环境为 macOS 27.0 (26A428)、arm64，`vw-agent-apps` 0.6.0。各阶段动态领取槽位 1，`CARGO_TARGET_DIR` 为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。实现后验证的 `CARGO_BUILD_BUILD_DIR` 使用同一路径。

| 阶段 | 命令 | 结果 |
| --- | --- | --- |
| 实现前 | `cargo test -p vw-agent-apps usage_subscriptions_copilot_ -- --nocapture` | 退出 101，执行 0 项。包含两个计划测试的模块报 E0432，缺少 `super::copilot`。 |
| 格式化 | `cargo fmt --all` | 退出 0。在源码副本运行，仅同步本任务 Rust 文件。 |
| 实现后 | `cargo test -p vw-agent-apps usage_subscriptions_copilot_ -- --nocapture` | 退出 0，2 项通过，0 项失败。 |
| 授权回归 | `cargo test -p vw-agent-apps auth6_copilot_token_ -- --nocapture` | 退出 0，2 项通过，0 项失败。 |
| 共享工作区 | `cargo clippy --all-targets --all-features -- -D warnings` | 退出 101。现有 ACP 改动缺少 `vw-acp` 的 `PermissionConfirmation` 和 `with_permission_confirmation`，以及网关客户端的 `session_set_full_access` 和 `GatewaySessionUiSummary.full_access`。 |
| 隔离复核 | `cargo clippy --all-targets --all-features -- -D warnings` | 退出 0。基于批次 30 提交 `c3b018399`，叠加本任务 Rust 文件和共享工作区已有的 `postgres_tests.rs` 第三参数修正。 |

`usage_subscriptions_copilot_returns_expected_result` 和 `usage_subscriptions_copilot_rejects_invalid_input` 使用本机 HTTP 夹具核对两种账号选择、企业域名、请求头、重置日期、缺失凭据、拒绝及脱敏。差异检查通过，槽位锁已释放，编译缓存已保留。真实账号留待任务 57 验收。 [聚焦测试](../../../../crates/vw-agent-apps/src/usage/subscriptions/copilot_tests.rs#L97)

## 任务 32 Kimi 账号额度

`query_kimi_account` 使用主机已核验的账号、区域及授权范围。有效令牌直接请求该区域的 `GET /coding/v1/usages`，复用 Kimi Coding Plan 的限长响应解析。官方订阅结果保留五小时和周窗口的额度、比例及重置时间。HTTP 401 或 403 返回已过期状态，错误说明不包含上游响应。 [查询入口](../../../../crates/vw-agent-apps/src/usage/subscriptions/kimi.rs#L24) · [用量解析](../../../../crates/vw-agent-apps/src/usage/coding/kimi.rs#L114) · [上游账号查询](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/kimi_oauth/account.rs#L379-L440) · [页脚账号选择](../../../../crates/vw-agent-apps/upstream/src/components/KimiAccountQuotaFooter.tsx#L32-L39) · [按账号查询键](../../../../crates/vw-agent-apps/upstream/src/lib/query/subscription.ts#L252-L277)

令牌过期时由执行主机提供已授权的原生配置。授权模块在托管账号租约和 Kimi 原生文件锁下核验账号绑定或共享令牌，再复用既有刷新与身份校验。共享令牌轮换后同步托管凭据，查询不会改写默认模型。没有获准的原生配置或锁已被占用时返回待续期。 [授权续期](../../../../crates/vw-agent-apps/src/auth/kimi_account/usage.rs#L12) · [共享凭据保存](../../../../crates/vw-agent-apps/src/auth/kimi_account.rs#L437) · [上游文件锁续期](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/kimi_oauth/account.rs#L246-L322)

验证环境为 macOS 27.0 (26A428)、arm64，`vw-agent-apps` 0.6.0。
各阶段动态领取槽位 1，`CARGO_TARGET_DIR` 和 `CARGO_BUILD_BUILD_DIR` 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。

| 阶段 | 命令 | 结果 |
| --- | --- | --- |
| 实现前 | `cargo test -p vw-agent-apps usage_subscriptions_kimi_ -- --nocapture` | 退出 101，执行 0 项。测试模块报 E0432，缺少 `super::kimi`。 |
| 格式化 | `cargo fmt --all` | 退出 0。在源码副本运行，仅同步本任务 Rust 文件。 |
| 实现后 | `cargo test -p vw-agent-apps usage_subscriptions_kimi_ -- --nocapture` | 退出 0，3 项通过，0 项失败。 |
| 套餐回归 | `cargo test -p vw-agent-apps usage_coding_kimi_ -- --nocapture` | 退出 0，5 项通过，0 项失败。 |
| 授权回归 | `cargo test -p vw-agent-apps auth6_kimi_account_ -- --nocapture` | 退出 0，2 项通过，0 项失败。 |
| 共享工作区 | `cargo clippy --all-targets --all-features -- -D warnings` | 退出 101。现有 `vw-agent` ACP 改动缺少 `PermissionConfirmation` 和 `with_permission_confirmation`，`vw-agent` 与 `vw-desktop` 的会话改动缺少网关客户端 `full_access` 和 `session_set_full_access`。 |
| 隔离复核 | `cargo clippy --all-targets --all-features -- -D warnings` | 退出 0。使用批次 31 提交 `2a4a62c36`，叠加本任务 Rust 文件及共享工作区已有的 `postgres_tests.rs` 第三参数修正。 |

`usage_subscriptions_kimi_returns_expected_result` 验证续期锁、区域请求、共享凭据写回及两档额度。`usage_subscriptions_kimi_rejects_invalid_input` 验证区域不符和锁冲突。`usage_subscriptions_kimi_queries_valid_managed_account_directly` 验证有效令牌直接查询、上游拒绝及脱敏。差异检查通过，槽位锁已释放，缓存已保留。HTTP 夹具验收覆盖上述契约，真实账号留待任务 57。 [独立行为测试](../../../../crates/vw-agent-apps/src/usage/subscriptions/kimi_tests.rs#L28)

## 任务 33 脚本模板与参数

`UsageTemplate::select` 保留上游模板标识，优先处理受信账号的原生查询，再选择已保存模板、旧版 New API 参数和已检测的原生能力。Copilot、官方订阅、余额及 Coding Plan 返回 `PreparedUsage::Native`，该分支不读取脚本源码或凭据。 [模板选择](../../../../crates/vw-agent-apps/src/usage/scripts/config.rs#L113) · [上游选择](../../../../crates/vw-agent-apps/upstream/src/components/UsageScriptModal.tsx#L493-L533) · [上游原生分流](../../../../crates/vw-agent-apps/upstream/src-tauri/src/commands/provider.rs#L658-L710)

`UsageTemplate::prepare` 提供 Custom、General、New API 和 Sub2API 预设，并接受已保存的脚本代码。主机解析的非空参数覆盖供应商默认值，基础 URL 去除尾部斜杠。四种占位符只展开一次，值按 UTF-16 代码单元写成 JavaScript 转义，保留引号、反引号及字面占位符。缺少被引用的参数时返回类型化错误。预设内的后备套餐名和错误文案沿用上游英文文本。 [脚本准备](../../../../crates/vw-agent-apps/src/usage/scripts/config.rs#L139) · [上游预设](../../../../crates/vw-agent-apps/upstream/src/components/UsageScriptModal.tsx#L60-L169) · [上游参数优先级](../../../../crates/vw-agent-apps/upstream/src-tauri/src/services/provider/usage.rs#L96-L125) · [上游变量替换](../../../../crates/vw-agent-apps/upstream/src-tauri/src/usage_script.rs#L425-L444)

验证环境为 macOS 27.0 (26A428)、arm64，`vw-agent-apps` 0.6.0。
各阶段动态领取槽位 1，`CARGO_TARGET_DIR` 和 `CARGO_BUILD_BUILD_DIR` 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。

| 阶段 | 命令 | 结果 |
| --- | --- | --- |
| 实现前 | `cargo test -p vw-agent-apps usage_scripts_config_ -- --nocapture` | 退出 101，执行 0 项。测试模块按预期报 E0432，缺少 `super::config` 中的配置符号。 |
| 格式化 | `cargo fmt --all` | 退出 0。在源码副本运行，仅同步本任务 Rust 文件。 |
| 实现后 | `cargo test -p vw-agent-apps usage_scripts_config_ -- --nocapture` | 退出 0，3 项通过，0 项失败。 |
| 共享工作区 | `cargo clippy --all-targets --all-features -- -D warnings` | 退出 101。既有 ACP 改动在 `vw-agent/src/session/llm/acp` 缺少 `PermissionConfirmation` 和 `with_permission_confirmation`。网关会话与桌面代码还缺少客户端的 `session_set_full_access` 和 `GatewaySessionUiSummary.full_access`。 |
| 隔离复核 | `cargo clippy --all-targets --all-features -- -D warnings` | 退出 0。基于批次 32 提交 `043277e99`，叠加本任务 Rust 文件及共享工作区已有的 `postgres_tests.rs` 第三个参数修正。 |

`usage_scripts_config_returns_expected_result` 核对模板标识、参数优先级、URL 规范化及特殊字符还原。`usage_scripts_config_rejects_invalid_input` 核对缺参、空源码及未知模板。`usage_scripts_config_keeps_native_templates_out_of_javascript` 核对原生查询选择与脚本隔离。另以本机 Node 运行四个预设的请求和提取夹具。差异检查通过，槽位锁已释放，编译缓存已保留。QuickJS 和出站行为分别由任务 34、35 验证。 [独立行为测试](../../../../crates/vw-agent-apps/src/usage/scripts/config_tests.rs#L8)

## 任务 34 QuickJS 执行资源

`PreparedUsageScript::build_request` 与 `PreparedUsageScript::extract` 各自在 Tokio 阻塞线程上创建独立 QuickJS 实例，使用 5 秒线程 CPU 时间、16 MiB QuickJS 堆和 256 KiB QuickJS 栈。解释器轮询时中断超时，原生引擎调用返回后复核 CPU 用量。运行时仅返回待宿主处理的 JSON，错误保留类型且不携带异常文本、凭据或响应原文。网络请求与套餐校验由后续任务处理。 [运行时](../../../../crates/vw-agent-apps/src/usage/scripts/runtime.rs#L44) · [上游限额](../../../../crates/vw-agent-apps/upstream/src-tauri/src/usage_script.rs#L31) · [上游无限循环用例](../../../../crates/vw-agent-apps/upstream/src-tauri/src/usage_script.rs#L692)

来源 Cargo.lock 使用 rquickjs 0.8.1。本任务为 macOS 接入 rquickjs 0.14.0（内置 QuickJS-NG 0.16.2），仅启用 `std`。 [上游依赖](../../../../crates/vw-agent-apps/upstream/src-tauri/Cargo.toml#L70) · [本仓库依赖](../../../../crates/vw-agent-apps/Cargo.toml#L33) · [rquickjs 0.14.0](https://github.com/DelSkayn/rquickjs/releases/tag/v0.14.0)

验证环境为 macOS 27.0 (26A428)、arm64，`vw-agent-apps` 0.6.0。各阶段动态领取槽位 1，`CARGO_TARGET_DIR` 和 `CARGO_BUILD_BUILD_DIR` 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。

| 阶段 | 命令 | 结果 |
| --- | --- | --- |
| 实现前 | `cargo test -p vw-agent-apps usage_scripts_runtime_ -- --nocapture` | 退出 101，执行 0 项。`usage_scripts_runtime_returns_expected_result` 和 `usage_scripts_runtime_rejects_invalid_input` 等用例按预期报 E0432/E0599，缺少运行时模块及两个入口。 |
| 格式化 | `cargo fmt --all` | 退出 0。仅同步源码副本中属于本任务的文件。 |
| 实现后 | `cargo test -p vw-agent-apps usage_scripts_runtime_ -- --nocapture` | 退出 0，6 项通过，0 项失败，耗时 5.01 秒。 |
| 共享工作区 | `cargo clippy --all-targets --all-features -- -D warnings` | 退出 101。既有 ACP 改动引用了缺少的 `PermissionConfirmation` 和 `with_permission_confirmation`。桌面与网关会话代码还引用了缺少的 `session_set_full_access` 和 `GatewaySessionUiSummary.full_access`。 |
| 隔离复核 | `cargo clippy --all-targets --all-features -- -D warnings` | 退出 0。基于批次 33 提交 `3507c1106`，叠加本任务变更和共享工作区已有的 `postgres_tests.rs` 第三个参数修正。 |

成功夹具验证 General 模板构造请求和解析响应。拒绝夹具覆盖脚本异常脱敏、无效响应 JSON、两阶段无限循环、17 MiB 分配和深递归。请求循环用例还验证同一 Tokio 单线程执行器上的定时器可在脚本运行时推进。夹具使用真实 QuickJS 和合成输入，验证范围为运行时行为。差异审查通过，共享槽位锁已释放，编译缓存已保留。 [独立行为测试](../../../../crates/vw-agent-apps/src/usage/scripts/runtime_tests.rs#L30)

## 任务 35 脚本请求网络

`PreparedUsageScript::send_request` 用受限 QuickJS 生成请求，保留合法的 HTTP 方法、请求头和请求体，将宿主提供的超时限制在 2–30 秒并覆盖响应体读取。目标必须匹配宿主明确授予本次查询及其凭据使用的协议、主机和有效端口。非自定义模板还要求与已配置的基础地址同源，仅允许 HTTPS 或回环 HTTP。自定义模板可以访问与基础地址不同的已授权 HTTP(S) 端点。请求禁用环境代理、自动重试和重定向，并拒绝 URL 用户信息及能覆盖目标或报文边界的请求头。响应大小与 QuickJS 输入上限一致，错误分类不携带 URL、凭据或响应内容。 [上游请求方法和超时](../../../../crates/vw-agent-apps/upstream/src-tauri/src/usage_script.rs#L235-L274) · [上游地址校验](../../../../crates/vw-agent-apps/upstream/src-tauri/src/usage_script.rs#L497-L584) · [TS 超时输入](../../../../crates/vw-agent-apps/upstream/src/components/UsageScriptModal.tsx#L448-L466) · [本地网络入口](../../../../crates/vw-agent-apps/src/usage/scripts/network.rs#L47)

验证环境为 macOS 27.0 (26A428)、arm64，`vw-agent-apps` 0.6.0。各阶段动态领取槽位 1，`CARGO_TARGET_DIR` 与 `CARGO_BUILD_BUILD_DIR` 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。

| 阶段 | 命令 | 结果 |
| --- | --- | --- |
| 实现前 | `cargo test -p vw-agent-apps usage_scripts_network_ -- --nocapture` | 退出 101，执行 0 项。成功及拒绝用例因缺少 `NetworkError` 和 `send_request` 无法编译（E0432/E0599）。 |
| 格式化 | `cargo fmt --all` | 退出 0。仅将本任务文件从源码副本同步回共享工作区。 |
| 实现后 | `cargo test -p vw-agent-apps usage_scripts_network_ -- --nocapture` | 退出 0，6 项通过，0 项失败，耗时 2.01 秒。 |
| 共享工作区 | `cargo clippy --all-targets --all-features -- -D warnings` | 退出 101。既有 ACP 会话改动引用了缺失的 `PermissionConfirmation` 和 `with_permission_confirmation`，桌面与网关会话改动引用了缺失的 `session_set_full_access` 和 `GatewaySessionUiSummary.full_access`。 |
| 隔离复核 | `cargo clippy --all-targets --all-features -- -D warnings` | 退出 0。基于批次 34 提交 `38e9a3b2c`，叠加本任务源码及共享工作区已有的 `postgres_tests.rs` 第三个参数修正。 |

真实 QuickJS 与本机 HTTP 夹具核对 PATCH 方法、路径、查询串、请求头、请求体和提取结果，并确认未授权端口与已获授权的重定向目标均未收到连接。拒绝用例还覆盖无效方法、URL 用户信息、非 HTTP(S) 协议和 Host 头覆盖。超时用例覆盖 2–30 秒边界及响应体停滞，HTTP 403 用例验证类型化鉴权错误。验收覆盖宿主网络入口，凭据、响应和目标来自本机夹具。差异审查通过，共享槽位锁已释放，编译缓存已保留。 [独立行为测试](../../../../crates/vw-agent-apps/src/usage/scripts/network_tests.rs#L79)

## 任务 36 校验脚本返回套餐

`PreparedUsageScript::extract_packages` 将单对象或非空对象数组转换为 `UsageScriptPackage`，按顺序保留八个约定字段。零和负数保留，未提供的可选字段继续表达为缺失。有效性未声明或为真的套餐至少需要一个额度值。显式失效的套餐可只提供失效状态和说明。错误按类型区分，省略脚本异常和响应原文。套餐文本含有主机传入的 API Key 或访问令牌原文时拒绝整个结果。 [上游结果校验](../../../../crates/vw-agent-apps/upstream/src-tauri/src/usage_script.rs#L315) · [上游套餐类型](../../../../crates/vw-agent-apps/upstream/src-tauri/src/provider.rs#L342) · [TS 字段说明](../../../../crates/vw-agent-apps/upstream/src/components/UsageScriptModal.tsx#L1671) · [本地结果类型](../../../../crates/vw-api-types/src/apps_usage.rs#L148) · [本地提取入口](../../../../crates/vw-agent-apps/src/usage/scripts/extract.rs#L35)

验证环境为 macOS 27.0 (26A428)、arm64，`vw-agent-apps` 0.6.0。各阶段动态领取槽位 1，`CARGO_TARGET_DIR` 与 `CARGO_BUILD_BUILD_DIR` 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。

| 阶段 | 命令 | 结果 |
| --- | --- | --- |
| 实现前 | `cargo test -p vw-agent-apps usage_scripts_extract_ -- --nocapture` | 退出 101，执行 0 项。成功、拒绝和凭据用例按预期缺少模块及 `extract_packages`（E0432/E0599）。 |
| 格式化 | `cargo fmt --all` | 退出 0。仅将本任务文件从源码副本同步回共享工作区。 |
| 实现后 | `cargo test -p vw-agent-apps usage_scripts_extract_ -- --nocapture` | 退出 0，3 项通过，0 项失败。 |
| 脚本模块回归 | `cargo test -p vw-agent-apps usage_scripts_ -- --nocapture` | 退出 0，18 项通过，0 项失败。 |
| 共享工作区 | `cargo clippy --all-targets --all-features -- -D warnings` | 退出 101。既有 ACP 会话改动缺少 `PermissionConfirmation` 和 `with_permission_confirmation`，桌面会话改动缺少 `session_set_full_access` 和 `GatewaySessionUiSummary.full_access`。 |
| 隔离复核 | `cargo clippy --all-targets --all-features -- -D warnings` | 退出 0。基于批次 35 提交 `daba0a2a1`，叠加本任务源码和共享工作区已有的 `postgres_tests.rs` 参数修正。 |

真实 QuickJS 夹具覆盖单套餐和多套餐、零与负数、显式失效、缺值、字段类型错误、非法 JSON、NaN 和无穷值，以及凭据出现在套餐字段和脚本异常中的情况。本机 HTTP 夹具也验证了收到响应后调用受校验的提取入口。差异审查通过，槽位锁已释放，编译缓存已保留。 [独立行为测试](../../../../crates/vw-agent-apps/src/usage/scripts/extract_tests.rs#L27) · [本机 HTTP 测试](../../../../crates/vw-agent-apps/src/usage/scripts/network_tests.rs#L79)

## 任务 37 取消脚本查询

`PreparedUsageScript::run` 复用上游的请求构造、HTTP 与提取顺序。每次查询由调用者提供独立 `Operation`。QuickJS 在构造和提取阶段轮询取消信号，查询等待阻塞线程退出后返回 `RunError::Cancelled`。取消 HTTP 阶段会释放请求或响应体。取消结果无套餐可供写入成功缓存。缓存层接入时负责协调提交与取消的竞争。 [上游执行流程](../../../../crates/vw-agent-apps/upstream/src-tauri/src/usage_script.rs#L9-L164) · [TS 测试入口](../../../../crates/vw-agent-apps/upstream/src/components/UsageScriptModal.tsx#L736-L747) · [任务契约](../../../vibewindow/agent-apps/任务事件与错误契约.md#L23) · [查询入口](../../../../crates/vw-agent-apps/src/usage/scripts/run.rs#L34)

验证环境为 macOS 27.0 (26A428)、arm64，`vw-agent-apps` 0.6.0。各阶段动态领取槽位 1，`CARGO_TARGET_DIR` 为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。首次失败验证和首次实现后编译继承了仓库的中间构建目录配置。修正后 `CARGO_BUILD_BUILD_DIR` 也指向槽位 1。

| 阶段 | 命令 | 结果 |
| --- | --- | --- |
| 实现前 | `cargo test -p vw-agent-apps usage_scripts_run_ -- --nocapture` | 退出 101，执行 0 项。`usage_scripts_run_returns_expected_result` 和 `usage_scripts_run_rejects_invalid_input` 等用例按预期缺少模块及 `run`（E0432/E0599）。 |
| 格式化 | `cargo fmt --all` | 退出 0。仅同步源码副本中属于本任务的文件。 |
| 实现后 | `cargo test -p vw-agent-apps usage_scripts_run_ -- --nocapture` | 退出 0，5 项通过，0 项失败。 |
| 脚本回归 | `cargo test -p vw-agent-apps usage_scripts_ -- --nocapture` | 退出 0，23 项通过，0 项失败。 |
| 生命周期取消回归 | `cargo test -p vw-agent-apps cc_apps_operation_cancel_ -- --nocapture` | 退出 0，3 项通过，0 项失败。 |
| 共享工作区 | `cargo clippy --all-targets --all-features -- -D warnings` | 退出 101。既有 ACP 会话改动缺少 `PermissionConfirmation` 和 `with_permission_confirmation`，桌面和网关会话改动引用了缺失的 `session_set_full_access` 与 `GatewaySessionUiSummary.full_access`。 |
| 失败复验 | `cargo test -p vw-agent-apps usage_scripts_run_ -- --nocapture` | 在批次 36 源码副本中仅加测试，退出 101，执行 0 项，复现 E0432/E0599。 |
| 隔离复核 | `cargo clippy --all-targets --all-features -- -D warnings` | 退出 0。基于批次 36 提交 `54919a7c5`，叠加本任务 Rust 文件和共享工作区已有的 `postgres_tests.rs` 参数修正。 |

真实 QuickJS 和本机 HTTP 夹具核对成功结果、预先取消、请求构造循环、提取循环及响应体停滞。并发夹具验证取消后连接关闭，同源另一笔查询正常返回。差异检查通过，槽位锁已释放，缓存已保留。真实账号由后续任务验收。 [独立行为测试](../../../../crates/vw-agent-apps/src/usage/scripts/run_tests.rs#L69)

## 任务 38 保存查询设置

`UsageSettingsStore::{get,save}` 复用 `GatewaySettingsStore` 的 SQLite/PostgreSQL 后端，在独立命名空间原子替换整条设置。平台与公司由底座隔离，配置键再区分项目、主体、目标主机、应用、供应商和可选账号。保存模板、查询开关、0–1440 分钟刷新间隔及剩余/已用显示方式。0 表示关闭自动刷新，缺省显示剩余。API key、访问令牌和成对的 AK/SK 使用 UUID v4 引用。脚本源码与凭据原值留在执行侧，后续设置 API 在调用存储前校验作用域和引用使用权。 [上游保存模板](../../../../crates/vw-agent-apps/upstream/src/components/UsageScriptModal.tsx#L559-L577) · [刷新间隔规则](../../../../crates/vw-agent-apps/upstream/src/components/UsageScriptModal.tsx#L467-L491) · [显示规则](../../../../crates/vw-agent-apps/upstream/src/components/quota/useQuotaDisplay.ts#L1-L8) · [实现](../../../../crates/vw-agent/src/agent_apps/usage_settings.rs)

验证环境为 macOS 27.0 (26A428)、arm64，`vw-agent` 0.6.0，本机 PostgreSQL 18.4。两个阶段均动态领取槽位 1，`CARGO_TARGET_DIR` 和 `CARGO_BUILD_BUILD_DIR` 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。

| 阶段 | 命令 | 结果 |
| --- | --- | --- |
| 实现前 | `cargo test -p vw-agent usage_usage_settings_ -- --nocapture` | 退出 101，执行 0 项。`usage_usage_settings_returns_expected_result` 和 `usage_usage_settings_rejects_invalid_input` 等用例按预期因缺少 `usage_settings` 模块而失败（E0432）。 |
| 格式化 | `cargo fmt --all` | 退出 0。仅将第 38 项的三个 Rust 文件从源码副本同步回共享工作区。 |
| 实现后 | `cargo test -p vw-agent usage_usage_settings_ -- --nocapture` | 退出 0，3 项通过，0 项失败。 |
| 共享工作区 | `cargo clippy --all-targets --all-features -- -D warnings` | 退出 101。现有 ACP 改动引用缺失的 `PermissionConfirmation` 和 `with_permission_confirmation`，桌面与网关会话改动引用缺失的 `session_set_full_access` 与 `GatewaySessionUiSummary.full_access`。 |
| 隔离复核 | `cargo clippy --all-targets --all-features -- -D warnings` | 退出 0。基于批次 37 提交 `02377910a`，叠加本任务 Rust 文件和共享工作区已有的 `session/store/postgres_tests.rs` 参数修正。 |

SQLite 和 PostgreSQL 夹具实际保存并重读设置，验证各作用域维度的读取隔离、不同主机的独立更新、0 分钟与默认显示。拒绝测试检查越界间隔、非法主机、原始密钥字段及非法引用格式，并确认失败后原设置完整。差异审查通过，槽位锁已释放，编译缓存已保留。 [独立行为测试](../../../../crates/vw-agent/src/agent_apps/usage_settings_tests.rs#L89)
