# Snow 模型来源适配证据

状态：已完成。最新共享工作区上的正式 Cargo 回归已通过；仅验证 Darwin arm64，
其他系统未验证。缓存生产二进制结果保留为历史诊断，正式验收以本次 Cargo 结果为准。

固定发行包 `snow-ai@0.9.14`，源码基线 `a9eba5f9062afbf639829b4992e90a986924f519`。
独立清单见 [snow/manifest.json](snow/manifest.json)，npm integrity 与基线一致。
包安装于 `target/snow-cli-0.9.14`；专属配置目录中的 `--version` 及 ACP agentInfo
均核对为 0.9.14，没有读取真实用户 Snow 配置或使用生产模型账号。

## 实现

`snow::configure` 写入 0600 的私有 `config.json`，`SNOW_CONFIG_DIR` 指向 profile
的 0700 临时目录。主、辅助模型均使用会话真实模型，中继令牌仅保存在私有文件，
argv 保持原样。不复制 OAuth、其他 profile 或视觉模型回退配置。

运行时 profile 接收本次解析的 API 元数据、输出限制和 temperature 能力：

| 所选模型 API | Snow requestMethod | 中继配置根 |
| --- | --- | --- |
| openai/openai-compatible | chat | /v1 |
| openai-responses，或 OpenAI 的 /responses 端点 | responses | /v1 |
| anthropic | anthropic | /v1 |
| google | gemini | /v1beta |

不认识的协议直接拒绝；上游 URL 不写入 CLI 配置。Chat 固定携带 temperature，
模型不支持时在 CLI 启动前报错。`maxTokens` 使用当前模型输出上限；零上限使用
集中默认值，避免 Snow 默认 64000 超过模型上限。`cli_local` 保留原命令、参数及环境，
不调用本 driver。取消、恢复、撤销及模型禁用仍经过既有生产模型来源链。

## 验证记录

- 原始红灯：槽位 1，`cargo test -p vw-agent --lib --all-features acp_model_source_snow -- --nocapture`，
  退出 101，具名测试命中原 Snow 占位错误。更早的测试编译错误不算红灯。
- 最终聚焦验证：同一命令，槽位 3，退出 0，**3 passed / 0 failed**：
  `acp_model_source_snow_private_config_and_revocation`、
  `acp_model_source_snow_rejects_unsupported_temperature_before_config_publication`、
  `acp_model_source_snow_selects_native_protocol_without_changing_upstream_credentials`。
- `cargo fmt --all`：最终槽位 3，退出 0。
- `cargo clippy --all-targets --all-features -- -D warnings`：最终槽位 3，退出 0（1m 26s）。
- Python 合同测试：`python3 -m unittest discover -s scripts -p 'acp_model_source_fixture_tests.py'`，
  退出 0，17 项通过；新增目录鉴权及 Snow 绝对读取路径用例先失败后通过。
- 四种原生协议：`python3 docs/vibewindow/acp-model-source/evidence/snow/native_probe.py`，
  退出 0。真实 Snow ACP 各完成 1 次带认证的模型请求，核对版本、模型和精确路径。
  [原生协议观察](snow/native-protocols.json) 属于供应商原生接口证据，不是四种 Provider 的完整生产生命周期验收。

共享验收修复在本批次其他七项退出后串行执行：模型 fixture 支持带认证的目录探测、
`filesystem-read` 与本次临时文件的绝对路径；验收模型显式声明 temperature 能力；
测试进程默认使用 32 MiB 线程栈。目录探测不计作生成请求，错误认证仍被拒绝。
没有放宽生产中继能力检查，没有模拟 ACP CLI 或供应商工具执行。

## 正式 Cargo 回归（2026-11-04）

使用原始 `scripts/verify_acp_model_source.py`，未替换 Cargo 执行器。阶段记录见
[formal-regression.json](snow/formal-regression.json)。

| 来源 | 脚本退出码 | Cargo 退出码 | 有效测试 | 真实生成请求 |
| --- | --- | --- | --- | --- |
| [session-model](snow/session-model.json) | 0 | 0 | 1 passed | 13 |
| [cli-local](snow/cli-local.json) | 0 | 0 | 1 passed | 13 |

两种来源均 `passed=true`、fixture 错误数 0，认证及模型正确、工具结果达到 2；
新建、第二轮、取消、恢复、切换、隔离及握手均通过。session-model 从 primary
切换到 switched，cli-local 始终使用 primary。正式运行前的聚焦预热也通过：3 passed，退出 0。

本次动态领取槽位 **2**，`CARGO_TARGET_DIR` 与 `CARGO_BUILD_BUILD_DIR` 均为
`/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。全部相关进程结束后已释放自己的锁，
保留缓存。此前已通过且未受改动影响的 Clippy、Python 合同测试和四协议探测不重复执行。

## 历史诊断：缓存生产二进制与正式 Cargo 验收的区别

使用槽位 3 中刚通过上述 3 项测试的**同一生产测试二进制**，调用任务 10 原有
验收入口、原有判定及真实 Snow 子进程；只将 Cargo 启动替换为该二进制执行，避免
其他计划持续改动触发整库重复重编译。报告明确记为 `cached-production-binary`，
`command` 记录真实二进制命令，退出码记作 `binary_exit_code`。

| 来源 | 有效测试 | 二进制退出码 | 生成请求 | 模型与工具回合 |
| --- | --- | --- | --- | --- |
| [session-model](snow/session-model-cached.json) | 1 | 0 | 13 | primary → switched，实际工具结果达到 2 |
| [cli-local](snow/cli-local-cached.json) | 1 | 0 | 13 | 始终 primary，实际工具结果达到 2 |

两份结果均 `passed=true`、认证全部正确、fixture 错误数 0；新建、第二轮、取消、
重启恢复、切换、隔离及握手均为真。session-model 原生配置故意包含错误模型和
错误凭据，真实请求没有使用它们。此前原始 Cargo cli-local 入口也取得过
1 passed / 13 requests，但不能代替最新源码的统一回归。

此前正式 Cargo 重跑遇到其他计划的 `stream.rs` 配置字段改动、Rust 增量编译错误，
以及持续源码改动引起的重复重编译/验收器 300 秒超时。当时检测到再次重编译后，
依调度指示停止该重复构建；没有改动无关文件，也没有把零测试记作通过。

前次实现与缓存二进制验证槽位为 **3**：
`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-3`，
`CARGO_BUILD_BUILD_DIR` 使用相同绝对路径。此前使用过槽位 1、5、10，分别对应
`/Users/xiangmin/worker/code/shiju/target/rust-slot-1`、`rust-slot-5`、`rust-slot-10`。
所有槽位动态领取，自己的锁在所有相关进程结束后释放，缓存保留。

## 正式验收复现命令

冻结无关源码改动，按编号任务规则领取槽位并设置两个绝对构建目录；先预热聚焦测试，
再执行以下原任务 10 验收器。公共版本清单仍保持只读，使用独立 Snow 清单：

```sh
export PATH="$PWD/target/snow-cli-0.9.14/node_modules/.bin:$PATH"
cargo test -p vw-agent --lib --all-features acp_model_source_snow -- --nocapture
python3 scripts/verify_acp_model_source.py --agent snow --mode session-model --manifest docs/vibewindow/acp-model-source/evidence/snow/manifest.json --output target/acp-model-source-evidence/snow/session-model-final --offline-fixture
python3 scripts/verify_acp_model_source.py --agent snow --mode cli-local --manifest docs/vibewindow/acp-model-source/evidence/snow/manifest.json --output target/acp-model-source-evidence/snow/cli-local-final --offline-fixture
cargo clippy --all-targets --all-features -- -D warnings
```

本次两种来源均已取得正式 Cargo 的 1 passed，已解除待回归并删除编号任务文件。
任务 33 集成时还需更新共享“所有 driver 未验收”基线测试，再发布能力。
