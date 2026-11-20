# Snow 会话技能与 MCP 共存验收

2026-11-20，Darwin arm64，固定 `snow-ai@0.9.14`，入口 `bundle/cli.mjs --acp`。npm 缓存包的 SHA-512 与受管合约一致。上游入口 SHA-256 为 `bcc0a6cf468a245729605bcf75fd762fd13cb48553d342b31121cafc3153d905`。受管 v4 入口自报 0.9.14，SHA-256 为 `b3fb29de76fe1e89ab229b0954bca728589948940e2f203d0ca7ab56ea8702dd`，通过固定补丁的摘要校验和 `node --check`。

## 基线与修复

受管 v3 入口 SHA-256 为 `2a38b3678070cf5d47892ad0c8c43429090b73e5a4000cb3d92edaa5478c4a77`。旧网关与 v3 的探针退出 1。首次模型请求已到达，首个差异为 `unexpected native skill catalog`。无选集时宿主传入空目录值，v3 的独占分支随即返回空目录。基线报告位于 `target/acp-skill-coexistence/snow-baseline.json`。 在更新合约前，另一份旧网关也先于模型请求拒绝了 v4 入口，报告分类为 `reviewed_runtime`。

v4 沿既有顺序扫描内置、用户和项目技能，再从宿主校验的会话视图追加新 ID。原生同名 ID 保留优先级，项目 `disabledSkills` 仍由 Snow 应用于发现和激活。空选集继续扫描原生目录。挂载 stdio MCP 的工具冲突检查、Chat 路由、审批及下游调用路径沿用受审实现。独立原生 ACP 对照在投影中加入与项目技能同名的条目，实际激活及读取命中项目源。原生 ACP 的放行和拒绝对照各完成五个关联调用，stdio 下游执行分别为一次和零次。[原 MCP 验收](../mcp-acceptance/snow.md)

探针由生产网关解析 `defaults.skills` 与 `defaults.mcp`，沿 `session_model` 选中 `coexistence-fixture/deepseek-v4.1-flash-1017`。供应商模型响应由私有回环 Chat Completions 服务提供。探针对实际 `skill-execute` 目录发起激活，从结果的绝对路径使用 `filesystem-read` 读取附件，再以原生 stdio 路径调用只读 MCP。正文、附件和 MCP 随机值均按调用 ID 从模型续轮验证。第一条模型请求检查内容无提前展开，目录检查涵盖未选中技能、显式禁用的原生技能以及撤除后的旧视图。

## 生产网关复验

`cargo build --locked -j 1 -p vw-cli --bin shiju` 通过仓库槽位入口串行执行，构建和探针均退出 0。`CARGO_TARGET_DIR` 与 `CARGO_BUILD_BUILD_DIR` 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-3`，探针结束后释放槽位。网关 SHA-256 为 `403bb2827df94239bc831633f3cd3a1d10dd832594fe9d8471cc1c8bde2ebacf`，四轮回执位于 `target/acp-skill-coexistence/snow.json`。

在仓库根可用已安装的受管 v4，或以 `SNOW_COEXISTENCE_RUNTIME` 指向通过合约校验的 v4 安装目录。

```sh
python3 scripts/cargo_slot.py sh -c 'cargo build --locked -j 1 -p vw-cli --bin shiju && python3 scripts/verify_acp_skill_coexistence.py --agent snow --gateway-bin "$CARGO_TARGET_DIR/debug/shiju" --output target/acp-skill-coexistence/snow.json'
```

| 轮次 | 新会话 | 模型请求 | `chat.done` / `chat.error` | MCP 审批 | 下游调用 |
| --- | --- | ---: | ---: | --- | ---: |
| 基线 | `ses_e079fdd67ffev2Wt42p5uFhrDS` | 3 | 1 / 0 | 无 | 0 |
| 挂载 | `ses_e079fd06cffe8vQzf7id2TtFQt` | 6 | 1 / 0 | 一次放行 | 1 |
| 撤除 | `ses_e079fbfc4ffeck2hjVsuPOEXLj` | 3 | 1 / 0 | 无 | 0 |
| 拒绝 | `ses_e079fb49cffeKl0tuPGsYPq1I5` | 6 | 1 / 0 | 拒绝 | 0 |

挂载轮的原生技能激活与附件读取对应 `call_c3582fba9acf529f846df330`、`call_1c1e24a9bacdf5adad3fea3b`，选中技能激活与会话视图附件读取对应 `call_4e0fb5089ec163c8f23ccf65`、`call_13dc4e1fe107908b7e931ca7`。批准的 MCP 返回以 `call_08688e7a6ea276bad4d47dc3` 到达模型。拒绝轮以 `call_c852941e64283bdf440900e2` 收到拒绝结果，审计记录零次下游调用。

四轮回读后端 `snow`、指定模型与 `session_model`，访问设置为 `full_access=false` 和 `requested=false`，权限支持状态为 `unavailable`。原生技能激活及附件读取分别按 Snow 请求单次批准。撤除后原生技能和附件仍可读取，旧会话视图与 MCP 均未出现在模型目录。

共享 Python 验收判定运行 5 项，Snow 旧夹具的绝对路径读取检查运行 1 项，均通过。本次验收覆盖 Darwin arm64、Snow 0.9.14 v4、Chat Completions、`session_model` 与 stdio。旧生命周期套件、`cli_local`、其他协议和平台未复验。Rust 技能配置与能力门禁无行为改动，未另运行 Rust 单测或构建桌面客户端。
