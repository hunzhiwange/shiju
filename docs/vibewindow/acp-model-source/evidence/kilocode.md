# Kilocode 模型来源验收（2026-11-05）

状态：已完成，待回归。Kilocode 全部专属验收已通过；当前全仓 Clippy 被无关桌面死代码阻断。

固定版本：真实 `@kilocode/cli@7.8.18`，macOS 27.0 / arm64，Node v26.13.0。
未验证 Linux/Windows，不从 npm 平台包推断通过。

## 实现

`drivers/kilocode.rs::configure` 使用原生 `kilocode acp --pure`，不改稳定注册 key，
不使用 `OPENCODE_*` 配置。npm 同时导出 `kilo` / `kilocode`；正式安装来自
`https://registry.npmjs.org/@kilocode/cli/-/cli-7.8.18.tgz`，包完整性与调查 manifest
的 SHA-512 一致，真实 `--version` 返回 `7.8.18`，无上游二进制补丁。

- 原生 Provider 使用 `@ai-sdk/openai-compatible` 和 `/v1/chat/completions`；
  按会话派生独立 Provider 名，避免与常见本地 Provider 配置合并。
  主模型、small/subagent/memory 模型绑定同一原生别名，实际模型 ID 随所选模型更新。
  其他上游协议使用已有生产中继的显式转换边界，本证据只验收 Chat 上游。
- 配置同时通过 `KILO_CONFIG` / `KILO_CONFIG_CONTENT` 注入；只允许托管 Provider，
  短期令牌通过 `{env:VW_KILO_RELAY_TOKEN}` 引用，不写入配置文件或 argv。
  配置文件 0600，临时根及持久历史边界 0700；profile 释放撤销令牌并删除临时配置。
- XDG 配置、缓存使用本次私有目录；数据和历史按租户/项目/会话隔离，
  同来源重启保留状态，切模保持原生别名、更新真实模型与短期令牌。
  `cli_local` 沿用原启动描述，不调用本 driver，不注入客户端覆盖。
- 显式冲突环境、模型参数、非原生命令和非 loopback 端点拒绝；
  清空 Node/Bun 注入及代理覆盖，禁止模型目录自动下载/自动更新。
  `--pure` 是该版本原生选项，禁用外部插件，不替换 Agent 或执行模型服务。
- 原生配置仍保留项目权限规则；系统托管配置在 inline 配置之后加载，
  因此检测到管理员配置、失效策略符号链接或不能检查策略时直接拒绝接管，
  不覆盖管理员策略。原生模式未被此检查改写。

## 真实生产链

[私有固定版本 manifest](kilocode/cli-versions.json) 与
[验收入口](kilocode/verify.py) 复用任务 10 的 `run`、真实二进制版本检查、
生产 Rust ACP 入口和生命周期断言；没有替换 Cargo、生产路由或 Agent。
仅扩展原生 fixture 的多 XDG 根隔离，并禁止模型目录联网及自动更新。
标题辅助请求依据实际 HTTP 消息中的 title 标识及无工具请求识别，只保留布尔值；
辅助模型是额外必须通过的观察，不靠配置字段推断通过。

| 来源 | 有效生产测试 | Cargo 退出码 | HTTP 请求 | 标题辅助请求 | 错误 |
| --- | ---: | ---: | ---: | ---: | ---: |
| session-model | 1 | 0 | 11 | 2 | 0 |
| cli-local | 1 | 0 | 8 | 1 | 0 |

两来源所有请求均认证成功，路径均为 `/v1/chat/completions`。
托管只收到 `acp-fixture-primary` / `acp-fixture-switched`，
本地只收到 `acp-fixture-primary`。冲突原生配置的 forbidden 模型/令牌未被使用。
新建、续轮、取消、重启、切模、隔离和握手均为 true；两次真实文件工具结果到达上游。
脱敏原始结果：[托管](kilocode/session-model.json)、[本地](kilocode/cli-local.json)。

复现时安装固定包到私有目录，并将其 `node_modules/.bin` 加入 PATH；
先按任务规则领取共享槽位。两个命令均在仓库根执行：

```bash
python3 docs/vibewindow/acp-model-source/evidence/kilocode/verify.py --agent kilocode --mode session-model --manifest docs/vibewindow/acp-model-source/evidence/kilocode/cli-versions.json --output target/acp-model-source-evidence/kilocode/session-model.json --offline-fixture
python3 docs/vibewindow/acp-model-source/evidence/kilocode/verify.py --agent kilocode --mode cli-local --manifest docs/vibewindow/acp-model-source/evidence/kilocode/cli-versions.json --output target/acp-model-source-evidence/kilocode/cli-local.json --offline-fixture
```

任务中原样公共 manifest 命令也已运行，两项均退出 1、有效测试 0，原因是公共 manifest
没有已审查的 offline 配方，不算通过证据。按任务边界不改公共 manifest、registry 或安装器；
任务 33 应集成此私有固定版本配方及 XDG 隔离要求后发布能力。

## 测试与共享槽位

红灯：具名 `acp_model_source_kilocode_creates_revocable_private_model_configuration`
命中 1 项，因原 driver 返回 `has not been verified` 失败，退出 101。
最初类型编译错误不计为红灯。首轮还发现仓库 `build.build-dir=target` 会分离中间产物，
后续验证同时设置 `CARGO_BUILD_BUILD_DIR=$CARGO_TARGET_DIR`，产物已确认位于共享槽位。

聚焦命令（格式化后）：

```bash
cargo fmt --all
cargo test -p vw-agent --lib --all-features acp_model_source_kilocode -- --nocapture
cargo clippy --all-targets --all-features -- -D warnings
```

最终 `cargo fmt --all` 退出 0；聚焦命令退出 0，**5 passed / 0 failed**。
另执行以下公共边界回归，各命中 1 项、退出 0：

```bash
cargo test -p vw-agent --lib --all-features acp_model_source_relay_chat_oauth_expiry_and_revocation_never_use_cached_keys -- --nocapture
cargo test -p vw-agent --lib --all-features acp_model_source_relay_binding_http_isolation_limits_and_lifecycle -- --nocapture
cargo clippy -p vw-agent --all-targets --all-features -- -D warnings
```

两项分别验证缺失/撤销凭据不使用缓存旧密钥，以及真实 HTTP 的模型禁用、隔离、
认证、限流和取消；包级 Clippy 退出 0。
最后一次全仓 `cargo clippy --all-targets --all-features -- -D warnings` 退出 **101**：
`crates/vw-desktop/src/app/components/system_settings_agents/shared.rs:218` 的
`models_for_provider` 和 `:240` 的 `with_selected_option` 未使用，被 `-D warnings` 拒绝。
这些是其他任务的工作区改动，本任务未修改。较早全仓检查曾退出 0，不能替代最终检查；
因此保留编号任务文件为“已完成，待回归”，不宣称全仓通过。
桌面改动稳定后重跑全仓 Clippy；通过后删除任务 24 文件并交由任务 33 集成。

聚焦测试验证私有配置及撤销、冲突输入拒绝、切模凭据轮换及历史目录、
真实 HTTP/子进程的四种模型角色，以及管理员策略拒绝。
真实 CLI 证据与上述配置消费者测试分开计数。

实际槽位 **3**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-3`；
后续所有验证同时设置相同的绝对 `CARGO_BUILD_BUILD_DIR`。
阶段结束仅释放自己的锁，保留缓存。

## 官方来源

- [固定 npm 版本元数据](https://registry.npmjs.org/@kilocode/cli/7.8.18)
- [固定源码：配置优先级](https://github.com/Kilo-Org/kilocode/blob/v7.8.18/packages/opencode/src/config/config.ts)
- [固定源码：XDG 状态目录](https://github.com/Kilo-Org/kilocode/blob/v7.8.18/packages/core/src/global.ts)
- [固定源码：ACP 恢复模型](https://github.com/Kilo-Org/kilocode/blob/v7.8.18/packages/opencode/src/acp/service.ts)
- [固定源码：托管策略](https://github.com/Kilo-Org/kilocode/blob/v7.8.18/packages/opencode/src/config/managed.ts)
