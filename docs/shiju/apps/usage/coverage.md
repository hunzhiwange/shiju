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
