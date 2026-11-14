# 最终清理交接

2026-11-14，任务28，静态基线 `0476811f2`。本轮按用户要求只实施和静态审查，没有执行任何 Cargo 相关命令、构建、单元测试、实机测试或最终回归。编号任务卡删除仅表示本轮实施收尾，**不表示 macOS、编译或业务等价验收通过**。母任务21、母计划及本计划 plan.md/prompt.md 保持只读。

## 五项需求与证据

|母任务21需求|静态交付与限制|
|---|---|
|1. 领域唯一所有者|[准入与所有者.md](准入与所有者.md)列14域；下表各域证据绑定真实API/SDK/执行端/Iced与作用域。已有统一所有者采用零删除，不制造改动。|
|2. 调用者和写入切换后删除|各域未发现需要再次切换的同职责运行副本；未操作真实用户数据、未生成业务前后对照，不能据静态链路宣称重启/回退行为通过。实际删除及恢复见下表。|
|3. 存储、IPC、TS和代理边界|[database.md](database.md)、[ipc.md](ipc.md)、[frontend.md](frontend.md)、[relay.md](relay.md)记录正式路径和真实保留用途；不同作用域存储、常驻路由与会话relay不强合。|
|4. 运行依赖按真实调用者清理|[tauri-dependencies.md](tauri-dependencies.md)确认正式清单无Tauri运行时；通知依赖保留。[scripts.md](scripts.md)确认查询脚本仍使用唯一受限QuickJS，不删有调用者执行能力。|
|5. 可恢复归档与许可|[archive.md](archive.md)、[归档清单.json](归档清单.json)、[snapshot-removal.md](snapshot-removal.md)记录持久归档、1495文件恢复核对及1489来源文件删除，完整MIT许可和实际夹具另存。此为静态恢复核对，功能验证尚未执行。|

## 领域索引与保留边界

|领域|证据|保留用途|
|---|---|---|
|供应商|[providers.md](providers.md)|应用配置与推理供应商不同职责|
|模型|[models.md](models.md)|共享目录能力与元数据解析|
|账号|[accounts.md](accounts.md)|主机凭据、刷新和授权租约|
|配置|[config.md](config.md)|原生文件事务与租户网关设置分离|
|MCP|[mcp.md](mcp.md)|受管目录、分发与运行时连接分离|
|技能/插件|[skills.md](skills.md)|主机安装、项目技能、插件与ACP权限边界|
|提示词|[prompts.md](prompts.md)|CAS权威存储与激活文件投影|
|会话|[sessions.md](sessions.md)|外部历史与自主代理会话分离|
|用量/价格|[usage.md](usage.md)|唯一入账与价格所有者，保留来源幂等身份|
|数据库|[database.md](database.md)|同路径工厂复用与租户键隔离|
|网络|[network.md](network.md)|代理、编码、超时、流式及受控访问差异|
|调度|[scheduling.md](scheduling.md)|cron、任务、网关后台与设备同步作用域分离|
|事件|[events.md](events.md)|revision及租户作用域；Iced查询刷新不宣称SSE订阅|
|安装更新|[installation.md](installation.md)|应用生命周期与ShiJu自身升级分离|

调度证据前段为历史阻塞调查，末尾“Recovery implementation”覆盖其阻塞状态。本轮 `76c13d5d1` 在设置加密/落盘前明确拒绝未支持的 auto_sync=true，API授权后返回422，手动配置保存或清除后等待同目标生命周期停止。完整自动上传启动仍缺显式持久化的资源选择授权、执行目标和远端基线合同，不能由开关推断授权，也不能报告已完成该母领域功能。归档保留旧来源以供未来实现对照。

## 实际删除、迁移与恢复点

|原路径或行为|结果与静态依据|恢复点|
|---|---|---|
|`crates/vw-agent-apps/upstream/` 1489文件|仅来源参考，不在正式模块树；唯一实际include_str夹具迁移后，无正式运行/测试路径引用，搜索范围与结果见snapshot-removal.md；各域功能等价仍待运行验证|`64ad9bee4`归档证据及仓库外持久tar；严格按归档清单核对再按需恢复|
|根Cargo.toml的来源workspace.exclude|来源移除后失效，只删除此项，不改其他成员/锁文件|`ef9ccbeb4`指定hunk|
|MCP Codex include_str旧来源路径|迁至`crates/vw-agent-apps/tests/fixtures/mcp/codex-config.toml`，原字节SHA256见snapshot-removal.md；消费者未编译|`ef9ccbeb4`指定fixture与引用hunk|
|upstream/LICENSE|完整许可保存在`crates/vw-agent-apps/CC-Switch-LICENSE`，不因删除丢失版权|`f2a96de3a`；归档亦保留原许可|
|本计划24张编号任务卡|按用户写完即删除，plan/prompt与正式证据保留|从各卡删除前Git版本按需提取，不重置工作区|

恢复先保存本地差异，按archive.md解压到新目录并核对SHA256/模式，再选择需要的路径或hunk；禁止整体reset/clean、覆盖其他任务或恢复新旧双写。归档位于本机持久目录，其他主机使用前须另行取得文件并核对归档SHA256；仓内清单不能代替归档本体。历史来源清单含一项已脱敏内容哈希差异，按本次归档清单恢复，不重新引入已清空值。准入索引中的upstream路径为删除前历史位置，当前许可位置以上表为准。

## 后续统一验证与macOS交付

[api-sdk-regression.md](api-sdk-regression.md)列明真实测试模块及覆盖层次，区别HTTP固件、实际Router/SDK/隔离存储；不存在的cc_convergence测试不能记作通过。本轮没有行为命令退出码或有效测试数，静态空白检查不能代替行为验收。

统一验证优先覆盖调度新改动的拒绝无副作用、API422、停止等待和作用域隔离，再覆盖迁移夹具编译及删树后的依赖可达性，之后按14域已有有效过滤入口补API/SDK、持久化重读、取消及权限失败行为。执行时遵守仓库槽位入口与Rust重型编译串行约定；本轮不运行这些命令。

macOS实机仍待用户按母计划人工验收单确认：原生窗口与暗黑主题/非ASCII渲染、各领域SDK页面读写、授权与错误反馈、进度取消、浏览器/剪贴板等本机动作、应用安装及桌面更新。平台版本、架构、真实操作与结果均须届时记录，本轮不补写为已通过。下一步在统一验证后由母任务22的执行会话处理macOS交付，母任务21是否完成由其原会话判断；本交接不改写母任务状态。
