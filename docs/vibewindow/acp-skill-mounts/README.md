# ACP 技能目录入口审查

审查日期：2026-11-05。范围为注册的 22 项固定发行物，平台为 Darwin-arm64。
本任务仅审查发行物、协议接入和静态发现实现，不运行 CLI、模型回合或 Cargo。
[agents.json](agents.json) 为逐项记录；其中未知字段是明确的证据缺口，不是能力承诺。

结论：**15 candidate、7 unverified、0 unsupported、0 verified**。
全部 agent 都需要后续实际 ACP 验收。审查清单完成不等于技能挂载适配完成。

## 状态与证据边界

- `candidate`：固定包/源码内有目录发现或配置实现，可以作为下一步适配候选；不证明隔离、热更新、完整替换或恢复成功。
- `unverified`：只有字符串/当前文档线索，或尚未定位完整调用链。账号、环境和安装缺失都不能转换成 unsupported。
- `unsupported`：必须提供固定发行物明确拒绝或缺失入口的负向证据；本次没有这样的证据，不能据此跳过任何 agent。
- `verified`：留给后续固定 CLI、真实 ACP、按需激活与支持文件/脚本工具回合。本任务不产生此状态。

旧模型来源的 17/22 报告只用于版本定位。每项 `independent_sources` 分别记录本地配置根候选和旧模型报告定位；本次两者均没有重新运行验证，更不能据模型入口受限判定技能不支持。

## 固定发行物逐项入口

以下路径属于发现候选，不能直接写入用户全局目录。带“当前文档”或“字符串”的路径尚未绑定固定 ACP 行为。

| agent | 固定版本 | 状态 | 目录/配置线索 |
| --- | --- | --- | --- |
| auggie | 0.36.0 | candidate | <cwd>/.augment/skills；~/.augment/skills |
| claude | 0.26.0 | candidate | <cwd>/.claude/skills；CLAUDE_CONFIG_DIR/skills；_meta.claudeCode.options.plugins（待 SDK 契约验证） |
| codex | 2.5.0 | candidate | additionalDirectories[i]/.agents/skills；<cwd>/.agents/skills（engine 默认发现待验证） |
| codebuddy | 2.179.3 | unverified | ~/.codebuddy/skills（固定包路径线索） |
| goose | v1.57.0 | candidate | <cwd>/.agents/skills；~/.agents/skills；Paths::config_dir()/skills |
| vibe | 3.0.4 | candidate | config.skill_paths[]；harness project_skills_dirs；harness user_skills_dirs |
| hermes | 0.19.0 | candidate | HERMES_HOME/skills（活动 profile） |
| snow | 0.9.14 | candidate | <cwd>/.agents/skills；<cwd>/.snow/skills；~/.agents/skills；~/.snow/skills；发行物 builtin skills |
| copilot | 1.0.105 | unverified | ~/.copilot/skills（当前官方文档线索）；<repo>/.github/skills（当前官方文档线索） |
| cursor | 2026.10.22-27dcf5f | candidate | .cursor/skills（固定包路径分类）；.agents/skills（固定包路径分类） |
| droid | 1.3.0 | unverified | ~/.factory/skills（当前官方文档）；<repo>/.factory/skills（当前官方文档） |
| gemini | 0.66.0 | candidate | user .gemini/skills；user .agents/skills；<cwd>/.gemini/skills；<cwd>/.agents/skills |
| iflow | 0.5.19 | candidate | Tn()/skills（配置根）；<cwd>/.iflow/skills |
| kilocode | 7.8.18 | unverified | .kilo/skills（当前官方文档） |
| kimi | 2.5.0 | candidate | KIMI_CODE_HOME/skills（根绑定待验证）；<cwd>/.kimi-code/skills；.agents/skills |
| kiro | 3.0.0 | unverified | skill://.kiro/skills/*/SKILL.md（固定二进制字符串）；agent resources 的 skill:// glob（当前文档） |
| opencode | 1.19.5 | candidate | config.skills.paths[]；config.skills.urls[]（网络入口，本计划不使用） |
| openclaw | 2026.10.10 | candidate | config.skills.load.extraDirs[]；workspace/skills；managed/plugin skills（优先级需验收） |
| pi | 0.0.22 | candidate | PI_CODING_AGENT_DIR/skills；<cwd>/.pi/skills；~/.agents/skills |
| qoder | 1.1.82 | unverified | .agents/skills（固定包可读字符串）；.qoder/skills（当前官方文档线索） |
| qwen | 0.26.7 | candidate | Storage.getGlobalQwenDir()/skills；其他 SKILL_PROVIDER_CONFIG_DIRS 下的 skills |
| trae | 0.121.3 | unverified | 未核实：固定 0.121.3 CLI 的技能目录入口 |

## ACP adapter 和发现方式

Codex 实际链路是 `codex-acp 2.5.0 → Codex 0.165.0 app-server`。审查的是 adapter 的原始发布 bundle：`refreshSkills(cwd, additionalRoots)` 将额外工作目录映射为 `<root>/.agents/skills`，调用 `skillsExtraRootsSet`。这不是直接给裸 Codex 增加参数。adapter 还过滤部分不支持的内置技能；清单存在不代表正文激活。

Claude 实际链路是 `claude-agent-acp 0.26.0 → claude-agent-sdk 0.2.96`，SDK 内嵌 CLI 2.1.96。adapter 从 `_meta.claudeCode.options` 取得选项，默认设置 `settingSources = [user, project, local]`，并将 `additionalRoots` 合并到 SDK 的 `additionalDirectories`。额外工作目录能否引出技能发现仍须验证；不能把裸 CLI 技能文档当 adapter 支持证据。SDK 载荷中的 `.claude/skills` 路径单独留有散列与片段。

Pi 同样有两条链：`pi-acp 0.0.22` 的 Skills 资源清单扫描与 Pi 0.73.1 的 `loadSkills(skillPaths, includeDefaults)`。必须证明 adapter 和 engine 使用同一目录视图。

仓库接入通过 RustCodeGraph 核查了 `BUILT_IN_AGENT_DEFINITIONS` 和 `client/protocol.rs::new_session_info`：注册器仍存在 latest/range；创建请求目前显式发送 cwd、mcpServers 和 launch meta，没有通用技能目录挂载字段。固定清单不代表所有实际安装都已固定，也不据上游 schema 冒充本仓库已有标准字段。

## 选择、权限、更新和恢复

发现目录后，通常先列出名称、描述和文件位置，再由工具/模型按需读取正文。选择型默认技能、enabled/disabled 过滤、TUI 导入、插件计数和握手都不能证明按需激活。Snow 的技能工具描述、Hermes 的 `skills_list/skill_view`、Vibe 的过滤与缓存、Qwen 已忽略的 `experimental-skills` 标志在逐项记录中区分。

额外根不自动屏蔽已有用户/项目/内建/插件目录。Goose 显式读取全局目录，Vibe 的 `skill_paths` 为追加合并，OpenClaw 的 `extraDirs` 为低优先级根，都需要解决完整替换与隔离。不能通过替换 HOME、放宽工具权限或直接写全局目录完成挂载。Gemini 项目技能还受 trust 门禁，Qwen 的技能可读根不等同脚本执行授权。

所有条目的权限、原生更新和跨进程恢复都明确标为未验收。Codex refresh、Vibe 重新发现及 OpenClaw watch 只是静态线索，不承诺原子替换。OpenClaw 是 Gateway 桥接，必须验证执行端 Gateway 的视图，而非仅重启桥接子进程。

下一步各 agent 需在固定发行物上运行：目录元数据发现、实际按需激活、支持文件/脚本工具调用、隔离及删除、更新失败保旧、空闲重建/原生更新、跨进程恢复。只允许替代外部模型 HTTP，不能用 mock agent、旧运行报告、零测试或握手结果替代。

## 追溯和复查

JSON 中每个证据项含文件 SHA-256、大小、来源路径和定位片段；源码另带行号，二进制片段以字节偏移定位。二进制字符串只说明符号存在，不能据此确定权限或 ACP 可用性。版本清单、managed contracts 和注册器也保存了本次读取的散列，未修改旧模型报告。

`target/...` 是本机保留的隔离发行物，不是仓库跟踪文件。`${GLOBAL_NODE_MODULES}` 指本机只读全局 npm 包根，`${REVIEW_CACHE}` 指审查缓存。证据路径缺失时，使用每项固定包/发行物 URL 重新获取到隔离目录并核对散列；不得安装 latest 或运行安装脚本。缓存路径命名本身不证明发行物身份，JSON 对未做二进制运行身份验证的项有明确说明。

本次新检查：22 项无遗漏无重复；14 个 package.json 的版本匹配；Kimi 2.5.0 和 Droid 1.3.0 的本机二进制与清单中的发布校验值匹配。其他 native 发行物的运行身份未重新验证。没有读取用户账号配置、模型日志或 MCP 载荷。

不透明发行物另查当前官方文档：[Copilot](https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/add-skills)、[Kilo](https://kilo.ai/docs/customize/skills)、[Kiro](https://kiro.dev/docs/cli/skills/)、[Droid](https://docs.factory.com/harness/skills)、[Qoder](https://docs.qoder.com/cli/Skills)。这些文档不绑定固定版本，不能提升状态。TRAE 检索得到的 [IDE 技能文档](https://docs.trae.cn/ide_skills?lang=zh) 不适用于固定 CLI 0.121.3，故未据此填写目录契约。

## 本任务验证

实际执行 `python3 -m json.tool docs/vibewindow/acp-skill-mounts/agents.json` 一次，退出 0。
另执行独立轻量 Python 检查一次：严格 22 项 key 集合、无重复、版本与证据非空、状态与必填审查字段、证据 SHA-256 格式及当前本机文件回读一致性，全部通过。
这是文档/发行物审查，没有运行测试套件，不将检查数写成真实 CLI 验收通过数；未调用 Cargo 或领取构建槽位。

## ShiJu 会话挂载扩展 v1

共享类型由 `vw-acp` 导出。此处定义自有协议，不修改 ACP 标准字段，不向其他 agent 强发自家扩展；外部 agent 仍须逐项核实其入口。

initialize 响应中的 `_meta["shiju.mounts"]`：

```json
{"version":1,"skills":false,"setMounts":false}
```

两项能力默认关闭；仅服务端接通技能目录后声明 `skills:true`，任务 7 接通原子更新后才允许 `setMounts:true`。客户端缺失声明、未知版本或 `setMounts:false` 时不得发送更新。`SetSessionMountsRequest::to_ext_request` 强制检查此门禁。

session/new 与 session/load 的 `_meta["shiju.skillMounts"]`：

```json
{"version":1,"skills":[{"name":"demo","path":"/absolute/session-view/demo"}]}
```

`skills` 必填，禁止正文、调用参数和未知字段。new 缺省命名空间等同空集合；load 缺省则恢复已保存目录描述，显式 `skills:[]` 清空，`null` 无效。`SessionMountSelection::from_meta` 保留缺省与空集合的区别；`resolve` 在 new 传入无保存集合，在 load 传入保存集合。名称保留已有 Unicode/下划线标识，拒绝空名称、控制字符、路径分隔符及 `..`；同集合名称必须唯一。路径必须是执行端操作系统的绝对路径，拒绝控制字符和 `.`/`..` 分量。类型只校验形状，不证明目录存在、可读或安全；执行端仍须按租户/项目/会话绑定、canonicalize 和重新审核，不扩大权限。

完整替换的原始 wire 方法为 `_shiju/session/set_mounts`，参数：

```json
{"sessionId":"session-1","expectedRevision":3,"skills":[],"mcpServers":[]}
```

SDK 0.10.4 的 `ext_method` 自动添加前导 `_`；传入 `SET_SESSION_MOUNTS_METHOD`（`shiju/session/set_mounts`），避免双下划线。两数组必须完整重传，遗漏或 null 均拒绝；显式空数组清空该集合。`mcpServers` 沿用 SDK 标准 `McpServer` 类型，不引入自定义 MCP 结构或 TCP 端口。服务端比较 `expectedRevision` 与当前 revision；冲突或计数溢出明确拒绝。仅所有新技能与 MCP 资源准备并原子发布成功后推进 revision，失败保持旧集合；运行中请求不得被打断。

更新响应以及 new/load 响应 `_meta["shiju.mounts"]` 的生效快照：

```json
{"revision":4,"skills":["demo"],"mcpServers":["fixture"]}
```

响应仅包含实际生效名称，不含路径、凭据或完整 MCP 配置。`SessionMountRevision` 使用非负 u64；`next` 校验冲突和溢出，不负责提前发布状态。标准 new/load 的 `mcpServers` 每次由调用方完整提供，持久化仅保存技能目录描述及必要身份/revision，不保存 MCP 凭据用于恢复。

协议类型与契约测试不等同服务端挂载、真实 CLI 技能发现或 MCP 执行验收；这些留给后续任务。

### 协议任务验证（2026-11-05）

- 红灯：`cargo test -p vw-acp --lib session_mounts_contract_ -- --nocapture --test-threads=1`，5 个测试中 3 失败、2 通过，退出 101；失败分别为未知版本、非法路径、重复名称。槽位 3，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-3`。构建目录配置检查后重跑仍得到同一红灯结果。
- 绿灯：先执行 `cargo fmt --all`（退出 0），再执行同一聚焦命令，9 通过、0 失败、0 ignored，退出 0；最终补齐路径形状断言和 Windows fixture 后再次执行，仍为 9 通过。两轮绿灯使用槽位 2，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。
- `cargo clippy --all-targets --all-features -- -D warnings` 在两轮绿灯后均退出 0。中间一次实现编译因 SDK 非穷尽 MCP 枚举缺少拒绝分支而失败，已补明确错误；该次零测试不计行为通过。
- 因仓库设置了 `[build].build-dir="target"`，最终验证同时设置 `CARGO_BUILD_BUILD_DIR="$CARGO_TARGET_DIR"`，避免中间产物绕出槽位。每阶段通过原子 mkdir 动态领槽并记录 PID；结束后释放自己的锁，保留缓存。没有执行全量测试、wasm 检查或 Makefile。
- 证据覆盖严格解析、wire 往返、缺省/清空、revision 冲突与溢出、未声明能力时不生成扩展请求。此处有效行为测试数为 9，不计为真实 CLI 验收。
