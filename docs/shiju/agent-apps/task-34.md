# 任务34：生命周期API

状态：已完成。2026-11-06，macOS。

## 依赖与实际接线

已核对任务33证据和真实授权文件加载器、TaskId/TaskStatus、固定动作DTO、主机写锁、Operation、包管理器配方和卸载执行器。未将母计划旧状态标签视作缺少交付；没有新增执行主机RPC或通用命令转发。生产入口仍为handler_router→agent_apps::router，此次合并生命周期路由；plan.md、来源快照保持只读。

## 最终路由与SDK映射

均在现有管理API及其/v1等价前缀，要求既有Basic/TenantScope和独立x-agent-apps-token：

- POST /agent-apps/actions/preview?targetHostId=…：AppActionPreviewRequest→AppActionPreview。
- POST /agent-apps/actions?targetHostId=…：AppActionSubmitRequest→202 AppActionExecutionResult。
- GET /agent-apps/operations/{id}?targetHostId=…：AppOperationStatus。
- GET /agent-apps/operations/{id}/events?targetHostId=…：最多25秒长轮询，返回当前AppOperationStatus；断线重新读取当前快照，不重提写入。不是stdout/stderr流，不承诺重放每个中间阶段。
- DELETE /agent-apps/operations/{id}?targetHostId=…：授权取消，等待现有Operation实际子进程树回收后返回AppOperationStatus。

SDK后续直接映射这些DTO与路径，不复用任意命令TaskSpec。底座Operation此前尚无等价HTTP资源，故此处为唯一生命周期HTTP资源。操作和预览保留在执行网关进程中：预览5分钟有效且只能提交一次；各最多1024项，操作到容量时清理终态句柄。断开HTTP请求不终止写入，进程重启或句柄清理后旧ID返回404；不宣称跨网关重启的持久恢复。跨平台、公司、主机不能读取、订阅、提交或取消另一作用域ID。

## 默认拒绝的写授权

沿用任务33私有管理员JSON文件，grant增加可选actions数组，元素仅install/update/uninstall；省略为[]，原有读取grant不会获得写权限。目标host增加可选lifecycle对象：prefix为管理员预先创建、规范绝对路径的独立安装前缀，npm为管理员批准的npm绝对路径。该npm目录必须有node，host.search_paths必须只包含prefix/bin。升级/卸载的npm路径仍须符合既有配方的同目录管理器要求，推荐在prefix/bin登记npm/node；不搜索宿主PATH。

示例字段（追加到管理员实际登记记录，不是客户端输入）：

```json
{"actions":["install","update","uninstall"]}
```

```json
{"lifecycle":{"prefix":"/approved/prefix","npm":"/approved/prefix/bin/npm"}}
```

两类字段均只能由已有受信任管理员文件提供，不建立HTTP认领入口，不更改OS权限。在线与local授权仍按任务33真实加载路径校验。每次请求重授权，队列获得实际写锁后再次复核对应动作与写环境。预览ID绑定租户、主机和服务器计划，全部动作要求confirmed=true；未知/过期/重复ID不执行。

应用配方、来源识别、计划重建与执行保留在vw-agent-apps。当前本地适配器公开已有10项npm固定安装配方；无对应受限安装配方的Desktop/Hermes/Agy/Omp返回422，不把native配方的默认HOME路径伪装为独立prefix支持。Windows/WSL目标同样明确拒绝由本地POSIX适配器执行；不冒充其他平台证据。升级/卸载只接受当前主机重新发现的安装ID、受批准prefix内的入口与真实路径；未知来源拒绝。npm来源须验证该应用固定包名的实际package.json、规范包目录和入口归属，再使用既有update/uninstall配方。没有Homebrew元数据时不伪造所有权。

安装/升级在既有主机写锁内重新构建完整计划并比较，卸载复用其唯一协调执行器及刷新闭包。子进程使用绝对工具和固定argv，env_clear后只给独立prefix作为子进程HOME、限定工具PATH、npm_config_prefix、engine_strict=true和/dev/null用户/全局npm配置；不修改进程全局HOME，不继承个人凭据或个人npm配置。Node不兼容由实际npm严格engines检查拒绝；不伪造“满足”结果，不重复网络元数据缓存。执行输出丢弃，不向HTTP传递原始stderr、凭证或进程载荷。安装/升级要求实际版本探测可运行，升级额外验证选中实例，卸载要求选中入口确实消失。

## 验证记录

全部阶段动态原子mkdir领取仓库共享槽位，写入任务/会话/PID，退出trap释放自身锁，缓存保留。检查.cargo/config.toml中build.build-dir="target"，同时覆盖中间产物路径：

- 实际槽位：slot-2。
- CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2
- CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2

初始红灯：`cargo test -p vw-agent --lib cc_apps_lifecycle_api_scope -- --nocapture`，退出101；1有效测试失败，最小已接线路由返回503，断言要求缺少身份返回401。不是编译失败或零测试。随后实现认证的同一命令退出0，1通过、11455过滤。

中间夹具验证：同一聚焦命令退出101，1通过、3失败；macOS临时目录的/var路径不等于/private/var规范路径，触发生产前缀检查。修正夹具使用规范临时目录，不放宽检查；再次运行退出0，4通过、11455过滤。

最终检查从同一slot-2阶段顺序执行，先`cargo fmt --all`（退出0），没有无关格式化差异：

| 确切命令 | 退出码 | 有效测试数 |
| --- | --- | --- |
| `cargo test -p vw-agent --lib cc_apps_lifecycle_api_scope -- --nocapture` | 0 | 5通过、11455过滤 |
| `cargo test -p vw-agent --lib cc_apps_read_api_authorization -- --nocapture` | 0 | 7通过、11453过滤；包含另1项生产路由子进程，不重复计数 |
| `cargo test -p vw-agent-apps --lib cc_apps_uninstall_revalidates_plan -- --nocapture` | 0 | 3通过、157过滤 |
| `cargo test -p vw-agent-apps --lib cc_apps_operation_cancel_reaps_process -- --nocapture` | 0 | 3通过、157过滤 |
| `cargo test -p vw-api-types --lib cc_apps_action_contract_roundtrip -- --nocapture` | 0 | 3通过、74过滤 |

| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 全仓通过，无Clippy诊断 |
| `git diff --check` / `git diff --cached --check` | 0 | 仅本任务实现、独立测试与本文档进入提交 |

Operation既有用例有意触发fixture panic并验证恢复，测试实际通过。最终阶段退出0，所有Cargo与测试子进程结束后slot-2自身锁已释放，缓存保留。编译器仍有此前任务33已记录的macOS链接器__eh_frame提示，聚焦测试实际退出0；未修改链接设置隐藏提示，无新增Clippy警告。

有效验收场景包括：真实固定npm argv调用和版本探测、安装/升级/卸载实际临时文件变化；预览不执行写入；跨租户预览提交/状态/事件/取消拒绝；未确认、重复提交、预览后实例改变、未知动作与客户端command/args/env/path/targetHostId拒绝；缺失身份、读取grant无写权限和运行后撤权；失败进程状态不泄露stderr；取消等待真实进程回收。相邻回归覆盖既有生产认证接线、原卸载重验证与操作进程树恢复。合计21个顶层有效测试（5+7+3+3+3），生产读取子进程另1项不重复计数。

差异已审查：保留先前任务1–33文件的既有删除，不暂存或提交它们；没有修改plan.md或来源与接口.md。后续推荐任务35沿用同一可信授权来源，任务38等SDK调用使用上列路径和DTO；原生安装前缀适配、真实npm网络与逐应用macOS实机验收不由本夹具冒充。本任务无UI变更，不需要渲染验收；使用真实临时文件和可控包管理器OS边界，不修改个人安装或凭据，不运行全量cargo test、fmt检查、wasm检查或Makefile。
