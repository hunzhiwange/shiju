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
