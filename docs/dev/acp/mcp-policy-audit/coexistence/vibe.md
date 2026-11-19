# Vibe 3.0.4 受管工具共存

2026-11-19，Darwin arm64。注册 key 为 `vibe`，入口是 `bin/vibe-acp`，会话模型绑定使用 `--session-model-config`。受管 v3 停止应用旧 MCP 直列补丁，恢复 Core 的程序化发现和 MCP 命名空间。会话绑定、Skill 隔离及原生审批继续由现有实现处理。受管安装目录为 `vibe-3.0.4-model-source-v3`。

复验从已审核的私有 v2 副本开始。复制后逆向应用原安装补丁，校验固定包源文件摘要，再按当前安装钩子应用会话绑定补丁，生成私有 v3。两版均校验各自合约中的七项摘要。入口 SHA-256 同为 `80547aa0b83be7cd1d63536ff041daf93cae8b43fd7c6ee645ff1da67927b7b1`，两版由合约中的运行时和 MCP 宿主摘要区分。本轮使用离线副本，未执行网络冷装。

两版均在 `sandbox-exec` 内以虚拟环境 Python 解释同一个受管入口脚本。沙箱限制派生执行为该 Python 和现有 RustCodeGraph，禁用 Mach 服务、Apple Events、钥匙串文件及其他网络连接，只允许指定回环模型 TCP 端口。MCP 通过 stdio 访问私有 RueJS 已索引副本。夹具启用 Vibe 自带的 `VIBE_TEST_DISABLE_KEYRING=1`，并选择空 Python keyring 后端。试跑时沙箱实际拒绝了 `/usr/bin/security`，随后通过启用 Vibe 自带的测试开关完成红绿复验。全程未启动供应商原包或登录，未更改钥匙串或系统权限。

| 制品 | ACP 会话 | 结果 |
| --- | --- | --- |
| v2 | `318c5bee-1f1d-5426-54ad-2fcb145f9503` | MCP 直列可见，`search_tool_functions` 和 `run_typescript` 缺失。进程退出 0，`end_turn`，共存探针按预期退出 1 |
| v3 | `75b858c8-d53f-022d-1b67-66b5ebfdbad4` | 发现、原生读取和 MCP 查询均完成。进程与探针均退出 0，`end_turn` |

| 工具及执行方 | 模型原调用及续轮 ID | ACP 执行 ID | 续轮结果 |
| --- | --- | --- | --- |
| Vibe Core `search_tool_functions` | `probe-discovery` | `effect-tool-discovery-probe-discovery` | 找到原生读取和挂载查询的声明 |
| Vibe 原生 `file_system.read_file` | `probe-native` | `effect-696fce62-b6f4-5e2b-9cfa-a67e646999fa` | `VIBE_NATIVE_READ_CHALLENGE` |
| RustCodeGraph `rustcodegraph_query` | `probe-mcp` | `effect-725ef016-a747-5dbb-a904-f79cd7ccaa7e` | `createRueSiteApp app/app.tsx:32` |

原生读取和 MCP 调用各由独立的 `run_typescript` 发起。MCP 的 `session/request_permission` 使用同一个 ACP 执行 ID，夹具按原选项批准一次。模型在同一续轮按各自的原 ID 收到真实结果后返回 `PROBE_REPLY`。

用私有受管环境复验。

```sh
"$VIBE_RUNTIME/bin/python" docs/shiju/acp-model-source/evidence/vibe/coexistence_probe.py \
  --entry "$VIBE_RUNTIME/bin/vibe-acp" --graph "$GRAPH_BIN" \
  --project "$INDEXED_RUEJS_FIXTURE" --output "$PRIVATE_RUN/result.json"
```

私有证据目录为 `/private/tmp/shiju-vibe-39.bQdSza`。本轮按限定范围只复验单个挂载会话，名单、撤除、恢复、跨平台、原包对照、历史生命周期和否定用例沿用原有记录与入口。未运行 Cargo。
