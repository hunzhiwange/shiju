# CodeBuddy 模型来源适配证据

状态：正式 Cargo 双来源回归通过。两种来源各 1 项测试通过、Cargo 退出 0、各 13 次真实模型请求。仅验证 Darwin arm64、Node v26.13.0；早期缓存二进制证据保留为历史记录。

固定官方包 `@tencent-ai/codebuddy-code@2.179.3`，安装于 target/acp-codebuddy-cli，实际 --version 为 2.179.3。独立 [manifest](codebuddy/manifest.json) 保留发行包 integrity、启动命令及原生离线配方；公共 manifest 未改动。模板仅含 fixture 占位符，未写真实用户配置或使用生产模型账号。

## 驱动行为

`drivers/codebuddy.rs::configure` 仅接受默认 --acp 参数，拒绝额外模型、设置、恢复参数。CODEBUDDY_CONFIG_DIR 和 WORKBUDDY_CONFIG_DIR 指向 profile 0700 私有根；Chat BASE_URL 为中继 /v1，API_KEY 为短期绑定令牌，仅在环境传递，不进入 argv。

主模型、lite、reasoning、code subagent 均映射当前真实模型。CODEBUDDY_CODE_MAX_OUTPUT_TOKENS 传递当前输出限额（0 使用共享默认值）；固定上游最多向下截断到 100,000。Chat 会携带 temperature，不支持时在启动前明确报错。

官方源码 resolveModelBaseURL 优先取模型文件 URL，因此关闭 CODEBUDDY_DISABLE_CUSTOM_MODELS_FILE，并追加 --setting-sources none，防止 models.json、settings env、apiKeyHelper 抢路由。AUTH_TOKEN 为空、apiKeyHelper 禁用、API_KEY_DISABLED 为空；后者字符串 "0" 在上游仍视为禁用，已用真实 CLI 分别复现认证失败及空值后成功。cli_local 沿用共享运行时旁路，不调用驱动或注入客户端覆盖。

独立真实上游探测验证 BASE_URL=/v1、models.json 完整 URL=/v1/chat/completions、无需登录的 API_KEY 路线、用户/项目冲突配置禁用；每轮 4 次认证请求及两次读文件工具结果。这些探测与下面生产链证据分开。

参考：[官方模型文件](https://www.codebuddy.ai/docs/cli/models)、[环境变量](https://www.codebuddy.ai/docs/cli/env-vars)，另核对固定 npm 包源码。

## 底层 boolean 协议修复

真实 session/new 响应包含 multitask 的 type=boolean 配置项。原 ACP schema 未启用对应功能，真实响应反序列化报 unknown variant boolean, expected select；这发生在生成请求之前，不能归因于认证或历史目录。

经协调独占七文件窗口完成共享修复并提交 **21fe2c560**：启用现有 unstable_boolean_config，不升级依赖；字符串选择显式使用 SessionConfigValueId，模型配置收到布尔值时返回 invalid_params。普通 select、未知 toggle、模型拒绝行为均有聚焦证据；未修改 fixture、转换器或 Claude 原有协议测试。

- 红灯：槽位 1，cargo test -p vw-acp --lib acp_model_source_codebuddy_boolean_session_options -- --nocapture，退出 101，命中具名测试并因上述真实解码错误失败（随后 panic/SIGABRT，非通过）。
- 绿灯：槽位 2，同命令退出 0，1 passed；model_selection_protocol_updates_prompt_and_load_state 退出 0，1 passed；client::session_control_tests 退出 0，7 passed。格式化退出 0。
- 全仓 cargo clippy --all-targets --all-features -- -D warnings：槽位 6，boolean 修复和本驱动当前代码均在场，退出 0。该结果保留为历史检查记录；本轮无 Rust 变更，不重复执行已通过的驱动聚焦和 Clippy。

七文件窗口已释放，当时 Claude 原有 protocol_tests.rs 41 行测试未混入共享修复提交。

## 驱动聚焦与真实生产二进制

第一轮红灯命中 acp_model_source_codebuddy_child_routes_primary_and_auxiliary_without_argv_secrets，原 stub has not been verified，Cargo 101（槽位 10）。第一轮失败记录不算成功证据。

当前驱动具名测试：

- acp_model_source_codebuddy_child_routes_primary_and_auxiliary_without_argv_secrets
- acp_model_source_codebuddy_conflicting_routes_and_resume_fail_closed
- acp_model_source_codebuddy_rejects_unsupported_temperature_before_launch

修复 boolean 后，槽位 2 执行 cargo test -p vw-agent --lib --all-features acp_model_source_codebuddy -- --nocapture，退出 0，3 passed/0 failed（target/codebuddy-post-schema-focused.log）。同一测试二进制为 target/rust-slot-2/debug/deps/vw_agent-2af45fce6a9d7cfa。

复用刚通过聚焦测试的同一生产二进制，运行任务 10 原有 Rust 验收入口、fixture 和通过判定，启动真实 CodeBuddy 子进程。[辅助脚本](codebuddy/run_cached.py) 仅替换 Cargo 启动方式；报告明确记录 evidence_kind=cached-production-binary、真实 command、binary_exit_code。

| 来源 | 有效测试 | 二进制退出码 | 生成请求 | 实际模型与工具结果 |
| --- | --- | --- | --- | --- |
| [session-model](codebuddy/session-model-cached.json) | 1 | 0 | 13 | primary、switched；工具结果最大 2 |
| [cli-local](codebuddy/cli-local-cached.json) | 1 | 0 | 13 | 始终 primary；工具结果最大 2 |

两份均 passed=true、认证正确、/v1/chat/completions、fixture 错误 0；新建、第二轮、取消、重启、切换、隔离、握手均 true。session-model 原生配方故意放置错误模型和凭据，实际请求未采用。未复现历史目录恢复问题，无需修改历史目录边界。

主/辅助环境入口已核对固定源码并经子进程断言；不把字段断言描述为真实执行了全部辅助 Agent 分支。延后的正式 Cargo 入口现已完成，见下节。

## 正式 Cargo 回归（2026-11-04）

基于实现阶段结束后的共享工作区运行原 scripts/verify_acp_model_source.py，未使用 run_cached.py、未替换 Cargo 启动。两份报告均 evidence_kind=real-cli，command 为实际 cargo test 命令，记录 cargo_exit_code=0。

| 来源 | 有效测试 | Cargo 退出码 | 生成请求 | 实际模型 |
| --- | --- | --- | --- | --- |
| [session-model](codebuddy/session-model-formal.json) | 1 | 0 | 13 | primary、switched |
| [cli-local](codebuddy/cli-local-formal.json) | 1 | 0 | 13 | 始终 primary |

两种来源认证及 /v1/chat/completions 路由正确，fixture 错误 0，真实工具结果达到 2；新建、第二轮、取消、重启、切换、隔离、握手全部 true。固定真实 CLI 2.179.3 的版本检查由原验收脚本重新执行；不据此声明其他平台通过。

为避免首次编译超过验收器时限，先执行 cargo test -p vw-agent --lib --all-features acp_model_source_real_cli --no-run，退出 0；该步骤只准备测试目标，不计作测试通过。预编译出现测试大二进制的 linker __eh_frame compact unwind 警告，无 Rust 源码变更，未为此修改共享构建配置。正式两入口随后各命中 1 个具名测试。

从仓库根按任务规则动态领取槽位，设置 CARGO_TARGET_DIR 和 CARGO_BUILD_BUILD_DIR 为其绝对路径，设置 RUST_MIN_STACK=33554432，PATH 前置 target/acp-codebuddy-cli/node_modules/.bin，执行：

```bash
python3 scripts/verify_acp_model_source.py --agent codebuddy --mode session-model --manifest docs/vibewindow/acp-model-source/evidence/codebuddy/manifest.json --output target/acp-model-source-evidence/codebuddy/session-model-formal --offline-fixture
python3 scripts/verify_acp_model_source.py --agent codebuddy --mode cli-local --manifest docs/vibewindow/acp-model-source/evidence/codebuddy/manifest.json --output target/acp-model-source-evidence/codebuddy/cli-local-formal --offline-fixture
```

两个实际 Cargo 入口为：

```bash
cargo test -p vw-agent --lib --all-features acp_model_source_real_cli_session_model -- --ignored --nocapture --test-threads=1
cargo test -p vw-agent --lib --all-features acp_model_source_real_cli_cli_local -- --ignored --nocapture --test-threads=1
```

正式回归动态领取 **槽位 4**：CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-4，CARGO_BUILD_BUILD_DIR 同路径；所有相关 Cargo 和 CLI 子进程退出后自己的锁已释放，编译缓存保留。阶段日志 target/codebuddy-formal-stage.log、预编译日志 target/codebuddy-formal-build.log。

此前驱动与缓存生产二进制验证使用槽位 2（/Users/xiangmin/worker/code/shiju/target/rust-slot-2），共享修复 Clippy 使用槽位 6（/Users/xiangmin/worker/code/shiju/target/rust-slot-6），红灯使用槽位 1、10，中断阶段使用过槽位 5；均为动态领取。

公共 manifest 未改；本独立 manifest 已用于正式回归。修复前失败、被中断构建和早期 cached-production-binary 结果均未冒充本次正式通过。编号任务文件可按完成契约删除，总计划 plan.md 保持只读。
