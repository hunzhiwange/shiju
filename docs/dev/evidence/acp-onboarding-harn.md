# Harn 自然语言与指定模型接入证据

日期：2026-11-18。范围为Darwin arm64的内置acp-harn、指定会话模型和真实Rue.js编辑器文件交付；其他平台与上游能力对等不在本次认证范围。

## 来源与固定版本

- [官方v0.10.201发行](https://github.com/burin-labs/harn/releases/tag/v0.10.201)，Darwin arm64 gzip包SHA-256：`332a7a94162e8c9d3345bbc34fcc6e1043e952b707f1ba44ba79fceb2c21a414`，与GitHub release asset digest一致。
- [原生ACP执行实现](https://github.com/burin-labs/harn/blob/v0.10.201/crates/harn-serve/src/adapters/acp/prompt.rs)：无文件模式把提示视为Harn程序，因此使用本仓库静态pipeline消费prompt并调用官方agent_loop。
- [供应商overlay与工具协议](https://github.com/burin-labs/harn/blob/v0.10.201/docs/src/llm/providers.md)、[宿主工具桥](https://github.com/burin-labs/harn/blob/v0.10.201/docs/src/bridge/host-tools.md)。

## 实现与权限边界

管理安装走现有系统端点，校验官方包及三个运行文件，保留许可证，跳过两个已知的非ACP别名。自然语言程序、传输适配、测试和契约位于 `patches/acp-model-source/harn/` 与 `acp-agent-registry/src/managed_runtime/`。

launcher仅转发JSON-RPC、约束环境并把read_file/write_file转成标准ACP宿主文件请求；不执行推理或直接读写用户文件。实际推理和agent循环仍由官方原生Harn运行。明确拒绝未知宿主工具；宿主拒绝写入时文件不会落盘。源码中的直接harness.fs写入已替换，避免绕过宿主审批。

模型驱动使用现有绑定relay和原provider/model，要求工具能力和私有回环端点，写入0700目录内的0600配置及pipeline；拒绝启动/环境/项目供应商覆写。relay令牌只通过in_process grant供模型调用使用，子命令和MCP不继承。选模使用Harn支持的session/set_config_option，并要求精确回读，无旧接口失败回退。full_access只报告client_only，不声称原生免审批。

## 本次实机结果

|项目|结果|
|---|---|
|系统安装/发现|POST系统安装200，管理入口摘要通过；官方原生引擎0.10.201|
|普通诊断|HTTP200，authentication_required，未当作诊断通过|
|内置预设|acp-harn、builtin=true、backend=harn；启用保存重读，结束恢复disabled|
|指定模型|session_model；alibaba-token-plan-cn/deepseek-v4.1-flash-1017；执行中connected=true|
|真实文件|Harn工具创建6项页面/源码/配置/README，后续由同一后端补全和修正；未运行网页构建或浏览器|
|成功终态|补全会话ses_1eb8e9a8c001trQR138CGlci3R及说明修正会话ses_1eb937736001SbYKa1LXS8gE5L均chat.done、无chat.error|
|原请求及恢复|原请求超时后同请求ID续传，最终预算耗尽；提高轮次预算后显式新会话补全已有产物。跨网关重启的旧原生会话恢复未通过，未用新会话冒充恢复|
|目录/清理|6297→6303项，最终仅目标6文件变化；自有运行缓存保留到私有证据，所有自有网关退出、端口关闭|
|配置恢复|有效预设与最初基线相同；最初无显式条目，API禁止删除builtin，因此保留disabled覆写。最终各补全阶段完整配置与会前一致|
|其他原生功能|取消、MCP、技能及跨平台能力未在本次作对等认证；管理runtime的CLI-local来源明确不支持|

## 验证

通过7项Rust聚焦测试和6项Python/原生测试。覆盖安装摘要与档案链接拒绝、自然语言进入模型边界、原模型线协议、宿主拒绝、授权写入、超过40轮读取后完成写入，以及选模确认与错误回读拒绝。红绿日志均在私有证据目录。

使用 `scripts/cargo_slot.py` 的槽位缓存执行受影响目标，最终CLI SHA-256为 `245d9857ee0005fba1936d1a8bea31719406ef6c550bfe0cc53cb46115ce130e`；构建为共享工作区加本任务实现，不是clean HEAD声明。acp-client的--no-deps Clippy通过；其他受影响库Clippy遇到既有agent-log/llm-runtime诊断，未宣称通过。没有整仓测试、桌面客户端编译或页面回归。

完整请求、SSE、失败及续传、成功绑定、SHA-256快照和清理证据位于本机0700目录 `/tmp/shiju-harn-complete-20261118/` 与 `/tmp/shiju-harn-retry-20261118/`。仓库不收录原始敏感载荷或供应商二进制。详细过程与限制见 `docs/dev/acp/ACP验收.md` 的本次追加记录。

复跑入口（原生测试需先自行指定已核验的官方Harn路径）：

```sh
python3 scripts/cargo_slot.py cargo test -p acp-agent-registry -p config-acp-install -p llm-runtime --lib harn_ -- --nocapture
python3 scripts/cargo_slot.py cargo test -p acp-client --lib harn_model_selection_ -- --nocapture
python3 patches/acp-model-source/harn/installer_test.py
HARN_TEST_BINARY=/absolute/path/to/verified/harn python3 patches/acp-model-source/harn/pipeline_test.py
HARN_TEST_BINARY=/absolute/path/to/verified/harn python3 patches/acp-model-source/harn/host_bridge_test.py
python3 scripts/cargo_slot.py cargo clippy -p acp-client --lib --no-deps -- -D warnings
```
