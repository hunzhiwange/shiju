# Kimi Code 模型来源验收

2026-11-05，Darwin-arm64。只验证本平台，Linux/Windows 尚未运行验收；Windows 仍受运行时私有目录 ACL 校验限制。

## 发行包与原生入口

使用官方 Kimi Code 后继发行包 **2.5.0**，不是其它 Agent，也没有修改厂商权限校验。

- [官方版本 manifest](https://code.kimi.com/kimi-code/binaries/2.5.0/manifest.json)，tag `@moonshot-ai/kimi-code@2.5.0`。
- [官方原生二进制](https://code.kimi.com/kimi-code/binaries/2.5.0/kimi-code-darwin-arm64)，SHA-256 `d4dd02d40f18f3a784699fc37ae0c55d676a4704c37a3b91ab96541b0db673a1`，下载后逐字节摘要与官方 manifest 相符。
- [npm 固定发行包](https://registry.npmjs.org/@moonshot-ai/kimi-code/2.5.0)，MIT；integrity 和六平台原生摘要保存在公共 cli-versions.json 的 kimi 项。安装映射已改为 `npm install -g @moonshot-ai/kimi-code@2.5.0`，启动保持 `kimi acp`。
- 原 `kimi-cli==1.52.0` 是弃用门禁发行版：`--version` 虽报告版本，但 `acp`/`--acp` 只输出迁移提示并退出0。其保留源码不等于可运行入口，本任务不绕过门禁调用内部函数。

2.5.0 真实 `initialize`、`session/new` 在独立 Provider 配置下无需 Kimi OAuth 即成功；证明不是版本探针。配置语义按[官方配置](https://moonshotai.github.io/kimi-code/en/configuration/config-files)和[Provider 文档](https://moonshotai.github.io/kimi-code/en/configuration/providers)以及2.5.0已发布 npm 的 dist/main.mjs 核对。

## 行为与边界

`drivers/kimi.rs::configure` 为托管来源生成 `KIMI_CODE_HOME/config.toml`。Provider 类型按所选协议映射到 `openai`、`openai_responses`、`anthropic`、`google-genai`，显式使用 `api_key_env = "VW_KIMI_RELAY_TOKEN"`；中继令牌仅进入进程环境，不写配置或 argv。本轮真实模型请求验证的是 Chat Completions，其他原生协议未单独做真实模型请求验收。

目录按租户、项目和会话摘要隔离，0700；配置以0600临时文件原子发布。原生历史保存在专属状态目录，稳定 `session-selected` 别名允许恢复后使用新模型与新中继；持久配置只有环境变量名和失效后的 loopback 路由，不含凭据。每次 profile 仍独立签发/撤销短期令牌。不同来源使用既有来源隔离逻辑，cli_local 不进入驱动，不改原生 command/args/env。

拒绝额外 ACP flags、显式模型/账户环境冲突、非本机中继、缺失/未知协议；屏蔽继承的临时模型和自定义认证 header 覆盖，禁用遥测和自更新，不改变工具权限。测试目录均为临时目录或 target，不写用户配置。

## 有效证据

- [session-model.json](kimi/session-model.json)：11次认证成功的 `/v1/chat/completions` 请求，实际模型为 `acp-fixture-primary`、`acp-fixture-switched`；3次原生 coder 子代理模型请求，工具结果达到2。故意冲突的原生配置没有接管托管模型。
- [cli-local.json](kimi/cli-local.json)：9次认证成功请求，含3次 coder 子代理请求，模型始终为 `acp-fixture-primary`，客户端切模没有覆盖原生模型。
- 两项各命中1个生产边界验收测试，Cargo退出0；new、second_turn、cancel、restart、switch、isolated、handshake全部true。仅模拟外部模型服务，CLI、进程启动、仓库路由、中继、会话持久化、原生工具与子代理均实际运行。
- [native-boundaries.json](kimi/native-boundaries.json)：校验官方原生二进制摘要后，缺失显式令牌（同时提供冲突的普通 OPENAI/KIMI_API_KEY）与未知模型均被真实CLI拒绝，ACP错误-32000、模型请求0。上游返回通用 Authentication required，不宣称错误携带变量名。
- `verify_native_boundaries.py` 可重现负向探针；不调用无参数kimi、不执行迁移安装器、不读取用户账号、不发付费请求。

## 验证命令

从仓库根目录按任务槽位协议动态领取 **槽位3**，本轮所有Cargo/验收脚本继承：

```sh
export CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-3
export CARGO_BUILD_BUILD_DIR="$CARGO_TARGET_DIR"
export PATH="$PWD/target/kimi-code-2.5.0:$PATH"
cargo fmt --all
cargo test -p vw-agent --lib --all-features acp_model_source_kimi -- --nocapture
cargo test -p vw-config-types --lib acp_install -- --nocapture
python3 -m unittest discover -s scripts -p acp_model_source_fixture_tests.py
python3 scripts/verify_acp_model_source.py --agent kimi --mode session-model --manifest docs/vibewindow/acp-model-source/cli-versions.json --output target/acp-model-source-evidence/kimi/session-model --offline-fixture
python3 scripts/verify_acp_model_source.py --agent kimi --mode cli-local --manifest docs/vibewindow/acp-model-source/cli-versions.json --output target/acp-model-source-evidence/kimi/cli-local --offline-fixture
python3 docs/vibewindow/acp-model-source/evidence/kimi/verify_native_boundaries.py target/kimi-code-2.5.0/kimi
cargo clippy --all-targets --all-features -- -D warnings
```

初始红灯：聚焦3测试命中，2失败/1通过，退出101；两项正向行为因原驱动 `has not been verified` 明确失败。早期无关编译错误不计为红灯。

最终有效验证：

- `cargo fmt --all`退出0。
- Kimi聚焦 **4项通过、0失败**，退出0；日志`target/kimi-green-verified.log`，包含缺失凭据/禁用模型的真实HTTP边界。
- 安装契约 **2项通过**，退出0；日志`target/kimi-installer-tests.log`。
- fixture **25项通过**，退出0；日志`target/kimi-fixture-tests.log`。
- 两种真实CLI生产验收各 **1项通过**，退出0；独立真实CLI负向探针退出0，原始脱敏证据见上文。
- `cargo clippy --all-targets --all-features -- -D warnings` **退出0**；日志`target/kimi-clippy-verified.log`。

手工检查初期发现仓库另有build-dir配置，已同时固定两个Cargo目录并重跑上述最终检查；真实CLI脚本始终同时设置两个路径。期间的无关DelegateCapabilityPolicy/unused_mut编译错误由其它工作修复，本任务没有修改或提交那些文件。

槽位阶段结束释放自己的slot-3锁并保留缓存。公共能力发布由任务33集成，本任务不声明22项或其它平台全量支持。
