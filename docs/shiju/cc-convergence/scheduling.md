# 后台调度所有者与未交付边界

2026-11-14，任务13。基线 `0d9eddb7b4fdeafed46a4f55bb89309c0c80b98e`。按用户要求不运行 Cargo、构建、单元测试、实机或最终回归；本文件仅记录静态调查，不声称行为通过。

## 现有职责

|业务|生产入口|保留边界|
|---|---|---|
|用户 cron|`crates/vw-agent/src/daemon/mod.rs` 的 scheduler supervisor → `cron/scheduler.rs::run`|daemon生命周期与cron配置；不是桌面刷新|
|快照维护|`crates/vw-agent/src/snapshot/native.rs::init` → `scheduler::register`|按worktree的Instance作用域，任务id为snapshot.cleanup；通用scheduler按作用域与id注册|
|自主代理任务|`crates/vw-agent/src/gateway/api/handlers/agent_scheduler.rs` → `agent_scheduler/`|项目任务图、运行状态与outbox，不与cron强合；SDK为agent_scheduler_api|
|用量刷新/扫描及周期备份|`crates/vw-agent/src/gateway/bootstrap.rs::serve_gateway` 中usage_refresh、usage_scans、periodic_backups|各自在网关select中运行，消费授权策略，职责不同|
|设备自动同步|`crates/vw-agent/src/device_sync/lifecycle.rs::SyncLifecycle` → `auto_sync::run`|目标、租户、主机、principal与transport的完整DataManagementTarget；不使用默认租户推断授权|

## 真实前置缺口

`gateway/bootstrap.rs::run_gateway_until` 与 `gateway/runtime.rs::start_until` 分别创建生命周期，注入路由并在关闭时等待shutdown，属于两个可选启动入口。`SyncLifecycle::start` 实现持锁查找相同target/transport，避免重复注册；stop/shutdown取消并等待worker。但尚无生产调用者调用start或stop。

已复核 `device_sync/mod.rs` 的全部模块、`device_sync_config.rs` 的WebDAV/S3 write和WebDAV clear、配置store与网关启动包装器：配置路由仅保存/清除设置，未提取生命周期Extension，未构建reload/upload闭包。全 `crates/vw-agent` 的 `SyncLifecycle`、`auto_sync::run`、`auto_sync::` 与生命周期方法引用搜索只发现类型定义、唯一内部run调用和上述创建/关闭；未发现函数指针或包装器注册路径。

前置正式证据 `docs/vibewindow/agent-apps/device-sync/lifecycle.md` 同样明确：尚无授权目标枚举与自动上传选择入口，生命周期契约等待配置/API所有者接线；网关不会恢复连接执行自动上传。这是母任务19实际业务未交付，不是本轮不运行测试导致的阻塞。不能通过删除旧WebDAV/S3来源服务或臆造默认租户恢复掩盖缺口。

本任务保留已阻塞，不修改生产代码、不新增无真实启动链的测试、不删除来源。需要母任务19交付授权配置恢复与启停接线后，再核对是否存在等价副本可删。来源仅只读参考：`crates/vw-agent-apps/upstream/src-tauri/src/services/{webdav_auto_sync,s3_auto_sync}.rs`、`upstream/src/hooks/{useUsageAutoRefresh,useAutoExchangeRates}.ts`。来源归档归任务21/22负责。

## 依赖与恢复

任务17声明依赖任务13；其“所有后台业务已等价”的整体收口不能完成，后续依赖17的18–22以及23/28的整体完成亦受此门槛影响。其他批次2领域（14、15、16）可继续；17–20不触及自动同步旧入口的独立静态盘点可推进，但不能据此移除该来源或宣称全计划完成。

本任务没有业务数据写入或代码切换，未生成业务夹具、切换前后哈希、行为退出码或有效测试数。恢复仅撤销本文及任务卡状态hunk，保留其他任务和用户改动；后续验证应覆盖同作用域重复启用、禁用清除、跨作用域隔离与网关关闭等待。

## 静态检查

RustCodeGraph status确认索引存在；宽查询未覆盖设备同步细节、files筛选无结果，转rg和原文件读取。已读取准入证据及上述实际调用链。未领取Cargo槽位，未执行任何构建或测试。`git diff --check`仅用于本文及任务卡空白检查，不代表行为验收。

## Recovery implementation (supersedes blocked status above)

The missing contract is not just a timer: SnapshotSelection requires explicit authorization and confirmation of each exported resource; connection DTOs carry neither that selection nor a confirmed remote baseline. A boolean cannot authorize exporting all free text or skills. No automatic upload plan or default tenant is invented here.

Both WebDAVSettingsStore::save and S3SettingsStore::save now reject auto_sync=true before credential encryption or persistence. Configuration handlers return 422 after authorization for that unsupported toggle. Existing stored auto_sync=true values do not create workers; users must disable the toggle when saving until an authorized upload-plan contract exists.

After saving manual configuration, or clearing WebDAV configuration, handlers await the injected gateway SyncLifecycle::stop for the same authorized target and transport. Store calls release the business lock before stop awaits workers, avoiding a lock cycle. Standalone routers with no lifecycle extension have no gateway worker to stop. Errors return 503. No second registry, timer or desktop worker is introduced.

Task13 is now complete within the safe cleanup scope and its card is deleted. Automatic start remains unsupported and must not be reported as delivered. Later work needs explicit persistent object-selection, execution-target and remote-baseline contracts before wiring start/recovery. This evidence does not authorize removal of unsupported upstream behavior as though equivalent. Task17 may continue caller analysis while retaining this limitation.

No business data was accessed or changed. No Cargo, build or test command ran. Static diff whitespace checks passed. Future unified validation must cover store rejection without side effects, API 422, stop waiting after disable/clear, and scope isolation. Revert only this task's settings-store/handler and evidence hunks, preserving unrelated work.
