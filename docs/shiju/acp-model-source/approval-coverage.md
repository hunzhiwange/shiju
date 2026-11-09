# ACP 双档权限验收

范围为生产 `/v1/global/config/acp/defaults` 返回的 22 个后端。宿主协议矩阵另外核对兼容别名、旧命令和同名命令覆盖。真实适配器证据来自 Darwin arm64 和临时项目。

宿主接到 `fs/write_text_file` 时，会核对当前会话、工作区和文件边界，再执行普通文件编辑。未明确授予完全访问时，单独传入 `ApproveAll` 会回落为普通审批。工具和终端回调同样需要当前授权，宿主批准只选择单次许可。

| 后端 | 固定版本 | 模型来源 | 控制与覆盖 |
| --- | --- | --- | --- |
| auggie | 0.36.0 | 原生 | 缺已安装固定版及隔离模型配置 |
| claude | 0.26.0，SDK 0.2.96，CLI 2.1.96 | 两种 | 两种来源均实测，原生写入走单次确认 |
| codebuddy | 2.179.3 | 两种 | 缺已安装固定版 |
| codex | 2.5.0，CLI 0.165.0 | 两种 | 两种来源均实测，审批档的原生写入与命令由只读模式拒绝 |
| copilot | 1.0.105 | 两种 | 两种来源均实测，敏感操作由宿主单次审批处理 |
| cursor | 2026.10.22-27dcf5f | 原生 | 缺已安装固定版及隔离模型配置 |
| droid | 1.3.0 | 原生 | 缺已安装固定版及隔离模型配置 |
| gemini | 0.66.0 | 两种 | 两种来源均实测，维持 `default`，宿主单次审批与原生工作区限制共同生效 |
| goose | 1.57.0 | 两种 | 缺已安装固定版 |
| hermes | 0.19.0 | 两种 | 缺已安装固定版 |
| iflow | 0.5.19 | 两种 | 缺已安装固定版 |
| kilocode | 7.8.18 | 两种 | 缺已安装固定版 |
| kimi | 2.5.0 | 两种 | 缺已安装固定版 |
| kiro | 3.0.0 | 两种 | 缺已安装固定版 |
| openclaw | 2026.10.10 | 两种 | 缺已安装固定版 |
| opencode | 1.19.5 | 两种 | 两种来源均实测，桌面原生执行要求当前完全访问授权 |
| pi | 0.0.22，CLI 0.73.1 | 两种 | 找到的适配器未通过固定版补丁身份检查，缺已核验的适配器与引擎组合 |
| qoder | 1.1.82 | 原生 | 缺已安装固定版及隔离模型配置 |
| qwen | 0.26.7 | 两种 | 缺已安装固定版 |
| snow | 0.9.14 | 两种 | 缺已安装固定版 |
| trae | 0.121.3 | 原生 | 缺已安装固定版及隔离模型配置 |
| vibe | 3.0.4 | 两种 | 缺已安装固定版 |

“两种”指智能体原生模型和客户端模型。五个仅支持原生模型的客户端路径标记为不支持。版本与离线配置取自[固定版本清单](cli-versions.json)。

Claude 通过桌面网关先后接收两次工作区写入、越界写入、命令和删除。审批档只对两次工作区写入答复单次允许，两次均落盘。完全访问期间五项操作均生效。撤销后的五次新请求均被拒绝。原生模型路径的模式回读为 `default → bypassPermissions → default`。客户端模型在轮次结束后释放进程，证据来自审批事件与实际文件状态。固定版 ACP 回退测试另行核对原生工具请求和组织拒绝规则。

Codex 的 `agent` 与 `workspace-write` 可绕过宿主文件回调写入系统临时目录。桌面审批档改用 `read-only`，恢复和显式设置也遵守这一限制。两种模型来源都观测到原生审批档拒绝五项操作、完全访问允许操作、撤销后再次拒绝。原生模型路径回读为 `read-only → agent-full-access → read-only`。

Copilot 的两种模型来源都通过单次批准后执行工作区写入命令，敏感操作在审批档与撤销后被拒绝。Gemini 的两种模型来源都在审批档确认并完成两次工作区覆盖写入。完全访问允许工作区编辑及测试命令，原生工作区限制继续拒绝越界 Write。Gemini 客户端模型验收使用原生 Google 协议中继。

OpenCode 实测在审批档直接执行了创建标记和删除文件的 Bash 命令。全局权限环境变量之后仍会叠加代理专属规则，无法用全局覆盖约束任意现有配置。桌面审批档在启动前拒绝这条原生执行路径。 [固定版配置合并](https://raw.githubusercontent.com/anomalyco/opencode/v1.19.5/packages/opencode/src/config/config.ts) · [固定版代理权限规则](https://raw.githubusercontent.com/anomalyco/opencode/v1.19.5/packages/opencode/src/agent/agent.ts)

同名 Codex 替换夹具只通过 ACP 回调操作文件。网关验证了无需批准的工作区创建和编辑、完全访问后允许越界写入、撤销并加载后恢复拒绝。两个并行工作区使用相同的远端会话 ID，同时等待独立审批，网关拒绝六次跨工作区答复和六次重复答复。未知自定义 key 由宿主协议矩阵覆盖，网关的智能体配置入口会拒绝该 key。

## 复现

按编号任务的共享槽位规则设置 `CARGO_TARGET_DIR` 和 `CARGO_BUILD_BUILD_DIR`。将已安装并核验的固定版适配器加入 `PATH`。验收网关与模型端点只使用回环地址，运行时使用隔离配置和临时工作区。

```sh
cargo test -p vw-acp --lib acp_desktop_approval_backend_matrix -- --test-threads=1
cargo test -p vw-acp --lib acp_full_access_ -- --test-threads=1
cargo build -p vw-cli --bin shiju
python3 -m unittest discover -s scripts -p acp_approval_model_tests.py
python3 scripts/verify_acp_approval.py --gateway "$CARGO_TARGET_DIR/debug/shiju" --output target/acp-approval-evidence/matrix.json
```

JSON 分别记录宿主夹具与真实适配器结果。本轮五个固定版后端共十条实测通过，五条客户端模型路径为生产不支持，另有二十九条缺少固定运行时或核验过的依赖组合。真实行缺少运行时或验收失败时，全矩阵以非零状态退出。使用 `--agent <key>` 可单独重跑某个供应商，该运行不作为完整矩阵。
