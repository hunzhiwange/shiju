# CodeBuddy 会话技能与 MCP 共存验收

2026-11-20，Darwin arm64，固定 `@tencent-ai/codebuddy-code@2.179.3`，受管入口 `bin/codebuddy --acp`。原始 npm 包 SHA-512 与受管合约一致，入口 SHA-256 为 `c8f736a33584a55d9d4c4cf5ac9785f582c5025e384034c73fb4a840901f2907`，入口自报 2.179.3。

| 受管 v2 文件 | SHA-256 |
| --- | --- |
| `dist/codebuddy-headless.js` | `fa53e4207ccd339b4235759270e3db530ecc4379f64bf040df0a2d7fcce00c9f` |
| `dist/codebuddy.js` | `f9bdd59643067ffe91c70d52ccc058221556363556fef2ce8495429d0dbbe6c8` |
| `dist-server/codebuddy.js` | `60dd275cda7026b50a036e371616fb6e8dd7324ba92c423abeed89192ebf58e2` |

## 基线与修复

v1 生产网关回环退出 1。首次模型请求已抵达，首个失败为 `unexpected native skill catalog`。独占式扫描在读取项目技能目录前返回。记录位于 `target/acp-skill-coexistence/codebuddy-baseline.json`。

v2 沿用 CodeBuddy 的项目、用户、连接器和内置发现，在原生扫描完成后向同一去重器追加宿主验证的会话视图。配置和插件合并交给供应商实现。无选集时追加零目录。宿主继续校验选中视图与受管制品，保留原有访问模式与受管内置命令限制。三个固定 JS 入口均通过 `node --check`。

仅替换制品后，旧启动目录仍会遗漏项目技能，生产网关复验再次退出 1，见 `target/acp-skill-coexistence/codebuddy-v2-before-workspace-launch.json`。直接 ACP 对照固定 `session/new.cwd`，分别从项目目录和网关目录启动，在 `user` 与 `none` 两种设置源下都只有前者能发现项目技能。宿主现在用绑定的项目目录启动 CodeBuddy，以便初始化时使用正确的项目根。

验证在独立回环网关以 `session_model` 绑定 `coexistence-fixture/deepseek-v4.1-flash-1017`，只替换模型响应。技能与 MCP 分别由 `defaults.skills`、`defaults.mcp` 选择。探针按实际 OpenAI Chat Completions 工具结构驱动 `Skill`、`Read`、`ToolSearch` 和 `DeferExecuteTool`，用新会话和调用 ID 校验目录、正文、附件与已批准的 MCP 返回。启动与 stdio 传输沿用现有验证路径。[MCP 验收](../mcp-acceptance/codebuddy.md)

插件对照单独使用同一固定 ACP 入口和私有用户设置进行，因为 `session_model` 启动参数关闭供应商设置层。追加前后，启用的 inline 插件技能均可发现，显式禁用的 inline 插件技能均不可发现。

## 生产网关复验

槽位 3 持有构建和探针，`CARGO_TARGET_DIR` 与 `CARGO_BUILD_BUILD_DIR` 均指向仓库内的 `target/rust-slot-3`。定向网关构建与最终探针均退出 0。网关 SHA-256 为 `92a1bcb04e026d7595e756afd2fba3f4f4b01929fad223d13d782cc59e8d10c3`。最终结构化回执位于 `target/acp-skill-coexistence/codebuddy.json`。探针复用该槽位已有的网关产物。以下命令也可从源码复验。

```sh
export CODEBUDDY_COEXISTENCE_RUNTIME="$PWD/target/acp-skill-coexistence/codebuddy-source/runtime"
python3 scripts/cargo_slot.py sh -c 'cargo build --locked -j 1 -p vw-cli --bin shiju && python3 scripts/verify_acp_skill_coexistence.py --agent codebuddy --gateway-bin "$CARGO_TARGET_DIR/debug/shiju" --output target/acp-skill-coexistence/codebuddy.json'
```

| 轮次 | 新会话 | 模型请求 | `chat.done` / `chat.error` | MCP 审批 | 下游调用 |
| --- | --- | ---: | ---: | --- | ---: |
| 基线 | `ses_e07b4c78bffeO4gygQHsBlXG9W` | 4 | 1 / 0 | 无 | 0 |
| 挂载 | `ses_e07b4ae7dffeglJ1F6vY73qqqB` | 7 | 1 / 0 | 一次放行 | 1 |
| 撤除 | `ses_e07b49390ffe0Qft7J6hLWmChx` | 4 | 1 / 0 | 无 | 0 |
| 拒绝 | `ses_e07b459a7ffe9IOYLiBg5DUypD` | 6 | 1 / 0 | 拒绝 | 0 |

挂载轮原生技能激活和附件读取分别为 `call_af9bf94f79a54eb871d14279` 和 `call_66c4c96de00c009ae38b5629`。选中技能激活和会话视图附件读取分别为 `call_0f73317313c6b69f034af4dd` 和 `call_ea718326035e066fc10dcf67`。单次 MCP 放行的结果以 `call_8235aa8d695201820d854ff1` 回到模型。拒绝轮以 `call_511d7b799f7c6caa2b938121` 核对原生工具参数和取消终态。CodeBuddy 在拒绝后结束回合，网关返回 `chat.step_finish` 的 `cancelled` 和一个 `chat.done`。夹具在请求调用时输出一条说明，以保留该取消回合的消息。

四轮均回读 `codebuddy`、`session_model`、指定模型和 `full_access=false`。权限状态为 `requested=false`、`support=unavailable`，对应按工作区安全边界审批。选中技能附件的两次读取各取得单次批准。每轮首个模型请求均无技能正文或附件，未选中技能始终留在目录之外，撤除后仍可读取原生技能。插件对照两轮也通过。

本次覆盖 Darwin arm64 的 `session_model` 与独立原生插件目录对照。其他平台与 v2 的 `cli_local` 生命周期未复验。Rust 技能配置和门禁沿用原实现，启动改动由本次真实网关回环验证。未另运行 Rust 单测或桌面构建。

共享验收判定的 Python 定向测试运行 3 项，全部通过。
