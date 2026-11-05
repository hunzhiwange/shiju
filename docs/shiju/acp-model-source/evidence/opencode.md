# OpenCode 模型来源验收

平台：macOS 27.0（26A428），Darwin-arm64；日期：2026-11-05。Linux/Windows 未运行，不宣称跨平台通过；Windows 仍需运行时私有目录 ACL 能力。

## 固定真实发行包

- 官方 [opencode-ai 1.19.5](https://registry.npmjs.org/opencode-ai/1.19.5) 的 Darwin-arm64 原生依赖：[元数据](https://registry.npmjs.org/opencode-darwin-arm64/1.19.5)、[原生包](https://registry.npmjs.org/opencode-darwin-arm64/-/opencode-darwin-arm64-1.19.5.tgz)。未使用其它 Agent 或修改 OpenCode 二进制。
- 原生包 SHA-512 integrity：`sha512-0MbSbKkmwVHtgP31GJyyECe6QVy4T7W4Wp7s2TsmXHTPPyf01ne3dWSX97X9dkNK28Lc6kt+zMPQUP/9GgDLZg==`。下载完成后逐字节校验相符，随后解包。
- 解包后 `bin/opencode` SHA-256：`feaafead8cd563af0c2dcb386b0757e4355e327ebb6bd1aaad5440fe2f83596c`；实际 `--version` 返回 `1.19.5`。专属验证入口同时检查摘要和版本，拒绝假 CLI 或其它平台二进制。
- 发行包仅位于 `target/task27-opencode`，未安装到用户全局目录。

## 实现与源码契约

`crates/vw-agent/src/session/llm/acp/model_source/drivers/opencode.rs::configure` 为 session_model 生成专属内联 Provider。`model`、`small_model` 与 build/plan/general/explore/title/summary/compaction 的模型均绑定到 `session-provider/<selected-model>`，`enabled_providers` 仅允许该 Provider，其 `whitelist` 仅允许当前所选模型。令牌使用 `{env:VW_OPENCODE_RELAY_TOKEN}` 引用，短期中继令牌只进入环境，既不进入 argv，也不持久化到原生配置。

配置语义核对固定标签源码：[配置合并顺序](https://github.com/anomalyco/opencode/blob/v1.19.5/packages/opencode/src/config/config.ts)、[配置目录发现](https://github.com/anomalyco/opencode/blob/v1.19.5/packages/opencode/src/config/paths.ts)、[XDG 与独立 home 查找](https://github.com/anomalyco/opencode/blob/v1.19.5/packages/core/src/global.ts)、[Provider schema](https://github.com/anomalyco/opencode/blob/v1.19.5/packages/core/src/v1/config/provider.ts)、[变量展开](https://github.com/anomalyco/opencode/blob/v1.19.5/packages/opencode/src/config/variable.ts)。模型标识必须是字面值，不能触发配置中的环境或文件引用展开。

使用独立 XDG config/data/state/cache、OPENCODE_CONFIG_DIR 与 OPENCODE_TEST_HOME；保留进程 HOME 供正常工具使用，不通过改变 HOME 扩大或模拟权限。关闭项目配置发现，隔离 home 侧 Agent/frontmatter，禁止原生账号接管模型。持久历史按租户、项目、会话及来源摘要隔离；配置目录与令牌每轮重建，旧 profile 丢弃即撤销旧令牌。未改变工具权限。

系统托管目录与 macOS 的系统/当前 OS 用户 MDM plist 优先级高于内联配置；检测到相应配置或继承的托管目录覆盖时明确拒绝托管来源，不绕过系统策略，也不加载或记录策略内容。该行为按[固定版本托管配置源码](https://github.com/anomalyco/opencode/blob/v1.19.5/packages/opencode/src/config/managed.ts)实现。

显式拒绝额外 ACP 参数、显式 OpenCode/XDG/代理环境冲突、无效中继地址、缺失或不支持的模型协议，以及模型标识中的配置展开。缺失上游凭据在真实中继 HTTP 边界返回 401，禁用模型在发起原生进程前被拒绝。cli_local 沿用原始 command/args/env，不进入该 driver；客户端切模不会覆盖原生配置。

按原生 SDK 配置 Chat（`@ai-sdk/openai-compatible`）、Responses（`@ai-sdk/openai`）、Messages（`@ai-sdk/anthropic`）、Gemini（`@ai-sdk/google`）。本任务真实请求验收仅覆盖 Chat Completions，其他协议没有单独的真实 CLI 请求证据。

## 实际请求证据

[session-model.json](opencode/session-model.json) 和 [cli-local.json](opencode/cli-local.json) 来自任务 10 的真实生产 ACP 验收入口，不以配置字段或握手代替模型请求：

| 来源 | 有效生产验收测试 | 实际请求 | 真实子代理请求 | 错误 |
| --- | ---: | ---: | ---: | ---: |
| session_model | 1 | 15 | 6 | 0 |
| cli_local | 1 | 10 | 3 | 0 |

所有请求均通过鉴权且路径为 `/v1/chat/completions`。session_model 实际收到 `acp-fixture-primary` 和 `acp-fixture-switched`；cli_local 始终收到 `acp-fixture-primary`。两项均有包含两个真实文件读取结果的请求；new、second_turn、cancel、restart、switch、isolated、handshake 全为 true，Cargo 退出码均为 0。

专属 [verify.py](../../../../patches/acp-model-source/opencode/verify.py) 与 [recipe.json](../../../../patches/acp-model-source/opencode/recipe.json) 补充本 CLI 的原生离线配置和完整 XDG 沙箱，调用未修改的共享 `scripts/verify_acp_model_source.py`。只模拟外部模型服务；fixture 返回任务调用，由真实 OpenCode `task` 子代理完成读取，并要求观测到子代理 HTTP 请求。未替换仓库分派、中继、会话持久化或 CLI 逻辑。session_model 另有冲突的项目 JSON、主/辅助/compaction 模型与 Agent frontmatter。

macOS 沙箱路径先解析为真实路径：初次使用 `/var` 别名时，原生子代理把它与实际 cwd `/private/var` 判断为外部目录，停在上游权限等待，未完成验收；规范化沙箱后通过，没有放宽任何权限。失败记录保留在 target，不计作通过证据。

## 验证记录

实际动态领取过的槽位（重跑必须重新领取，不能固定占用以下编号）：

- 初始有效红灯：槽位 3，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-3`。
- 通过验证与真实 CLI：槽位 2，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。
- 同时设置 `CARGO_BUILD_BUILD_DIR=$CARGO_TARGET_DIR`，覆盖仓库的 `[build].build-dir`；每阶段结束释放自己的锁，保留缓存。
- 聚焦测试使用 `RUST_MIN_STACK=33554432`，与共享真实验收入口一致；默认测试线程栈曾在缺失凭据用例溢出，不计通过。

```sh
cargo fmt --all
export RUST_MIN_STACK=33554432
cargo test -p vw-agent --lib --all-features acp_model_source_opencode -- --nocapture
python3 patches/acp-model-source/opencode/verify.py --cli target/task27-opencode/package/bin/opencode --mode session-model --output target/task27-opencode/aux-session-model.json
python3 patches/acp-model-source/opencode/verify.py --cli target/task27-opencode/package/bin/opencode --mode cli-local --output target/task27-opencode/aux-cli-local.json
cargo clippy --all-targets --all-features -- -D warnings
```

有效初始红灯：`acp_model_source_opencode_child_reads_isolated_primary_and_auxiliary_routes` 命中 1 项，因原 driver 返回 `has not been verified` 失败，退出 101。模型引用红灯：`acp_model_source_opencode_model_references_do_not_expand_native_configuration` 命中 1 项，因原实现接受配置展开而失败，退出 101；没有运行 CLI 或读取引用内容。

最终验证：`cargo fmt --all` 退出 0；聚焦 **7 项通过、0 失败**，退出 0（`target/task27-opencode/green-final.log`）；两种来源真实验收各 1 项，退出 0；全仓 Clippy 退出 0（`target/task27-opencode/clippy-final.log`）。最后一轮槽位记录为 `target/task27-opencode/finish-checks.log`，所有本任务锁均已释放。聚焦用例：

```text
acp_model_source_opencode_child_reads_isolated_primary_and_auxiliary_routes
acp_model_source_opencode_conflicts_fail_before_creating_runtime_state
acp_model_source_opencode_missing_transport_and_unowned_endpoints_fail_closed
acp_model_source_opencode_rotation_keeps_history_and_revokes_old_credentials
acp_model_source_opencode_missing_credentials_and_disabled_model_never_use_native_account
acp_model_source_opencode_model_references_do_not_expand_native_configuration
acp_model_source_opencode_managed_configuration_is_rejected_without_loading_it
```

`ld: __eh_frame section too large` 链接器警告在初始未实现 driver 的红灯阶段已存在；不修改无关链接配置。公共 manifest、registry、安装器和能力发布未修改，任务 33 应集成本任务配方与证据。
