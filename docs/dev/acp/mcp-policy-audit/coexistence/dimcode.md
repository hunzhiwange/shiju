# DimCode 官方工具与 MCP 共存复验

2026-11-19，Darwin arm64 上的受管 DimCode 0.5.28 通过直接 ACP 的 HTTP 和 SSE 共存三轮。回环模型使用 Chat Completions，服务用独立附件提供只读 `read_probe`。每轮另写随机文件标记及附件标记。私有 HOME、工作区和模型配置由探针创建并清理。

## 制品与红绿证据

- 原版 `dimcode acp` 的 SHA-256 为 `0d6298cc8881b15ae8cdf1901b6f39674b74dbf5c14caedadfe509ab5ae114b0`。同一探针对固定原版在两种传输下均退出 0。
- 旧受管 v1 的 SHA-256 为 `89a2f42669235390c13036f3fd63f6c454381bb8578b2f06f5fe1d659e2b170b`。HTTP 会话 `sess_1795071715173_inaua06xi3p`、SSE 会话 `sess_1795071717483_xmx3zf02z0c` 的挂载目录均缺官方 `read`。`call_mounted_read` 返回 `Outside MCP scope` 并标记为 `failed`。`call_mounted_coexistence__read_probe` 获准，服务实收 RPC ID 2 并正常回传。两轮均以 `end_turn` 结束，进程退出 0，共存断言退出 1。
- 新入口为 `acp-runtime/dimcode-0.5.28-model-source-v2/dim acp`，SHA-256 为 `ca470dad0e7663b669769ff8d22bc260339eb46fbff08b9dd7fc553ea225f746`。安装暂存脚本核对原始程序摘要后逐字节复制上游内容，再由安装器签名并核对产物摘要。三处旧补丁全部移除，原生 `PreToolUse` 和 `tools` 元数据筛选直接沿用固定原版。宿主的权限同步、逐服务名单拒绝、stdio 拒绝及受审入口门禁沿用现有实现。
- 官方归档冷下载三次均以 curl 28 超时。用本机已有归档复验安装，归档 SHA-256 为 `71634686a50bf9510862ec18a8381c49f5caf8df510c67085f2efac1773f04de`。私有安装脚本副本仅将下载替换为已缓存文件，后续解包、校验、签名、入口摘要检查和 `dim --version` 均退出 0。错误输入被源摘要校验拒绝，退出 1，未生成目标。冷下载与真实供应商模型尚无本轮通过证据。

## 新制品三轮

三轮均使用 `workspace-write`，模型只发读请求，宿主对 MCP 选择允许一次。官方工具由 DimCode 读取工作区内 `probe.txt`，MCP 工具由回环服务读取工作区外附件。下列成功调用在 ACP 工具更新中均为 `completed`，并以同一调用 ID 进入后续模型请求。挂载轮的服务审计各含一次 `initialize`、`tools/list` 及 `tools/call`。所有轮次的 `session/prompt` 均返回 `end_turn`，ACP 进程及探针退出 0。

| 传输与轮次 | ACP 会话 | 官方 `read` 的调用 ID 与文件标记 | MCP `coexistence__read_probe` 的调用 ID 与附件标记 |
| --- | --- | --- | --- |
| HTTP 无挂载 | `sess_1795071719532_jqzrnl8hm8c` | `call_before_read`，`native-ed704e70e40147f3b8194bbcacb2e16c` | 模型目录无该工具，服务零请求 |
| HTTP 挂载 | `sess_1795071720037_7uszuhh4wbw` | `call_mounted_read`，`native-63b144520f824e00a33ecab7171f6464` | `call_mounted_coexistence__read_probe`，RPC 2，`mcp-d6613a632ee6431eb021c48303e9193f` |
| HTTP 撤除 | `sess_1795071720419_1mj7q5xnr1ah` | `call_removed_read`，`native-d977abb4959c435ba8684c90b3e05422` | 模型目录无该工具，服务零请求 |
| SSE 无挂载 | `sess_1795071726960_53wvcu9uowe` | `call_before_read`，`native-93539187b8d84c54b6d961237f740de8` | 模型目录无该工具，服务零请求 |
| SSE 挂载 | `sess_1795071727409_al68ytz7i4c` | `call_mounted_read`，`native-8a2127b3a0dd4f0195f4084a392c7aaf` | `call_mounted_coexistence__read_probe`，RPC 2，`mcp-9376b97b475342b98e91646be83d2b55` |
| SSE 撤除 | `sess_1795071727787_n6w29mam4cm` | `call_removed_read`，`native-ee9a7cd743104ca08973693ba795f503` | 模型目录无该工具，服务零请求 |

## 拒绝、恢复与宿主测试

- 两种传输均在独立会话取消 `call_cancel_coexistence__read_probe` 的 ACP 审批，模型续轮收到 `Permission request cancelled`。另一轮用 `read-only` 调用 `call_deny_coexistence__read_probe`，模型收到 `Tool call denied: source: default`，无审批回调。两类调用均为 `failed`，服务审计只有初始化和目录读取，零 `tools/call`。
- 对两条挂载会话分别关闭 ACP 进程的输入并等其正常退出。在独立的新进程使用原会话 ID 执行 `session/load`、`session/resume`。传入目标服务时，`call_loaded_read` 和 `call_resumed_read` 均读取新文件标记，对应的 `call_loaded_coexistence__read_probe`、`call_resumed_coexistence__read_probe` 各按原 ID 回传新附件标记。传入空列表时，`call_loaded-withdrawn_read`、`call_resumed-withdrawn_read` 仍完成，MCP 目录缺该工具且服务零请求。强制终止的首次试跑遇到未释放的上游会话租约，正常 EOF 重连通过。
- 修复前后均运行 `python3 scripts/cargo_slot.py cargo test -j 1 -p vw-acp --test onboarding_dimcode acp_onboarding_dimcode_`，均退出 0，4 项通过。含注册、生命周期及取消、拒写和 stdio 拒绝。槽位 3 的 `CARGO_TARGET_DIR` 和 `CARGO_BUILD_BUILD_DIR` 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-3`。当前宿主安装及网关链路未做联调，直接 ACP 的传输与恢复证据和 Rust 宿主门禁分别记录。

复验命令为 `python3 crates/vw-acp/tests/fixtures/dimcode_coexistence.py /path/to/reviewed/dim`。原版、旧版及新版的探针日志保存在本机私有目录 `/tmp/shiju-dimcode-coexist.QJZnzgaJ`。
