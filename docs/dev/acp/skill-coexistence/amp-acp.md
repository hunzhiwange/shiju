# Amp ACP 技能共存验收

任务 23 已阻塞。受管 v4 的首个流式模型请求没有项目原生技能目录。启用 ShiJu 选集时，宿主在推理前拒绝挂载。

## 制品与隔离

macOS 27.0 arm64，Node v24.22.3。固定制品为 `amp-acp@0.12.0`、`@sourcegraph/amp@0.0.1778732631-gc5875c`，安装修订 `amp-acp-0.12.0-model-source-v4`。从已有受审安装复制到独立私有配置目录，核对包版本、`node_modules/.bin/amp-acp` 的解析目标及以下 SHA-256。未修改受管安装。[制品契约](../../../../crates/libs/acp-agent-registry/src/managed_runtime/contracts.json) · [MCP 审计](../mcp-policy-audit/coexistence/amp-acp.md)

| 文件 | SHA-256 |
| --- | --- |
| Amp ACP `dist/index.js` | `d56eae64eb5cf820fd6ea982e8298cedca182a1b52fe104965ee89fa19ed40c9` |
| 控制桥 `dist/shiju-local.mjs` | `8abbb35b9ef50e087f328c1aca06aec25e4878bec902e94213857759eefb8845` |
| Worker 启动器 `dist/amp-cli` | `f3076f4f083423e1215e366e18ec7808a4405e973cfe66d0135b1bd10592a1d5` |
| Worker `dist/main.js` | `e726b67e5a1d6d1332b3726a220e2e619e5408e3922755cfd999d37fe6abd987` |

使用生产网关、Amp Anthropic Messages 中继和既有只读 stdio MCP 夹具。以项目 `.agents/skills` 为原生入口，从 `defaults.skills` 与宿主 MCP 选择生成请求。模型绑定为 `session_model` 和 `coexistence-fixture/deepseek-v4.1-flash-1017`，使用 `full_access=false`。初次冷装在槽位 3 超过 180 秒，未进入网关。正式复跑复用槽位 6 的现有网关，两种 Cargo 缓存路径均为仓库的 `target/rust-slot-6`。网关 SHA-256 为 `8ee6357b62005376c74ef75d26aa03e9379af73f81e9e8b3a63f5552ec768c52`。本次改动限于探针与回执，未重新构建或运行 Rust 单测。

## 实测结果

| 检查 | 新会话 | chat.done / chat.error | 模型流 / MCP 调用 | 结果 |
| --- | --- | --- | --- | --- |
| 正式无挂载 | `ses_e0729d223ffe8f3gC4qono3Sga` | 0 / 1 | 1 / 0 | 首个目录缺少原生技能，探针拒绝请求 |
| 独立普通 Read 对照 | `ses_e072b06a1ffetQYFqE07mkEAcL` | 1 / 0 | 2 / 0 | 原生 `Read` 经宿主读取私有附件，随机值按 `toolu_bc5200100c4fcd15fdc8f6e2` 进入续轮 |
| 独立挂载诊断 | `ses_e072afb01ffey8Fl7dEEDJP3ZG` | 0 / 1 | 0 / 0 | 网关返回 `no verified skill directory mount branch` |

独立诊断仅在无挂载轮以普通 `Read` 验证网关和模型协议。该轮回读了上表所述后端、模型来源及 `full_access=false`，读取无交互审批。挂载轮未产生 ACP 工具调用或审批。正式探针和独立诊断均退出 1。撤除与 MCP 拒绝对照未执行，未取得技能激活、共存或 MCP 计数 0/1/0 的证据。

正式证据在 `target/acp-skill-coexistence/amp-acp.json`，独立诊断与冷装证据分别在同目录的 `amp-acp-diagnostic.json` 和 `amp-acp-cold-install.json`。从仓库根复跑时可令 `SHIJU_AMP_PROBE_RUNTIME` 指向已有受审 v4 安装。未提供该变量时使用受管安装器。

```sh
PYTHONPATH=scripts python3 -c 'import sys; from cargo_slot import run; sys.exit(run(sys.argv[1:], number=6))' \
  sh -c 'python3 scripts/verify_acp_skill_coexistence.py --agent amp-acp --gateway-bin "$CARGO_TARGET_DIR/debug/shiju" --output target/acp-skill-coexistence/amp-acp.json'
```

## 固定版断点

从本机 npm 缓存核对了未打补丁的固定 Worker，SHA-256 为 `458435712c9c4e222822847fc9e70080cdf023ffb3c820e959a30425a22a5836`。原生加载器 `s70` 同时处理工作区及祖先目录、用户技能、插件缓存、toolbox 和 `skills.path`。`skills.path` 只是该扫描器中的附加分支。底层扫描会跟随目录链接，读取技能正文和随附的 MCP 配置，并遍历附件。受管 v4 在扫描器入口返回空目录，所以直接传入 `skills.path` 无法注册选集。工具执行同样有独立边界，桥接白名单未包含 `skill`，不能直接放行原生激活。历史审计验证的是宿主 `Read` 和已选 stdio MCP。[Worker 补丁](../../../../patches/acp-model-source/amp-acp/apply.py) · [控制桥](../../../../patches/acp-model-source/amp-acp/local.mjs)

后续需要可审计的供应商扫描入口或受管补丁，将目录限制在项目原生技能和当次受信视图，约束符号链接与配置读取，并保留激活和附件读取的宿主审批。取得该入口后重跑正式三轮及 MCP 拒绝对照。
