# macOS 设备同步交付记录

2026-11-14。本轮按用户要求只实现代码，不运行 Cargo、单元测试、实机测试或统一回归。本记录是源码静态盘点，**不是 macOS 功能验收通过记录**。任务卡删除仅表示本轮交付收尾；所有运行行为仍待后续统一验证。Linux、Windows、wasm 和发布均未验收。

## 来源与前置

来源映射沿用[来源覆盖](来源覆盖.md)与[设备数据边界](设备数据边界.md)。历史来源哈希和旧证据未在本任务重新执行，不作为当前版本成功证据。只读计划保留拆分时的历史描述，其“业务包不存在”等文字不表示当前状态。

当前已有 `crates/vw-agent-apps/src/device_sync/` 协议/归档/两传输实现，以及 `crates/vw-agent/src/device_sync/` 网关存储、任务和应用实现。类型化快照来自 `agent_apps/shared_snapshot.rs::capture_shared_snapshot`，接收经 `shared_snapshot_import.rs::{preview_shared_snapshot,apply_shared_snapshot}`；设备同步导出与导入分别由 `device_sync/export.rs`、`import.rs` 消费。凭据不通过桌面自行持久化，连接读写由网关配置 handler 及 `webdav_settings.rs`、`s3_settings.rs` 承担。上述是当前文件/调用入口存在的证据，未经编译或行为验证。

## 11 个来源命令的静态接线

API 基路径为 `/v1/agent-apps/device-sync`。下表 SDK 文件均在 `crates/vw-gateway-client/src/client/`；桌面状态文件均在 `crates/vw-desktop/src/app/message/settings/`。

| 来源命令 | HTTP 路径与方法 | SDK 方法 | 桌面入口 |
| --- | --- | --- | --- |
| webdav_test_connection | POST /webdav/test | device_sync_test_webdav | device_sync_webdav::Event::Test |
| webdav_sync_save_settings | PUT /webdav/config | device_sync_save_webdav_config | Event::Save |
| webdav_sync_clear_settings | DELETE /webdav/config | device_sync_clear_webdav_config | Event::Clear / ConfirmClear |
| webdav_sync_fetch_remote_info | GET /webdav/remote | device_sync_remote | device_sync_preview::PreviewState |
| webdav_sync_upload | POST /webdav/upload | device_sync_upload | PreviewState 显式确认 |
| webdav_sync_download | POST /webdav/download | device_sync_download | PreviewState 显式确认 |
| s3_test_connection | POST /s3/test | device_sync_test_s3 | device_sync_s3::Event::Test |
| s3_sync_save_settings | PUT /s3/config | device_sync_save_s3_config | Event::Save |
| s3_sync_fetch_remote_info | GET /s3/remote | device_sync_remote | PreviewState |
| s3_sync_upload | POST /s3/upload | device_sync_upload | PreviewState 显式确认 |
| s3_sync_download | POST /s3/download | device_sync_download | PreviewState 显式确认 |

配置 SDK 位于 `device_sync_config_api.rs`，连接/预览/启动位于 `device_sync_operations_api.rs`；网关路由分别由 `gateway/api/handlers/device_sync_config.rs`、`device_sync_probe.rs`、`device_sync_jobs.rs` 和 `device_sync_download.rs` 声明。额外 POST /{transport}/local-preview 提供本地版本和连接版本供启动复核，不是来源第十二个命令；不新增 S3 清除语义。

## 两个事件、恢复与界面

来源 webdav-sync-status-updated / s3-sync-status-updated 由配置查询返回的 last_status、active_task_id 及 GET /tasks/{id} 承接。DELETE /tasks/{id} 取消任务；SDK 在 `device_sync_tasks_api.rs`。`components/system_settings_device_sync/task_panel.rs` 轮询状态，两个表单转发 Discover、Refresh 及取消消息，错误经现有 toast 路径展示。运行时断线恢复、自动错误来源、取消 too_late、成功附警告均待验证。

`components/system_settings.rs` 调用 `system_settings_device_sync::entry_points()` 和 `overlay()`，后者选择 WebDAV 或 S3 弹窗。两个状态模块分发至预览和进度组件，两个 modal 使用统一设置控件。源码入口已存在；暗黑对比、实际中文整形、4px 滚动条及留白、固定标题和操作栏、长内容交互均未实机观察。

两个弹窗共用 `system_settings_device_sync/selection.rs`，从当前作用域已加载库存显式选择供应商、MCP、规则、技能、技能仓库、偏好及组合，默认空选择；组合选中时带入供应商/MCP/规则引用。库存未加载时需先到对应设置页刷新。**范围限制：含技能引用的组合因缺少客户端 opaque installation_id 到授权目录的解析接口，明确禁用并展示原因。** 单独技能使用库存中的目录标识。库存作用域过滤、选择变化失效预览、组合依赖映射及授权目录均未运行验证；不能宣称完整来源能力等价通过。

## 后端静态覆盖与待验

| 关注点 | 当前源码入口 | 后续统一验证内容（本轮全部未执行） |
| --- | --- | --- |
| 独立格式、摘要与完整性 | agent-apps/device_sync/{manifest,artifact,conflict}.rs | 未知版本拒绝、大小上限、摘要失败、empty 与缺省区别 |
| 归档与路径 | archive_pack.rs、archive_extract.rs、skill_update.rs | 穿越/符号链接、数量/解压上限、替换失败和回退 |
| WebDAV | webdav_url.rs、webdav_transport.rs、webdav_remote.rs；agent/device_sync/webdav_upload.rs、webdav_download.rs | 编码、认证、ETag/摘要、超时、manifest最后发布、完整校验后应用 |
| S3 | s3_config.rs、s3_signing.rs、s3_transport.rs、s3_remote.rs；agent/device_sync/s3_upload.rs、s3_download.rs | AWS/custom签名、流限制、错误脱敏、同等提交与应用边界 |
| 凭据与锁 | agent/device_sync/{webdav_settings,s3_settings,operation_lock}.rs | Keep/Replace/Clear、换身份不继承、清除后旧凭据不再使用、作用域隔离 |
| 导出、接收及回滚 | export.rs、import.rs、apply.rs、skill_receive.rs、projection.rs | 双向字段过滤、设备字段保留、回退点和资源恢复、投影失败警告 |
| 冲突和确认 | device_sync_preview.rs、device_sync_probe.rs、device_sync_download.rs | 本地/远端/连接版本复核、409重预览、dirty和迟到回复隔离、启动无副作用重试 |
| 任务与取消 | agent/device_sync/{tasks,status,cancel}.rs | 进度单调、持久化失败警告、too_late与真实终态、重启interrupted不重放 |
| 自动同步 | change_events.rs、auto_sync.rs、lifecycle.rs；gateway/runtime.rs、bootstrap.rs | 两开关、提交后通知、1秒去抖/10秒上限、锁内重读、关闭窗口运行、清除/关停不迟到上传 |

本任务没有执行匿名 HTTP fixture、真实 WebDAV/S3 服务、归档或恢复测试；不存在本轮测试数量、成功退出码或 Cargo 槽位记录。后续先按受影响包完成必要定向编译与行为测试，再组织 macOS 隔离服务及实际窗口验收。用户人工验收依 `.plans/2026-11-05-CC能力完整融合/人工验收.md` 另行开展；母任务19和其它平台没有通过结论。

## 本次文档检查

人工复读以上映射和源码路径。仅运行 `git diff --check -- docs/vibewindow/agent-apps/device-sync/macOS验收记录.md`（退出码0）；新文件另经完整读取，空 diff 不作为行为通过证据。该检查仅覆盖文档空白错误。
