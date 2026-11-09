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
