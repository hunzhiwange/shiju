# Mistral Vibe 模型来源适配

状态：已完成；2026-11-04 两种来源均已取得正式 Cargo 真实 CLI 验收证据。

固定版本：官方 `mistral-vibe==3.0.4`，使用此任务的最小会话绑定补丁；仅验收
Darwin arm64 / Python 3.12.16，未声称 Linux 或 Windows 通过。
[发行包 URL 与 SHA256](vibe/upstream-distribution.json)、[补丁](vibe/mistral-vibe-3.0.4-session-binding.patch)、
[安装与复现说明](vibe/README.md)、[独立验收清单](vibe/manifest.json)。公共版本清单未修改。

## 实现与原阻塞的修复

`drivers/vibe.rs::configure` 写入 0600 的私有 TOML，并将 `VIBE_HOME` 指向本次
profile 的 0700 临时目录。Provider 只指向 loopback 中继，短期令牌仅在专用环境变量，
不复制用户 Mistral 账号，不将令牌或上游凭据写入 argv。模型名称通过 TOML 序列化，
包含引号或中文时仍保持原值。原生命令有配置冲突、未知协议、非 loopback 中继或
不支持 temperature 的模型时，在配置发布前拒绝。输出限制使用所选模型的上限。

原上游配置顺序允许 Agent profile 覆盖用户 TOML 和环境变量。实测内置 Lean profile
能将绑定模型替换为 `labs-leanstral-1-5`，压缩模型替换为 `mistral-small-latest`，
Provider 改回 Mistral 公网。因此补丁在每次 builder 合并后、schema 校验前投影本次
模型路由，固定主模型、所有子 Agent 经过的模型目录、压缩与标题/smart-approve 模型。
未改写 Agent 行为、工具权限、MCP 或系统提示；组织管理配置涉及模型字段时明确拒绝，
不绕过组织策略。视觉回退被清空；共享 profile 尚无图片能力元数据，此版本保守支持
文本，未声称图片能力通过。音频不在原生 ACP 声明的能力中。

| 所选 Provider | Vibe api_style | 私有中继 api_base |
| --- | --- | --- |
| openai / openai-compatible | openai | /v1 |
| openai-responses 或 OpenAI /responses 端点 | openai-responses | /v1 |
| anthropic | anthropic | 根路径（Vibe 自行追加 /v1/messages） |
| google | openai | /v1，使用现有 Chat 中继转换；未验收 Vibe 原生 Gemini |

`--session-model-config` 是显式的新 ACP 参数。原版固定版本在执行模型请求前拒绝此
未知参数（实测退出码 2），不会静默忽略绑定。补丁应用脚本仅允许独立 virtualenv
中的 3.0.4，先核对原始文件 SHA256，再 dry-run 和应用；改变过的基线直接拒绝。
`cli_local` 沿用原命令、参数及环境，不传该标记，不进入本 driver，保留原生 profile
选择行为。任务 33 发布能力前须将补丁纳入实际 CLI 分发。

## 上游与真实 CLI 证据

- 原版发行包对照：独立未补丁 virtualenv 命中基础 6 个配置用例，5 failed、
  原生行为用例通过，退出码 1。失败分别为 profile 模型越界、组织模型策略冲突未拒绝、
  远端绑定未拒绝、不安全权限未拒绝及缺失中继令牌未拒绝；不是导入或编译错误。
- 补丁后的 `upstream_tests.py`：7 passed，退出码 0，覆盖 profile 越界、原生行为、
  组织策略冲突、远端中继、不安全文件权限、缺失中继令牌及跨私有目录的历史隔离。直接调用安装发行包的真实
  配置实现，不 mock 配置合并。
- [session-model 三协议](vibe/native-session-model.json) 与
  [cli-local 三协议](vibe/native-cli-local.json)：两种来源各执行 Chat、Responses、
  Anthropic 真实 ACP 请求，每协议本次观察到 2 次认证成功的模型/辅助请求，路径分别为
  `/v1/chat/completions`、`/v1/responses`、`/v1/messages`；检查真实 ACP 返回文本和版本。
- [session-model 生命周期](vibe/lifecycle-session-model.json)：7 次请求，模型从
  `acp-fixture-primary` 切到 `acp-fixture-switched`；原生配置故意使用错误模型。
- [cli-local 生命周期](vibe/lifecycle-cli-local.json)：7 次请求，始终使用原生
  `acp-fixture-primary`。两种来源均实际完成两个文件读取工具回合、第二轮、取消、
  重启恢复；认证全部正确、fixture 错误 0。session-model 最终探针还轮换了私有
  `VIBE_HOME`，检查保留状态中短期令牌未持久化，退出码 0。

上述 Python 生命周期证据启动真实补丁 Vibe；唯一模拟的是模型 HTTP 服务，复用任务
10 的 fixture。它们是供应商原生 ACP 能力证据，不冒充 `do_stream_request_acp` 的生产
调用链验收。所有临时配置均位于专属目录，没有生产账号或付费模型请求。

## Rust 与生产调用链

原始红灯命令：
`cargo test -p vw-agent --lib --all-features acp_model_source_vibe -- --nocapture`。
槽位 3，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-3`；
`cargo fmt --all` 退出 0；3 个具名测试启动，主模型绑定用例命中原占位拒绝
`ACP vibe session_model launch has not been verified`，Cargo 退出 101。随后 panic
处理发生 abort，未将该 abort 或零测试当作红灯证据。

最终聚焦命令同上，动态槽位 1，`CARGO_TARGET_DIR` 和 `CARGO_BUILD_BUILD_DIR` 均为
`/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。`cargo fmt --all` 退出 0；
聚焦测试 **5 passed / 0 failed**，退出 0，具名测试为：

- `acp_model_source_vibe_private_binding_pins_primary_and_auxiliary_models`
- `acp_model_source_vibe_rejects_unsupported_protocol_before_publication`
- `acp_model_source_vibe_rejects_conflicting_native_home`
- `acp_model_source_vibe_rejects_temperature_and_missing_metadata`
- `acp_model_source_vibe_selects_native_protocol_and_rejects_remote_relay`

一次中间绿灯编译阶段被会话中断，未产生测试结论；核实该阶段 holder PID 和子进程均
已退出后，才清理自己的遗留锁。再次按原子 mkdir 动态领取槽位 1，保留原缓存完成上述
绿灯。没有把中断或编译成功当作行为验收通过。

`cargo clippy --all-targets --all-features -- -D warnings`：同一槽位 1，退出 **0**，
58.59 秒。之后仅修改/验证独立 Python 上游补丁，Rust 源码没有变化，未重复已通过的
聚焦测试和 Clippy。

生产链首次暴露真实问题：每轮删除私有 `VIBE_HOME` 时，Vibe 历史一起丢失，第二轮
加载失败，取消探针未再到达模型服务。保留了[失败观察](vibe/session-model-before-history-fix.json)，
没有把 3 次请求和两个工具回合冒充生命周期通过。随后新增跨私有目录行为红灯（1 failed），
在独立上游补丁中将历史及关联缓存分离到当前用户私有、按既有绑定摘要隔离的系统临时缓存，
凭据目录仍逐轮删除和撤销。另以 `sessions` 子目录隔离其父层的 MCP 缓存，专属行为用例
先失败后通过；最终上游 7 passed。详情见 [README](vibe/README.md)。没有修改共享生命周期代码。

### 实际生产入口的缓存二进制证据

使用当前槽位已编译的真实 `vw-agent` libtest 二进制，调用任务 10 的原生产验收入口、
原 HTTP fixture、原判定和真实 Vibe 子进程；仅替换 Cargo 启动边界，防止共享源码持续
变化导致重复整库编译。报告明确标记 `cached-production-binary`、`binary_exit_code`，
不声称正式 Cargo 验收已通过。

| 来源 | 有效测试 | 二进制退出码 | 生成请求 | 模型与生命周期 |
| --- | --- | --- | --- | --- |
| [session-model](vibe/session-model-cached.json) | 1 | 0 | 11 | primary → switched；所有生命周期布尔值为真 |
| [cli-local](vibe/cli-local-cached.json) | 1 | 0 | 8 | 始终原生 primary；所有生命周期布尔值为真 |

两种模式实际完成两个文件读取工具回合，所有请求认证正确，fixture errors=0；
session-model 的原生配置故意包含错误模型和错误凭据。最终 session-model 使用
**槽位 6**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-6`；
cli-local 使用槽位 1。两者均是 all-features 的真实生产测试二进制，session-model
报告额外保存编译特性。两种缓存证据均不是模拟 ACP agent。

正式 `verify_acp_model_source.py` 使用独立 manifest 启动后再次触发整库编译；在共享
源码仍变化期间，按最终统一正式回归安排停止重复构建，外层退出 130，未取得有效正式
测试结果。清理时发现 Cargo 使用独立进程组仍存活，按事先确认的本任务 PID/PGID
终止并核实全部退出，才继续领取槽位；没有杀其他任务或删除缓存。后续缓存重放改为同一
Python 进程持锁，所有相关子进程结束后释放自己的锁。历史另有槽位 2 的预检因二进制
不匹配直接退出，零测试，未作为任何验证证据。

上述是实现阶段的待回归记录，已由下方正式结果解除。共享能力发布和补丁实际分发仍按
整体计划集成；本任务未修改共享分派、profile、fixture 或公共 manifest。
本轮对最新共享工作区动态领取槽位并设置绝对构建路径，实际执行以下正式入口：

```sh
export PATH="$PWD/target/vibe-cli-env/bin:$PATH"
python3 scripts/verify_acp_model_source.py --agent vibe --mode session-model --manifest docs/vibewindow/acp-model-source/evidence/vibe/manifest.json --output target/acp-model-source-evidence/vibe/session-model-final.json --offline-fixture
python3 scripts/verify_acp_model_source.py --agent vibe --mode cli-local --manifest docs/vibewindow/acp-model-source/evidence/vibe/manifest.json --output target/acp-model-source-evidence/vibe/cli-local-final.json --offline-fixture
```

### 2026-11-04 正式 Cargo 回归完成

[回归记录](vibe/formal-regression.json) 使用原 `verify_acp_model_source.py` 直接调用
Cargo，没有替换 Cargo 启动函数，没有使用 cached-production-binary 结果充当通过。

| 来源 | 正式有效测试 | Cargo 退出码 | 请求次数 | 模型 |
| --- | --- | --- | --- | --- |
| [session-model](vibe/session-model.json) | 1 passed / 0 failed | 0 | 10 | primary → switched |
| [cli-local](vibe/cli-local.json) | 1 passed / 0 failed | 0 | 8 | 始终原生 primary |

两种来源的所有请求均认证正确，fixture errors=0；实际文件读取工具回合、新建、第二轮、
取消、重启恢复、切模/本地保持、隔离和握手检查全部为真。实际具名入口为 `acp_model_source_real_cli_session_model` 与
`acp_model_source_real_cli_cli_local`；两者均由 `cargo test -p vw-agent --lib --all-features`
执行，测试参数为 `-- --ignored --nocapture --test-threads=1`。完整 argv、有效数量和退出码
保存在各自的正式报告中。

本轮固定真实 CLI 仍为补丁后的 Mistral Vibe 3.0.4，Darwin arm64。
实际动态槽位 **1**：`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，
`CARGO_BUILD_BUILD_DIR` 使用同一绝对路径。两种入口及其子进程结束后，已释放自己的锁，
编译缓存保留。未修改 Rust 或上游补丁；之前通过且未受影响的 5 项 Vibe 聚焦测试、
7 项上游行为测试和 Clippy 不重复执行。没有为回归修改共享文件。

正式证据已齐备，编号任务按契约标为完成并删除。
