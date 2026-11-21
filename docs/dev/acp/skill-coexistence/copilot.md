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

## v1 的配置阻点

`session_model` 使用会话私有 `COPILOT_HOME`；当前启动合同没有租户和项目绑定的 Copilot 技能设置来源。应先确定可授权的配置来源，并只投影技能目录和排除项。直接恢复全量配置发现或引入用户全局配置会跨出现有隔离边界。探针支持按调用 ID 校验明确的拒绝终态，完整网关复验仍待受信配置接入。

共享验收判定的 Python 定向测试通过 5 项。此次未修改 Rust 技能配置、受管摘要或安装修订，排队的新网关构建在获得编译锁前取消，复用了同槽位已验产物。未运行 Rust 单测或桌面构建。

## 共享工作区复查

在 `2127c62f6b` 上逐项比较首次回执提交后的 Copilot 驱动、技能挂载模块、固定补丁和受管合约，Copilot 条目没有差异。新一轮替代路径诊断使用私有目录、同一 SHA-256 校验的原始和受管入口及回环模型。项目根、候选技能目录及模型凭据均为临时夹具。

| 固定入口与配置 | 显式目录 | 未登记的项目技能 |
| --- | --- | --- |
| 原版启动，仅传 ACP `session/new` 参数及 `_meta` | 不可见 | 可见 |
| 原版启动，`COPILOT_SKILLS_DIRS` | 可见 | 可见 |
| 原版启动，`--plugin-dir` | 可见 | 可见 |
| 受管 v1，空选择 | 不可见 | 不可见 |
| 私有窄补丁，`enableConfigDiscovery=false`，只投影技能设置 | 可见 | 不可见 |
| 私有窄补丁，技能名在 `disabledSkills` 中 | 不可见 | 不可见 |
| 私有窄补丁，技能目录在 `ignoredSkillsLocations` 中 | 不可见 | 不可见 |

私有窄补丁另用带 `allowed-tools` 声明的原生技能复验只读 SSE 工具。四轮的 `allow_all` 均为 `off`，MCP 仍向 ACP 客户端请求单次审批。批准轮调用下游一次，拒绝轮下游为零，收到同调用 ID 的 `code=rejected`。此对照只覆盖固定版和本次声明的 SSE 工具。目录和审批补测均退出 0，脱敏日志分别位于 `target/acp-skill-coexistence/copilot-alternatives.log` 和 `target/acp-skill-coexistence/copilot-direct-allowed-tools.log`。

共享 `.vwacprc.json` 中的进程覆盖由全局层和项目文件合并，现有字段没有明确授予原生技能读取范围的来源标识。租约的 `state_root` 也是网关级目录。原生 Copilot 工作区信任查询使用其配置目录，而模型驱动使用当前会话的私有 `COPILOT_HOME`。已有项目工具白名单仅限制已声明工具的可见性。上述入口均未提供可直接复用的租户及项目级技能授权。独立的 `cli_local` 也无法验证任务指定的 `session_model`。

继续接线所需的输入是租户管理员认可的项目级原生 Copilot 设置来源，以及允许投影的目录范围和排除项。例如授权一个按租户及项目登记、只投影 `skillDirectories`、`disabledSkills` 和 `ignoredSkillsLocations` 的配置来源。确认之前保留门禁和原制品。本轮重用已有网关失败证据，未重建网关或重跑相同探针。

## v2 生产接线与固定入口

新版从租户全局 `acp.copilot.env.SHIJU_COPILOT_PROJECT_SKILLS` 读取规范化的项目绝对路径。宿主将授权与当前租户存储和项目上下文核对，拒绝项目覆盖自行创造或更改授权。其他项目继续使用原有启动方式。授权仅用于工具开启的 `session_model`。宿主读取项目内 `.github/copilot/settings.json`，解析三项技能设置并写入私有 `COPILOT_HOME`。根目录仅接受原生项目技能位置，拒绝路径逃逸、符号链接、硬链接和非技能设置。单个技能用 `disabledSkills` 排除，`ignoredSkillsLocations` 只接受原生发现根，因为 SDK 会忽略该字段中的单技能子目录。

受管 v2 延续全量配置、插件和内置发现隔离。原生目录按配置顺序传给 SDK，后追加已校验的会话视图。SHA-256 校验过原始文件后重新执行仓库补丁，v2 `embedded/cli-main.js` 的摘要为 `5233b13e859de96f86a21bd8806611dc0bf534e4b63d45e6d81475f92100bd64`，通过 `node --check`。其余受管摘要沿用 v1。 [固定补丁](../../../../patches/acp-model-source/copilot/README.md)

Darwin arm64 的固定 v2 直连验证以 0 退出。四轮均读取原生随机正文及附件。挂载轮和拒绝轮额外从会话视图读取选中技能，未选项和两个原生排除项始终不可见。同名对照里由配置在前的原生项目技能胜出。SSE 单次允许后下游调用为 1，拒绝后为 0，原生 `allow_all` 四轮均为 `off`。证据位于 `target/acp-skill-coexistence/copilot-direct-v2.log` 和 `target/acp-skill-coexistence/copilot-direct-collision.log`。

补测在原生技能声明 `allowed-tools: bash` 后申请执行仅输出常量的命令。真实 ACP 发出权限请求，选择单次拒绝后回合结束。该命令没有获得技能自授权。诊断退出 0，见 `target/acp-skill-coexistence/copilot-direct-shell-approval.log`。共享 Python 验收判定通过 5 项。
