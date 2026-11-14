# 查询脚本执行边界

2026-11-14，任务20。本轮遵照用户要求仅静态核对与实现，不运行任何 Cargo、构建、单元测试、实机测试或最终回归。本文不是运行验收结论。

## 执行器取舍

保留 `vw-agent-apps` 的 `rquickjs` 依赖。查询脚本有真实生产调用者，删除 QuickJS 会破坏自定义脚本及通用模板查询。最小删除集合为空，不新增第二执行器、shell替代或脚本权限。

前置合同：[余额、套餐额度与查询脚本合同](../apps/usage/contract.md)，[来源覆盖及历史交付记录](../apps/usage/coverage.md)。历史测试输出不作为本轮通过证据；来源快照的归档移除由任务21、22统一负责。

|层次|实际生产链|职责边界|
|---|---|---|
|试运行API|`crates/vw-agent/src/gateway/api/handlers/agent_apps/apps_usage_script.rs` 的 `/apps/usage/script-test` → `script_test` → `run_trial`|选择租户、项目、主机及供应商资源，读取已授权凭据引用与origin；运行中每秒及响应前复核授权，撤权取消；结果不缓存|
|正式刷新|`crates/vw-agent/src/gateway/api/handlers/agent_apps/usage_refresh.rs` 的脚本分支 → `PreparedUsageScript::run`|复用同一执行器与受控请求，而非另起JS运行实现；原生订阅、余额、Coding Plan保持各自类型化适配器|
|脚本编排|`crates/vw-agent-apps/src/usage/scripts/run.rs` 的 `PreparedUsageScript::run`|同一Operation串联请求生成、宿主HTTP与结果提取；各阶段检查取消|
|JS引擎|`usage/scripts/runtime.rs` 的 `run_stage`|请求生成与提取均使用同一实现；每阶段新Runtime/Context用于隔离，属于隔离实例而非第二执行器|
|宿主网络|`usage/scripts/network.rs` 的 `send_candidate` → `build_http_request`|只接受宿主授权的精确origin，非自定义模板另绑定base URL；禁重定向、代理与自动重试；敏感头不公开|
|公开结果|`usage/scripts/extract.rs`|验证提取结果，保持既有UsageScriptPackage形状；不公开原始脚本异常、凭据或响应|

## 保留的限制

`run_stage` 通过 `spawn_blocking` 执行QuickJS，设置16 MiB内存、256 KiB栈、5秒线程CPU预算与Operation中断；源码及响应也受限额约束。Context只执行脚本和JSON转换，未注册文件系统、进程、任意网络宿主函数或模块加载器。网络由Rust在引擎外发起，脚本不能自行声明目标权限。

宿主请求拒绝未授权目的地、重定向、Host/转发/代理及请求分帧头；超时夹在2至30秒，响应有内存上限。`run_trial` 校验脚本类别、源码长度、凭据种类及资源授权，不用请求中的URL代替策略授权。上述为源码观察，本轮未执行越权请求或合法脚本。

## 静态证据与恢复

- `rustcodegraph status`：退出0，7988个已索引文件。使用精确文件node检查runtime、run、network、API试运行及已有runtime测试；宽泛explore包含来源同名项，不将其视为正式调用证明。
- `rg -n 'rquickjs|quick_js|quickjs|boa_engine|deno_core' --glob 'Cargo.toml' --glob '*.rs' crates/vw-agent-apps/src crates/vw-agent-apps/Cargo.toml`：退出0，仅正式包依赖及runtime的rquickjs引用。未扫描来源树作为运行调用者。
- 直接源码绑定 `run_trial` 的 `script.run(...)` 与 `usage_refresh.rs` 的 `script.run(...)`；两者共享 `PreparedUsageScript`，不删除任何一条业务入口。
- 已有独立测试文件为 `usage/scripts/{runtime,network,run,extract,config}_tests.rs`，包含合法结果、非法输入、超时、栈/内存限制、取消及目的地拒绝场景。本轮均未执行，未新增重复测试或字符串匹配业务测试。
- 后续统一验证仍需确认合法脚本响应形状、禁用文件/进程能力、网络origin拒绝、重定向拒绝与取消行为。不存在本轮有效测试数、通过结论或macOS实机结果。

未修改生产代码、Cargo清单/锁文件、来源快照、真实配置或用户数据。恢复沿用[准入与所有者](准入与所有者.md)；本任务仅文档，逆转本任务文档提交即可。
