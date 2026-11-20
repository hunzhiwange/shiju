# DeepAgents 0.1.42 原生技能与会话挂载

验收平台为 Darwin arm64，Node v24.22.3。使用 `session_model` 绑定
`coexistence-fixture/deepseek-v4.1-flash-1017`，Chat Completions 响应由回环夹具提供。
网关探针使用受管 ACP、宿主文件读取与实际 stdio MCP。MCP 仍使用受审的 macOS
沙箱及宿主逐次审批，夹具从专用 FIFO 读取随机值，沙箱外的写端独立记录读取次数。

## 受管入口

私有冷安装由仓库安装器从固定锁文件安装 56 个 npm 包，退出 0。受管修订 v5 的
`dist/cli.js` SHA-256 为
`55aace3a7b5eb077e8cb6314465f6fb965d8ee844d134e7f361bbca6f5396d4c`，
`dist/shiju-skills.mjs` 为
`ede7dd841c8817bf931a245aa94d992289249fea242a0cb21ead7d12e72365bf`。
安装器校验了供应包完整性及受管文件摘要。原 MCP sandbox 和工具守卫摘要与 v4
相同。守卫测试覆盖审批、取消、宿主撤销、未审工具拒绝及文件操作失败。

`session/new._meta["shiju.skillMounts"]` 传入租户会话视图里的选中目录。DeepAgents
原生 `SkillsMiddleware` 按其规则扫描视图及项目根下的 `.deepagents/skills`、
`skills`，原生来源按上游最后来源优先规则处理同名技能。显式配置的原生目录
保留替换默认目录的语义，范围限工作区。扫描阶段通过宿主读取
`SKILL.md` 并提取目录元数据，首次模型请求只有声明和路径。正文与附件由原生
`read_file` 通过原权限守卫后读取。挂载要求受审的 `session_model` 启动路径，视图仅为当前选集开放读取。
直接 ACP 四轮对照使用隔离客户端回调，四轮均返回 `end_turn`，并验证随机标记及
stdio 调用增量 0/1/0/0。调用 ID 见 `target/acp-skill-coexistence/deepagents-direct.json`。同进程
`session/load` 接受原选集并拒绝移除挂载后的选集。

## 修复前

旧网关 SHA-256 `808b01432ed6f30664f849b0130a97a320b952d395aa34c6723e6ec19be2edf8`，
受管 v4 入口 SHA-256
`99235b0c39283be84bef07a2d3beb908533e93d5697e929eac62b450a01a7f52`。
带 `ACP_DEEPAGENTS_V4_BASELINE=1` 运行相同验收入口，基线获得 1 个
`chat.done`、0 个 `chat.error`，确认 v4 缺少原生目录，但两次显式文件读取均
按调用 ID 返回。挂载轮无模型请求、无 MCP 调用，返回 1 个 `chat.error`，
原因为 `deepagents: no verified skill directory mount branch`。探针退出 1。
脱敏报告为 `target/acp-skill-coexistence/deepagents-before.json`。

## 网关复验

槽位 3 的 `CARGO_TARGET_DIR` 和 `CARGO_BUILD_BUILD_DIR` 均指向仓库
`target/rust-slot-3`。`cargo build --locked -j 1 -p vw-cli --bin shiju` 退出
101。`acp-agent-registry`、`acp-client` 和 `llm-runtime` 已编译，后续
`task-scheduler` 在 `src/store/postgres/task_plan.rs` 第 247、249 行访问
私有字段 `PlanMember.id`，两处报 E0616。该错误来自任务范围外的现有代码。
v5 的网关四轮、`chat.done`、模型及权限回读待复验。

从仓库根持有 Cargo 槽位执行。编译阶段沿用计划的共享 `cargo` 写锁。
已私下安装的受审制品可通过 `ACP_COEXISTENCE_DEEPAGENTS_RUNTIME` 指定目录。

```sh
python3 scripts/cargo_slot.py sh -c 'cargo build --locked -j 1 -p vw-cli --bin shiju && python3 scripts/verify_acp_skill_coexistence.py --agent deepagents --gateway-bin "$CARGO_TARGET_DIR/debug/shiju" --output target/acp-skill-coexistence/deepagents.json'
```
