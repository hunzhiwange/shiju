# Harn ACP 技能共存验收

受管 v4 基线中，Harn 0.10.201 的 CLI 能发现用户目录和环境变量指定目录，但 ACP 回合没有绑定原生技能注册表。受管 v4 的私有 pipeline 保留原生文件工具和 stdio MCP 桥。当前技能挂载门禁继续拒绝 Harn。[v4 MCP 基线](../mcp-acceptance/harn.md#v4-正式网关验收通过)

## 制品与直接对照

在 Darwin arm64 隔离目录中，用旧版受管安装提供的同一原生二进制与仓库的 v4 脚本组装私有副本。原生入口自报 `harn 0.10.201`，SHA-256 为 `23c951212c045c56deea491f9ce9505b8298472daa862334b33413ea31dcdcca`。启动器、pipeline 与 stdio 桥的摘要分别为 `17ad39da24845bb62914b46b748f30d63ebd11ca5f121af42fd8597a0ed94926`、`40ce0f80ad9033a8f72aa22d3228cba9f2068781aefaae7450664f312f335cdd`、`e6870d58c6c59cb1f1d224a66cdaa8455b34057249912b7bce5b9e21f95a074a`。四项均与现行合同一致。可用 `ACP_HARN_NATIVE` 指定同摘要的原生入口。

```sh
PYTHONPATH=scripts python3 -m acp_skill_coexistence.harn \
  > target/acp-skill-coexistence/harn-native.json
```

对照退出 0。原生 `harn skill resolved` 分别从私有用户目录的 `user` 层和 `HARN_SKILLS_PATH` 的 `env` 层发现夹具。同一二进制运行 `serve acp` 并完成握手与建会话。隔离诊断 pipeline 直接调用 `harness.agent.load_skill` 时，返回 `no skill registry is bound to this scope`。固定版 CLI 的加载代码负责同时设置全局目录和延迟读取器。ACP 的 VM 初始化缺少这两个绑定，且 `serve acp` 没有 `--skill-dir`。只修改受管 pipeline 或传入目录无法启用原生加载。裸 `harn run` 的加载路径也无法作为现有 ACP 协议和审批的直接替代。[固定版技能文档](https://github.com/burin-labs/harn/blob/v0.10.201/docs/src/skills.md) [CLI 技能绑定](https://github.com/burin-labs/harn/blob/v0.10.201/crates/harn-cli/src/skill_loader.rs) [ACP VM 初始化](https://github.com/burin-labs/harn/blob/v0.10.201/crates/harn-serve/src/adapters/acp/execute.rs)

## 私有网关回环

复用槽位 6 的现有网关，双缓存路径均为仓库的 `target/rust-slot-6`。网关 SHA-256 为 `8ee6357b62005376c74ef75d26aa03e9379af73f81e9e8b3a63f5552ec768c52`。网关使用 `defaults.skills` 与项目配置的只读 stdio MCP，模型绑定为 `coexistence-fixture/deepseek-v4.1-flash-1017`。夹具输出上限匹配实际 OpenAI Chat Completions 请求的 `max_tokens=16384`。默认 1024 的初次尝试返回 `provider_error`，模型端没有收到请求。

```sh
PYTHONPATH=scripts python3 -c 'import sys; from cargo_slot import run; sys.exit(run(sys.argv[1:], number=6))' \
  sh -c 'python3 scripts/verify_acp_skill_coexistence.py --agent harn --gateway-bin "$CARGO_TARGET_DIR/debug/shiju" --output target/acp-skill-coexistence/harn.json'
```

| 对照 | 模型请求 | `chat.done` / `chat.error` | MCP 审批 / 下游调用 | 首个断点 |
| --- | ---: | ---: | ---: | --- |
| 无挂载 | 2 | 1 / 0 | 0 / 0 | 首次模型请求含 `read_file` 与 `write_file`，缺少原生技能目录项和 `load_skill` |
| 单独挂载 | 0 | 0 / 1 | 0 / 0 | `harn: no verified skill directory mount branch` |

标准入口在基线的技能断言处退出 1。单独挂载对照临时只运行 `mounted`，同样退出 1，结果见 `target/acp-skill-coexistence/harn-mount-gate.json`。基线回读确认 `acp-coexistence / harn`、`session_model` 和 `full_access=false`，后端能力回读为 `unavailable`。首次模型请求没有预置技能正文、参考附件或 MCP 结果。两轮均没有可供核对的技能激活或 MCP 调用 ID。撤除和逐次审批拒绝对照缺少通过条件。

槽位 9 的新构建在复用网关取证后取消，退出 130，子进程已退出且槽位已释放。Python 语法和 Git 空白检查通过。Rust 和受管制品没有变更，未运行 Rust 单测。继续接入需要受审 ACP 制品在每个会话安装原生延迟技能注册表，并允许追加受信选择目录。

## 2026-11-21 受审替代路径复核

共享工作区仍使用 0.10.201 受管 v4 合同及原 Harn pipeline、启动器和中继驱动，Harn 选集仍落入宿主默认拒绝分支。上一轮回执中的网关与直接对照已将阻塞定位到供应商 ACP 初始化。这次没有重复构建或运行网关四轮。

- 扩展上述独立对照，使用同一摘要的原生二进制。CLI `harn run` 通过 `harness.agent.load_skill` 成功读取两份夹具正文；`HARN_SKILLS_PATH` 已设置且目录实际存在时，`serve acp` 中对包含 `name` 和真实 `skill_dir` 的元数据调用 `harness.agent.skill_render` 返回空串。随后 `load_skill` 仍以未绑定注册表失败。扩展对照退出 0，新增证据字段为 `cli_loaded_bodies=2` 与 `acp_metadata_only_render=empty`。
- Harn 的 `AgentSpec.skills` 能传入元数据，但标准库的回退需要已绑定的延迟读取器或预置正文。后者无法满足私有回环对首次模型请求和按需读取的约束，也无法替代原生签名检查。Harn 的 `HostSkillSource` 是 Rust 嵌入接口。固定版 ACP 没有对应的 RPC 注册入口，现有文件和 stdio MCP 回调无法为其安装读取器。[原生注册与渲染](https://github.com/burin-labs/harn/blob/v0.10.201/crates/harn-vm/src/stdlib/skills.rs) [标准库回退](https://github.com/burin-labs/harn/blob/v0.10.201/crates/harn-stdlib/src/stdlib/agent/skills.harn)
- 官方后续发行 v0.10.202 的 ACP 初始化仍未绑定技能注册表。以 0.10.201 标签提交 `4914194ae9f56b0046210ca90ef6f927988e727e` 测得 `cargo +1.99.1 metadata --locked --format-version 1` 在线解析退出 0，未编译供应商。源码级接入会生成与现有合同摘要不同的新原生可执行文件，现有脚本补丁和官方归档均无法提供该入口。[后续版 ACP 初始化](https://github.com/burin-labs/harn/blob/v0.10.202/crates/harn-serve/src/adapters/acp/execute.rs) [官方后续发行](https://github.com/burin-labs/harn/releases/tag/v0.10.202)

继续所需输入为带可核验摘要的受审 Darwin arm64 Harn ACP 制品，或经审定的固定源码补丁与可复现构建契约。该入口需在每个 ACP 会话绑定原生延迟读取器，支持当前工作区与受信选集，保留禁用与签名语义，并沿用 v4 的模型中继、MCP 沙箱及逐次审批。然后再接入宿主选集分派并运行三轮与拒绝对照。

## 受管 v5 源码接入

v5 固定使用上游 0.10.201 提交 `4914194ae9f56b0046210ca90ef6f927988e727e`。Darwin arm64 的 `cargo +1.99.1 build --locked -j 1 -p harn-cli --bin harn` 使用槽位 11 退出 0。源码归档 SHA-256 为 `26aca7b48967035e57924268ba86fce9b0adc8b687d006346deb755d662f7d2f`，补丁 SHA-256 为 `30752aa0fadd36675e1b088cbca5eb8a20de3e98cc0cc1f5a0c5b7d1e10d420a`，本次 Mach-O 引擎 SHA-256 为 `9c6cad508a3d976068c92737eaec22b659685603dd4b753c8a64351bb2d39598`。归档上的补丁预演通过。安装与快照测试合计 5 项通过。

补丁在 ACP 回合 VM 基线恢复后复用 CLI 元数据扫描、签名判断和延迟读取器，回合退出时恢复线程原有绑定。启动器仅暂存工作区 `.harn/skills` 的清单和签名，宿主通过既有会话租约追加选中的只读技能快照。原生同名项优先，供应商禁用标记继续生效。附件转给宿主文件入口，MCP 沿用 v4 沙箱及一次性审批。

使用 v5 引擎的直接 ACP 正向对照退出 0，两份私有正文均由 `harness.agent.load_skill` 读取。随后通过真实 v5 启动器、Chat Completions 回环和真实 stdio 服务运行四个独立 ACP 会话，全部返回 `end_turn`。模型请求数为 3/6/3/6，下游 MCP 次数为 0/1/0/0。挂载批准的 MCP 调用 ID 为 `call_harn_860dcede3420f9e994e358f8`，拒绝调用 ID 为 `call_harn_2ef7b42993677833dd6c115e`。挂载的原生激活、原生附件、选中激活、选中附件分别通过 `call_harn_287aa64ae9073cb263db8780`、`call_harn_8540f93242087a2f432ba2ee`、`call_harn_74ce088249da5f3a375fd53b`、`call_harn_69a5fe83b256ecb3c74c982a` 进入模型续轮。直接 ACP 证据位于 `target/acp-skill-coexistence/harn-direct-v5.json` 与 `target/acp-skill-coexistence/harn-pipeline-v5.jsonl`。

### 生产网关四轮

槽位 11 的 `cargo build --locked -j 1 -p vw-cli --bin shiju` 退出 0。网关 SHA-256 为 `ae85296f170c43eff181a717e9d5016b043b91089b43cc89cc9fff0f138c44b7`，构建和验收的 `CARGO_TARGET_DIR`、`CARGO_BUILD_BUILD_DIR` 均为仓库的 `target/rust-slot-11`。沿任务验收入口运行，`ACP_HARN_RUNTIME` 指向核验过构建回执的 v5 私有制品目录。

```sh
python3 scripts/verify_acp_skill_coexistence.py --agent harn \
  --gateway-bin "$CARGO_TARGET_DIR/debug/shiju" \
  --output target/acp-skill-coexistence/harn.json
```

退出 0。四个独立会话均回读 `acp-coexistence / harn`、`session_model`、`coexistence-fixture/deepseek-v4.1-flash-1017` 和 `full_access=false`。每轮恰有一个 `chat.done`，零 `chat.error`。首个模型请求没有预置技能正文或附件。禁用的原生技能、未选技能以及撤除的旧选择均未出现在目录。

| 轮次 | 网关会话 | 模型请求 | MCP 执行 | MCP 审批 |
| --- | --- | ---: | ---: | --- |
| 无挂载 | `ses_e03c4ced8ffela0IhkyCNDBQcS` | 3 | 0 | 无 |
| 挂载 | `ses_e03c45feeffeuOYRzk1gNvIX6C` | 6 | 1 | 一次 |
| 撤除 | `ses_e03c3d407ffetTjmJZVfSh7zdH` | 3 | 0 | 无 |
| 拒绝 | `ses_e03c349a0fferC6ArtMzcotjIb` | 6 | 0 | 拒绝 |

挂载轮中，原生技能及其附件通过 `call_harn_2200fd563b0cabb2dab387aa` 和 `call_harn_065a09e4ae00f8e3b59da070` 返回正文。选中技能及其附件通过 `call_harn_6a7de07fa83cd24d396d5586` 和 `call_harn_815a367a59508bff180bb419` 返回正文。MCP 批准结果以 `call_harn_e1c4b2a83c4bb6a0b83018f0` 进入模型续轮，拒绝反馈以 `call_harn_00f8332dedafb54c9cb00805` 进入续轮。结构化证据位于 `target/acp-skill-coexistence/harn.json`，原 v4 失败基线位于 `target/acp-skill-coexistence/harn-v4-baseline.json`。

基于同一 v5 引擎的原生文件权限回归 3 项和自然语言入口回归 1 项均退出 0。现行官方安装和用户配置未改。

槽位 11 的两个定向 Rust 测试均退出 0，各实际运行并通过 1 项。首轮 `llm-runtime` 测试在编译时发现测试夹具缺少 `Clone`，补齐后复跑通过。生产代码未因这次测试修复变更。

```sh
cargo test --locked -j 1 -p llm-runtime --lib acp_skill_coexistence_harn -- --test-threads=1
cargo test --locked -j 1 -p acp-agent-registry --lib harn_reviewed_natural_language_runtime_is_pinned -- --test-threads=1
```
