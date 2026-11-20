# Harn ACP 技能共存验收

**已阻塞。** Harn 0.10.201 的 CLI 能发现用户目录和环境变量指定目录，但 ACP 回合没有绑定原生技能注册表。受管 v4 的私有 pipeline 保留原生文件工具和 stdio MCP 桥。当前技能挂载门禁继续拒绝 Harn。[v4 MCP 基线](../mcp-acceptance/harn.md#v4-正式网关验收通过)

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
