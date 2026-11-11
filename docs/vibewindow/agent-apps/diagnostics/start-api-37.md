# 任务 37：诊断启动 API 阶段验收记录

2026-11-11 至 2026-11-12，macOS 27.0（26A428），arm64。当前目录实施，未创建工作树，未修改只读 plan.md。七个代码/测试文件已由同目录提交 `c2beaee9f` 收录，逐项核对与本任务补丁一致；本记录单独提交。

## 实现与授权

- `gateway/api/handlers/agent_apps_diagnostics.rs` 注册 `POST /v1/agent-apps/diagnostics/jobs`；`bootstrap.rs` 接入业务路由及路由清单。
- 查询使用 `DiagnosticScopeQuery`，租户来自网关可信 `TenantScope`。正文使用已有 `DiagnosticStartRequest`，拒绝不属于该 kind 的字段，不能覆盖 URL、命令、环境、账号或凭据。
- `agent_apps/access.rs::authorize_diagnostic_probe` 复用现有 `routing_manage`、项目/应用 `routing_apps` 和 `provider_config_read`。该要求沿用 `app_routing/outbound_probe.rs` 的主动诊断管理约定，不新增权限配置或人工审批。模型另由现有执行器核对保存的部署、目录和账号代次。
- MCP 对整批配置先核对已有 `mcp_probe`，再调用 `agent_apps/mcp/network_probe.rs::start_handshakes`，继续复核配置、摘要、凭据及 stdio 许可。未授权成员不会先读取配置或提交探测。
- 五种 kind 分别接入 `ReachabilityProbe`、`speedtest::start`、`ModelProbe`、`StreamProbe`、`start_handshakes`，不复制执行器。测速 ID 是已保存候选规范化 URL 的 SHA-256 十六进制值；只解析当前已授权供应商的候选，不接受客户端 URL。
- `app_routing/lifecycle_api.rs` 将已有 `DiagnosticJobs` 同时注入启动处理器与 `RoutingControl`，共享主机并发/容量限制；无路由管理配置时仍可初始化诊断注册表。
- 受理返回 202 和 `DiagnosticJobIdentity`，响应禁止缓存；拒绝使用脱敏公共错误体与关联 ID。主机离线/远程继续沿用授权底座的明确错误，不回退本机。

请求形状：

```text
POST /v1/agent-apps/diagnostics/jobs?projectId=project&targetHostId=host&appId=claude
Content-Type: application/json
{"kind":"model","providerId":"provider","modelId":"configured-model"}
```

## 检查证据与限制

前期 Cargo 阶段使用动态原子领取的槽位 **7**；夹具修复后的阶段重新顺次领取到 **2**（见末尾）：

```text
CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-7
CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-7
```

| 命令 | 退出码 / 有效测试数 | 结果 |
| --- | --- | --- |
| `cargo fmt --all -- --emit stdout --config skip_children=true` | 0 / 不适用 | 全仓入口输出到临时文件；仅用 rustfmt 写回负责文件，保留其他任务改动 |
| `cargo test -p vw-agent --lib diagnostic_api_start_authorizes_before_execution -- --nocapture`（红灯尝试） | 130 / 0 | 先出现无关 PiDirectoryRead/Write/State 未解析及 serde 属性错误；其他所有者补齐类型后停止仍持旧快照的本任务 Cargo/rustc，非行为红灯 |
| 同一聚焦命令（最终实现） | 130 / 0 | 先出现 `agent_apps/access.rs` 的其他任务 `AuthorizedMcpProbe.pi_directory` 上的无效 serde 属性，以及 `computer_control/native/observation_tests.rs` 的 `request_permission` / `NativePermission` trait 不匹配；停止自己的已失败构建后继续规定 Clippy，未运行测试 |
| `cargo clippy --all-targets --all-features -- -D warnings`（首次） | 130 / 不执行测试 | 先出现无关 ACP `drivers_tests.rs:57,85` 的 E0509；按新授权复核时源码已有最小修复，停止自己的旧快照检查并重跑 |
| 聚焦命令（复用 ACP 修复后的增量构建） | 130 / 0 | 运行约 55 分钟仍在单态化/MIR；采样后系统内存压力为 2，按用户授权停止自己的资源受限构建，保留缓存 |
| 自动排队的旧环境 Clippy | 130 / 不执行测试 | 仅启动短暂依赖检查；停止自己的进程树，改用用户指定的新环境 |
| 同一聚焦命令（关闭增量、单作业） | 退出码不可恢复 / 0 项有效证据 | 19:52:56+0800 启动；会话管理接口中断后，20:58 核实 Cargo/rustc 均已消失，工具会话失效，日志未到用例结果 |
| 完整 Clippy（上述中断轮次） | 未执行 | 聚焦验证未完成，未进入后续命令 |
| 聚焦命令（低内存环境） | 101 / 2 | 22:50:59 结束；1 通过、1 失败：无效模型预期 404，实际 503，源于临时凭据根目录权限 |
| 完整 Clippy（低内存环境） | 未执行 | 测试失败后该轮脚本退出 |
| 槽位 7 已生成二进制，`umask 077` 后同一过滤条件 | 0 / 2 | 两项通过，1.55 秒；确认夹具目录隐式权限是失败原因，不能替代修改后的 Cargo 重跑 |
| 聚焦命令（显式私有目录修复后，槽位 2） | 101 / 0 | 无关 providers/operations.rs:23 初始化了当时类型快照不存在的 aggregation_outcome 字段，E0560，未到用例 |
| 完整 Clippy（修复后，槽位 2） | 0 / 不执行测试 | 23:09:03 起，12 分 46 秒完成全部 targets/features，含本任务测试源码 |
| 聚焦命令（短暂领取槽位 3） | 143 / 0 | 上轮槽位 2 被其他任务抢先领取；及时发现冷缓存后仅停止自己的启动进程，复用空闲旧槽位 7 |
| 聚焦命令（复用槽位 7） | 0 / 2 | 11 月 12 日 00:17:28 前结束；2 通过、0 失败、12303 filtered，行为执行 2.05 秒，构建 53 分 42 秒 |
| 重复完整 Clippy（槽位 7） | 143 / 不执行测试 | 按用户指示核对并复用槽位 2 已通过记录，仅停止自己的重复进程；不计为新的通过证据 |

独立测试文件 `agent_apps_diagnostics_tests.rs` 的两项过滤用例已验证跨项目/主机/应用拒绝、无效成员、模型不存在、额外字段、MCP 批次中夹带未授权 stdio 配置、202 身份、五类结果、128 项活动任务后的 429，以及真实网关子进程入口。本地真实 HTTP/SSE/MCP 服务、数据库、任务快照与文件哨兵属于受控夹具，不代表真实供应商或账号验收。显式修复夹具后的源码已通过规定 Cargo 聚焦命令，两项均通过。

当前实现和夹具均取得行为绿灯，完整 Clippy 退出 0。历史有效行为红灯未取得：早期构建失败和后续夹具权限错误均不属于原功能缺失的断言失败，未将其包装为红灯。按用户最新补充要求，保留“已完成，待回归”任务卡，尚待补充真实前置基线回放的行为失败证据；只提交本任务夹具修复及本证据文件，保留其他任务的全部工作区和暂存改动。

## ACP 验证编译修复的归属

用户新增授权允许保留两处验证必需修复。复核 `crates/vw-agent/src/session/llm/acp/model_source/drivers/drivers_tests.rs` 第 57、85 行时，两处已由其他工作区改动写成 `.model_api.clone().unwrap()`，因此直接复用，没有重复修改、撤回或扩展 ACP 行为。补丁仅在调用 `unwrap` 前克隆 `Option<ApiInfo>`，避免从实现 `Drop` 的 `AcpLaunchProfile` 移出字段。该文件原本为其他任务未跟踪文件，不纳入诊断提交；留在工作区供整个计划持续验证。

## 编译耗时采样

2026-11-11 19:40:29，对自己的 rustc PID 40089 运行 `sample 40089 1 1 -file /tmp/shiju37-rustc.sample`，退出 0。当时运行约 44 分钟、累计 CPU 8 分 34 秒；采样 physical footprint / peak 为 9.5 GB。703 个编译线程样本位于 `collect_and_partition_mono_items → items_of_instance → optimized_mir → GVN → LayoutData` 哈希/布局分配，没有观察到增量依赖图保存栈。CPU 时间持续增加，按用户指示保留正常推进的当前构建。后续验证使用 `CARGO_INCREMENTAL=0` 与 `CARGO_BUILD_JOBS=1`，继续使用原槽位及绝对构建路径，不清理缓存。

19:51 左右进一步读取 `sysctl kern.memorystatus_vm_pressure_level` 为 2；`vm_stat` 的逻辑压缩页为 3,154,638 × 16 KiB（约 48 GiB），占用压缩区约 24.5 GiB。自己的 rustc 驻留约 1.5 GiB，先前 footprint 9.5 GB，构建累计约 55 分钟仍无测试执行。依据用户允许的资源压力分支，仅停止父链属于本任务的 Cargo/rustc（以及原脚本自动排队的 Clippy）；各命令中断码如上。保留槽位 7 缓存，19:52:56 起以 `CARGO_INCREMENTAL=0`、`CARGO_BUILD_JOBS=1` 重跑，未改仓库构建配置。

用户随后允许降低测试调试信息，但要求不为设置反复重启。核实此时关闭增量/单作业构建已在约两分钟内推进至 vw-acp，故保留本轮；`CARGO_PROFILE_TEST_DEBUG` 与 `CARGO_PROFILE_DEV_DEBUG` 均未设置，沿用 Cargo.toml 中 `line-tables-only`。若后续测试因实际失败需要重跑，再使用 `CARGO_PROFILE_TEST_DEBUG=0`（必要时 dev 同为 0），仅限命令环境，不写仓库配置。

对照任务 33 当前验证记录，成功命令除 `CARGO_INCREMENTAL=0` 外还使用 `CARGO_PROFILE_TEST_CODEGEN_UNITS=256`。本轮该变量未设置；实际 rustc 参数为 opt-level=1、debuginfo=line-tables-only，未显式带 codegen-units 或 incremental。当前进程仍计算推进，遵循不反复重启的指示保留；若后续确需重跑测试，将采用已有通过记录的 codegen-units=256，并结合用户允许的 debug=0。

## 会话中断后的接续

20:58 核实上一轮控制 shell/Cargo/rustc 已全部消失、旧工具会话不可恢复，日志没有用例结果，故不猜测退出码或通过数。旧 app-server PID 54011 已结束，仅校验并释放本任务旧 slot-7.lock；保留其全部缓存，未动其他任务锁。重新从 1–20 原子领取，再次获得槽位 7，持有 PID 48904。格式化退出 0，21:00:46+0800 启动聚焦测试。本轮实际环境：

```text
CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-7
CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-7
CARGO_INCREMENTAL=0
CARGO_BUILD_JOBS=1
CARGO_PROFILE_TEST_DEBUG=0
CARGO_PROFILE_DEV_DEBUG=0
CARGO_PROFILE_TEST_CODEGEN_UNITS=256
```

上述设置仅作用于本轮命令环境，仓库配置未修改。重跑时系统内存压力已恢复为 1。

22:10 对新环境的自身 rustc PID 60016 再做一次 1 秒阶段采样（退出 0），用于确认长编译是否有阶段进展。实际已进入 LLVM 代码生成，栈包含 LLVM 属性/指令构造与 rustc_codegen_llvm，区别于旧轮 MIR/GVN 布局哈希；未见增量图保存栈。footprint 15.3 GB、peak 16.9 GB。保留此正常推进的进程。实际参数已核实含 opt-level=1、codegen-units=256、strip=debuginfo，无 incremental，out-dir 为槽位 7/debug/deps。

## 夹具权限定位与修复

22:50:59 取得首次完整测试输出：两项执行，真实网关用例通过，主用例在无效模型处因 503 != 404 失败。未更改生产错误映射。读取 CredentialFileStore 的真实检查后，发现 tempfile 默认目录权限受 umask 影响，夹具将其直接作为 credential_root，却未显式设置 0700。用 `umask 077` 执行槽位 7 的原测试二进制：`target/rust-slot-7/debug/deps/vw_agent-3eba7dd0dd6325e4 diagnostic_api_start_authorizes_before_execution --nocapture`，退出 0，2 项通过、0 失败、12267 filtered，耗时 1.55 秒。该结果定位夹具，不冒充原 Cargo 命令已通过。

仅在独立测试的临时根创建后增加显式 0700，不弱化底层凭据安全检查。首次失败是夹具错误，仍不是原行为缺失的红灯证据。编译还产生 macOS 链接器 __eh_frame 超过 16MB 的警告；未改全局编译配置，也未屏蔽。

确认本阶段 Cargo、rustc 和测试子进程全部结束后释放自己的槽位 7 锁，保留缓存。22:53:23 重新从 1–20 顺次原子领取到槽位 2，持有 PID 28159。新阶段：

```text
CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2
CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2
CARGO_INCREMENTAL=0
CARGO_BUILD_JOBS=1
CARGO_PROFILE_TEST_DEBUG=0
CARGO_PROFILE_DEV_DEBUG=0
CARGO_PROFILE_TEST_CODEGEN_UNITS=256
```

`cargo fmt --all -- --emit stdout --config skip_children=true` 退出 0；测试文件仅新增一行私有目录设置，局部 rustfmt 与 diff --check 均通过。随后执行完整相同聚焦命令，再执行规定全量特性 Clippy；该阶段结果已补入上表。

## 无关字段快照与完整 Clippy

槽位 2 聚焦命令在 vw-agent-apps 编译阶段退出 101、0 项测试：providers/operations.rs:23 的 aggregation_outcome 与先编译的 ProviderOperationProgress 快照不一致（E0560）。复核当前 DTO 已由其他任务补上字段，本任务未触碰。随后规定完整 Clippy 退出 0，耗时 12 分 46 秒；包含当前私有目录夹具修复，未屏蔽警告。

用户新增规则允许优先原子复用本会话先前领取的槽位，忙时再遍历其余槽位。阶段结束释放槽位 2 后重领时，它已被聚合任务 41 占用；短暂领取到冷缓存槽位 3。发现本会话旧槽位 7 空闲后原子取得其锁，仅停止自己的槽位 3 shell/Cargo/rustc（退出 143，0 测试），核实消失后释放自己的槽位 3 锁。没有动其他所有者锁或进程。23:23:44 起复用槽位 7 的已生成缓存重新运行同一聚焦命令，之后顺次完整 Clippy，保持该完整阶段不释放；实际两个绝对目录恢复为 /Users/xiangmin/worker/code/shiju/target/rust-slot-7，其他低内存环境变量不变。

## 最终验收与资源清理

最终聚焦命令为 `cargo test -p vw-agent --lib diagnostic_api_start_authorizes_before_execution -- --nocapture`，在默认 features 下退出 **0**，实际 **2 passed / 0 failed / 12303 filtered out**，行为执行 2.05 秒；包括真实 gateway 子进程入口。五类受理均得到 202 身份及成功结果；越权请求 403、无效成员 401、额外字段 400、未知模型 404、容量上限 429。受控 HTTP/SSE/MCP 上游计数符合断言，未授权 stdio 哨兵未产生；网关子进程由隔离夹具退出时 kill/wait 清理。证据来自本地真实协议夹具，不宣称真实供应商或人工实机验收。

规定完整命令 `cargo clippy --all-targets --all-features -- -D warnings` 在槽位 2 退出 **0**，实际包含 22:53:00 写入的 0700 修复。最终复核启动处理器、独立测试、授权入口、生命周期注入、DTO、DiagnosticJobs、MCP 启动入口和五类执行器的文件修改时间均早于这次 Clippy；其后相关路径无新提交。按用户明确指示复用该完整通过证据，未再次等待重复检查。槽位 7 的重复 Clippy 仅终止本任务父链下的 PID 22270/22273/23600，实际退出 **143**，不计为通过。

槽位 7 脚本随后删除自己的 owner 并 rmdir；复核 PID 81477/81480/82533/22270/22273/23600 均已消失，slot-7.lock 已释放。此前自己的槽位 2、3 锁亦已在各阶段结束后释放；保留全部编译缓存，不处理其他任务新占用的锁。最终行为验证目录为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-7`；完整 Clippy 通过目录为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-2`，两阶段的 CARGO_BUILD_BUILD_DIR 均与各自 CARGO_TARGET_DIR 相同。

现有限制：没有有效历史行为红灯；macOS 链接器报告 __eh_frame 超过 16MB 的既有体量警告（测试仍正常执行通过），未屏蔽。建议下步在本会话取得受保护的前置基线回放窗口，补齐真实调用链的失败断言，再删除任务卡。

## 剩余项：有效行为失败回放

**状态：已完成，待回归。** 当前两项绿灯及完整 Clippy 均有效，但不能免除任务卡要求的行为失败证据。最终收尾时仍有 15 个其他 Cargo/编译器进程活动，且本任务此前提交涉及授权文件、父模块、bootstrap 与共享生命周期注入。本轮未在这些活动构建期间临时倒退共享源码；现有二进制又已链接当前实现，不能通过换夹具、故意改请求或伪造返回值冒充前置基线。未启动新的回放编译，未删除任务卡。

后续在本会话取得不会影响其他任务的源码回放窗口后，仅回放本任务可精确恢复的变更块、保留其他工作区内容；由真实启动调用链产生对应缺失行为的断言失败，并明确标注“基线回放证据”，不能声称是历史 TDD 红灯。恢复当前实现后补充必要聚焦回归，再清理任务卡。只读 plan.md 继续不动。
