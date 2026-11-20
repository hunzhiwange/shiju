# Copilot 1.0.105 技能共存验收

2026-11-20 在 Darwin arm64 复验受管修订 `copilot-1.0.105-model-source-v1`。启动入口为固定 npm 制品中的 `copilot --acp --stdio --no-auto-update`，以下四个文件经当前合约核验。

| 文件 | SHA-256 |
| --- | --- |
| `embedded/index.js` | `19f4cb854302397bdd880f5cde12ed069d373a630dce3f09a60e62b8ac8fcc4c` |
| `embedded/app.js` | `cb69bda81260cb16dcf740cc968c52da9e6c7bc80b7fb8eafa47fb986726e3ac` |
| `embedded/cli-main.js` | `1f8978f33a7a54105087ab9e66313a4b973dd852e198ab85b39c3de44959b502` |
| `embedded/prebuilds/darwin-arm64/runtime.node` | `423ba722d1fa544ba352fd6e702d5b67f78419995a0247580185aa030bcbc1fb` |

## 网关基线

复用槽位 10 已构建的 `shiju`，SHA-256 为 `b2a05ed113477712c36de9dd731517d27f151d4e51f7c8096c0e80ae2e27d276`。目标目录和构建目录均为仓库内的 `target/rust-slot-10`。探针启动私有网关，以 `session_model` 请求 `coexistence-fixture/deepseek-v4.1-flash-1017`，设置 `full_access=false`。模型请求实际到达回环 Chat Completions 夹具。

基线会话 `ses_e07ab35bdffe4in80BKQPYqZgS` 的第一条模型请求缺少 `.github/skills` 中的原生测试技能。网关发出一个 `chat.done`、零个 `chat.error`，下游调用为零。探针退出 1，首个断点为 `unexpected Copilot native skill catalogue`。基线止于目录验证，未执行网关挂载、撤除、拒绝和模式回读。结果见 `target/acp-skill-coexistence/copilot-before.json`。用最终探针复验同一网关，退出 1，会话 `ses_e07a389b1ffezuQ4q8iuwtV36l` 到达同一断点，见 `target/acp-skill-coexistence/copilot.json`。

探针沿用网关 `defaults.skills` 与项目级 MCP 配置，通过原生 `skill` 和 `view` 的调用 ID 核对正文与附件。MCP 使用独立鉴权的 SSE 只读服务，沿用 Copilot 已审传输。本次范围为 SSE。[原生工具和 MCP 审计](../mcp-policy-audit/coexistence/copilot.md)

## 隔离制品对照

从受审副本还原原始 JS 后校验了 `8b6ce892c34775028d9f8c31e39a346e24becdd94f660a6985f8ecb53e8b3fde`。原生 ACP 对照能发现工作区技能。受管版的空会话根关闭了技能，挂载实现仅转发会话根。

在私有副本中测试了两条路径。

- 启用广泛配置发现会把未经过显式信任的工作区技能纳入目录，超出现有受管隔离范围。
- 保持 `enableConfigDiscovery=false`，将私有 Copilot `settings.json` 的显式 `skillDirectories`、`disabledSkills` 和 `ignoredSkillsLocations` 传给 SDK，再追加会话根。对照观测到显式目录、选中目录与显式禁用均生效，未配置的工作区目录仍不可见。首次模型请求未包含夹具技能正文。[Copilot CLI 配置目录](https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-config-dir-reference)

第二条路径的直接 ACP 对照用独立 `COPILOT_HOME` 和 `allow_all=off`，验证了原生技能及会话技能激活、附件读回和 SSE MCP。基线、挂载、撤除分别收到 3、6、3 次模型请求，下游调用为 0、1、0。拒绝轮收到 5 次模型请求，Copilot 以同一调用 ID 发出 `failed`、`code=rejected`，下游零次执行，随后以 `end_turn` 结束，未产生模型续轮。这是私有供应商诊断，生产制品和网关合约未变。

## 阻塞

`session_model` 使用会话私有 `COPILOT_HOME`；当前启动合同没有租户和项目绑定的 Copilot 技能设置来源。应先确定可授权的配置来源，并只投影技能目录和排除项。直接恢复全量配置发现或引入用户全局配置会跨出现有隔离边界。探针支持按调用 ID 校验明确的拒绝终态，完整网关复验仍待受信配置接入。

共享验收判定的 Python 定向测试通过 5 项。此次未修改 Rust 技能配置、受管摘要或安装修订，排队的新网关构建在获得编译锁前取消，复用了同槽位已验产物。未运行 Rust 单测或桌面构建。
