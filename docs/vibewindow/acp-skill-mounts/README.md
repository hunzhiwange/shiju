# ACP 技能目录挂载

2026-11-06 交付：**16 项实测、1 项确认不支持、5 项用户豁免验收**。豁免不等于实测支持；OpenClaw 仅 session-model 已验证。版本和平台范围严格限定于已审核的固定 Darwin-arm64 发行物。

- [验收说明与逐项结论](acceptance.md)
- [22 项固定发行物与能力清单](agents.json)
- [本轮回归审查](evidence/regression.json)及[任务 33 原始命令与计数](evidence/lifecycle.json)
- [客户端最小接入示例](../ACP-对外接入.md)

出站使用会话专属目录视图，不将全文提前拼入首次 prompt；原生工具按需读取。未支持入口明确报错，无文本 fallback。自身服务端支持技能与标准 stdio/HTTP/SSE MCP 外挂、原子替换和跨进程恢复。模型 HTTP 可 fixture，真实 CLI、ACP 与工具执行不可替代。

`candidate/unverified` 保留证据缺口；`unsupported` 需要明确负向证据；`verified` 限定对应版本、平台与模型来源。用户豁免另存 `delivery_acceptance`，不会改写实测状态。缓存发行物缺失后须重取固定版本并核对散列，禁止用 latest 或模型来源旧结论代替技能验证。

## ShiJu 会话挂载扩展 v1

共享类型由 `vw-acp` 导出。此处定义自有协议，不修改 ACP 标准字段，不向其他 agent 强发自家扩展；外部 agent 仍须逐项核实其入口。

initialize 响应中的 `_meta["shiju.mounts"]`：

```json
{"version":1,"skills":false,"setMounts":false}
```

类型默认关闭；当前自身服务端声明 `skills:true` 与 `setMounts:true`。客户端缺失声明、未知版本或 `setMounts:false` 时不得发送更新。`SetSessionMountsRequest::to_ext_request` 强制检查此门禁。

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

响应只包含实际生效名称。`SessionMountRevision` 使用非负 u64，`next` 校验冲突和溢出。标准 new/load 的 `mcpServers` 每次由调用方完整提供。

识矩服务端的替换请求还接受可选的 `mcpCatalog` 数组，每项包含 `scope` 和 `record`，记录引用包含 `id` 和 `revision`。省略或传空数组均清除之前的受管选择。服务端依据已绑定会话的租户和项目验证作用域，挂载状态只保存受管引用。load 重新检查保存的修订，过期或删除的条目不再挂载。标准 MCP 连接定义和凭据不写入挂载状态。

协议测试与真实验收分开计数；实际生产证据见上述验收说明。

### 协议任务验证（2026-11-05）

- 红灯：`cargo test -p vw-acp --lib session_mounts_contract_ -- --nocapture --test-threads=1`，5 个测试中 3 失败、2 通过，退出 101；失败分别为未知版本、非法路径、重复名称。槽位 3，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-3`。构建目录配置检查后重跑仍得到同一红灯结果。
- 绿灯：先执行 `cargo fmt --all`（退出 0），再执行同一聚焦命令，9 通过、0 失败、0 ignored，退出 0；最终补齐路径形状断言和 Windows fixture 后再次执行，仍为 9 通过。两轮绿灯使用槽位 2，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。
- `cargo clippy --all-targets --all-features -- -D warnings` 在两轮绿灯后均退出 0。中间一次实现编译因 SDK 非穷尽 MCP 枚举缺少拒绝分支而失败，已补明确错误；该次零测试不计行为通过。
- 因仓库设置了 `[build].build-dir="target"`，最终验证同时设置 `CARGO_BUILD_BUILD_DIR="$CARGO_TARGET_DIR"`，避免中间产物绕出槽位。每阶段通过原子 mkdir 动态领槽并记录 PID；结束后释放自己的锁，保留缓存。没有执行全量测试、wasm 检查或 Makefile。
- 证据覆盖严格解析、wire 往返、缺省/清空、revision 冲突与溢出、未声明能力时不生成扩展请求。此处有效行为测试数为 9，不计为真实 CLI 验收。
