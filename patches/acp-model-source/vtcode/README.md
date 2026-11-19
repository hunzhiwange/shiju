# VT Code 0.183.0 会话 MCP 补丁原型

适用于 VT Code 0.183.0（`4fd8e47a885c`）。在私有源码副本应用补丁，再使用发行版指定的 Rust 1.100.0 构建 CLI。

```sh
git apply --unidiff-zero /absolute/path/vtcode-0.183.0-session-mcp.patch
cargo +1.100.0 build --locked -j 1 -p vtcode --bin vtcode
```

安装清单目前使用原厂发行，原型用于独立运行验证。

补丁让原生 `session/new` 从带宿主令牌的字面回环 HTTP 地址发现工具，将模型可见目录保存在该会话中。模型发出的调用仍经过 ACP 工具审批和 VT Code 权限检查，使用现有 MCP 客户端执行，沿用原生工具事件和同一模型调用 ID 回传结果。stdio 和 SSE 挂载显式拒绝。带 MCP 的重新挂载、加载和恢复需要新会话。

本地诊断使用和模型共用的沙箱已授权端口转发真实 RustCodeGraph，宿主按工具名及参数消费一次批准。生产接入还需要受管网关将实际选中服务投影到同一中继端口，把独立令牌与租户、会话和调用批准绑定，并在连接释放时撤销。交付前须验证该接线及发布可校验的 Darwin arm64 二进制，再跑指定阿里模型三轮。

运行聚焦探针时传入 VT Code 可执行文件、RustCodeGraph 可执行文件和已建索引的 RueJS 私有副本。探针覆盖无挂载、批准、拒绝、撤除，以及 MCP 令牌错误和重复提交已消费授权。批准轮先获准执行官方 `list_files`，再验证三个挂载工具的实际调用和回传。

```sh
python3 probe.py /absolute/path/vtcode /absolute/path/rustcodegraph /absolute/path/indexed-ruejs
```

可在末尾加 `allow` 单跑挂载。原版 0.183.0 在首次模型调用缺少三种 MCP 工具声明时失败，无挂载和撤除两个对照通过。

Rust 1.100.0 的 Darwin arm64 本地构建已通过，三类真实图谱工具都完成了原生回传。聚焦的批准、拒绝、无挂载、撤除四项通过。构建产物仅用于隔离试验。受管网关需要用已审构建的摘要校验入口，同时将同端口的 MCP 目录和一次性批准绑定到它拥有的会话。详细轮次见[验收记录](../../../docs/dev/acp/mcp-acceptance/vtcode.md)。
