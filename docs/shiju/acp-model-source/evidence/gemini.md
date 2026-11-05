# Gemini CLI 模型来源验收（2026-11-04）

状态：已完成。仅验证 macOS 27.0 / arm64、Node v26.13.0、真实
`@google/gemini-cli@0.66.0`；不推断 Linux/Windows 已通过。

## 实现与固定版本

- `drivers/gemini.rs::configure`：API-key/Gateway 路由到本机 Gemini 中继；
  短期令牌只进入进程环境，模型及认证配置写入 0600 文件，配置根为 0700。
  禁止 OAuth/Vertex/ADC 竞争来源、Node 注入、代理和显式冲突覆盖。
  保留管理员非路由策略，管理员路由策略存在时明确拒绝接管。
- 同来源续轮保留独立 0700 历史目录；租户、项目、会话、Provider、模型和
  generation 参与摘要。临时配置及令牌仍随 profile 撤销；本地来源不注入覆盖。
- `request.rs` 改用 `get_current()`，读取当前租户/项目配置，加载失败显式返回错误。
  原 `get()` 只读默认租户全局配置；实际子进程用例先失败后通过。
- 公共验收仅补充与观测模型严格相同的 Gemini generateContent/
  streamGenerateContent 路径；countTokens 和错误模型路径不能算成功。
- 固定版本补丁及六个 bundle 的原始/结果 SHA-256 位于
  [补丁说明](../../../../patches/acp-model-source/gemini/README.md)。
  补丁锁定主/辅助请求模型、替换原生 Gemini 专用采样预设、分离历史。
  仅移除 CLI 自行添加的 `skip_thought_signature_validator`；保留不透明供应商签名，
  不伪造跨协议思考支持。原生未打补丁入口拒绝 managed flag。

实际运行副本来自本机固定 0.66.0 安装，bundle 复制到
`target/acp-gemini-runtime`，依赖只读链接原安装；未修改全局 CLI、用户 HOME 或账号。
Task 33 必须验证、分发该补丁并集成公共 manifest/安装器，才能发布能力。
本任务仅保存 [私有固定版本配方](gemini/manifest.json)。

## 真实生产链结果

使用任务 10 的原验收实现，`gemini/verify.py` 仅指定私有 manifest、固定 CLI PATH
和闭集诊断；正式结果没有替换 Cargo、生产路由或通过条件。

| 来源 | 有效测试 | 退出码 | 真实请求 | 模型及实际上游路径 |
| --- | ---: | ---: | ---: | --- |
| session-model | 1 | 0 | 9 | primary/switched；`/v1/chat/completions`（Gemini 入站由生产中继转换） |
| cli-local | 1 | 0 | 7 | primary；`/v1beta/models/acp-fixture-primary:streamGenerateContent` |

primary/switched 为 `acp-fixture-primary` / `acp-fixture-switched`。
两来源认证全部成功、错误数 0；新建、续轮、取消、重启、切模、隔离和握手均为 true。
两次真实文件工具结果已到达模型上游，不能用输出文本冒充模型请求。
原始脱敏结果：[托管](gemini/session-model.json)、[本地](gemini/cli-local.json)。

补充原生 bundle 探针使用真实 `Config`、模型解析器、生成器和 Google SDK，
四个 helper/classifier/compression 别名均到达选中模型；另两次请求验证合成签名删除、
不透明签名保留，共 6 次真实 HTTP 请求，退出码 0。
它是原生组件补充证据，单独不能替代上述完整 ACP 生产验收：
[辅助与签名证据](gemini/native-auxiliary-probe.json)。
独立真实 CLI 还复现了删除配置根后的恢复失败，并验证补丁恢复成功：
[原版](gemini/native-history-probe.json)、[补丁](gemini/patched-history-probe.json)。

## 测试驱动及确切命令

最初聚焦红灯命中 2 项：1 成功、1 因原 driver 明确未验证而失败（退出 101）。
新增项目配置/路径用例后红灯命中 6 项：4 成功、2 失败；分别为未启动项目子进程、
合法 Gemini 路径未被接受。最终相同过滤命中 6 项，失败 0，退出 0。
Python 新增协议、absolute_path、精确路径判定均经历有效红灯，最终 4/4，退出 0。

```sh
cargo fmt --all
cargo test -p vw-agent --lib --all-features acp_model_source_gemini -- --nocapture
python3 -m unittest discover -s scripts -p acp_model_source_fixture_tests.py -k GeminiContract
python3 docs/vibewindow/acp-model-source/evidence/gemini/verify.py --mode session-model --output target/acp-model-source-evidence/gemini/session-model-final
python3 docs/vibewindow/acp-model-source/evidence/gemini/verify.py --mode cli-local --output target/acp-model-source-evidence/gemini/cli-local-final
cargo clippy --all-targets --all-features -- -D warnings
python3 docs/vibewindow/acp-model-source/evidence/gemini/native_auxiliary_probe.py
```

两条 wrapper 分别执行原命令：
`cargo test -p vw-agent --lib --all-features acp_model_source_real_cli_session_model -- --ignored --nocapture --test-threads=1`
和 `acp_model_source_real_cli_cli_local` 对应命令，有效测试各 1，均退出 0。
最终 Clippy 退出 0。链接器另有现有 macOS unwind 表大小提示，不是本任务 Rust 警告。
未执行全量单元测试。

附加运行同一类库已构建二进制的 `acp_model_source_routing --nocapture`，10/10，退出 0，
覆盖凭据缺失、禁用模型快照、本地来源不解析客户端凭据；此项不冒充 Cargo 或 Gemini CLI 验收。

## Cargo 槽位与依赖

动态实际领取过槽位 1、2、3、4；初始受仓库 `build-dir` 配置影响的锁等待已中止，
后续同时设置 `CARGO_BUILD_BUILD_DIR` 和 `CARGO_TARGET_DIR` 到所领槽位。
主验证最终使用槽位 **1**：
`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。
额外 routing 二进制测试使用槽位 **2**：
`/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。
早期阶段槽位 3、4 对应同根目录 `rust-slot-3` / `rust-slot-4`。
全部自有锁在子进程结束后释放，缓存保留。

本地来源曾被 `agent_config.env` 大写映射键的持久化检查阻止；纳入 Claude 任务维护的
底层映射键修复后，重新构建并取得上述正式通过结果。该依赖由其所属任务提交，
本提交不混入其变更。中途 cached-production-binary 结果不作为最终正式通过替代。
