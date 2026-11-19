# VT Code 0.183.0 会话 MCP 补丁原型

适用于 VT Code 0.183.0（`4fd8e47a885c`）。在私有源码副本应用补丁，再使用发行版指定的 Rust 1.100.0 构建 CLI。

```sh
git apply --unidiff-zero /absolute/path/vtcode-0.183.0-session-mcp.patch
cargo +1.100.0 build --locked -j 1 -p vtcode --bin vtcode
```

受管安装在 Darwin arm64 接收与 ShiJu 可执行文件同目录的 `vtcode-acp`。打包时使用经过验收且摘要与仓库清单一致的产物。安装将文件复制到 VT Code v2 运行时目录并回读校验。缺少该制品时，仍可下载或使用经校验的原版 v1，用于无挂载会话。

补丁让原生 `session/new` 从带宿主令牌的字面回环 HTTP 地址发现工具，将模型可见目录保存在该会话中。模型发出的调用仍经过 ACP 工具审批和 VT Code 权限检查，使用现有 MCP 客户端执行，沿用原生工具事件和同一模型调用 ID 回传结果。stdio 和 SSE 挂载显式拒绝。带 MCP 的重新挂载、加载和恢复需要新会话。

本地诊断使用和模型共用的沙箱已授权端口转发真实 RustCodeGraph，宿主按工具名及参数消费一次批准。受管路径复用同端口投影，令牌与请求的租户及会话绑定。转发只接受当前目录中的工具，调用还须消费与名称和参数绑定的一次 ACP 批准。官方工具继续使用现有的 VT Code 中继白名单。

运行聚焦探针时传入 VT Code 可执行文件、RustCodeGraph 可执行文件和已建索引的 RueJS 私有副本。探针覆盖无挂载、批准、拒绝、撤除，以及 MCP 令牌错误和重复提交已消费授权。批准轮先获准执行官方 `list_files`，再验证三个挂载工具的实际调用和回传。

```sh
python3 probe.py /absolute/path/vtcode /absolute/path/rustcodegraph /absolute/path/indexed-ruejs
```

可在末尾加 `allow` 单跑挂载。原版 0.183.0 在首次模型调用缺少三种 MCP 工具声明时失败，无挂载和撤除两个对照通过。

Rust 1.100.0 的 Darwin arm64 本地构建已通过，三类真实图谱工具都完成了原生回传。聚焦的批准、拒绝、无挂载、撤除四项通过。受管安装和中继使用同一受审产物。详细轮次见[验收记录](../../../docs/dev/acp/mcp-acceptance/vtcode.md)。
