# 任务 27：目录选择

状态：代码已实现，未验证。日期：2026-11-14。

依赖任务 1 的来源及接口盘点见 acceptance-1.md；母任务 15 的路径边界见 ../agent-apps/workspace-contracts.md。来源版本为 ca58c0313360eae7c851fbd7553c1614c7af03e7；实现基于共享 checkout，未改只读 plan.md。

实际入口为 app_providers/paths.rs 配置目录弹窗和消息处理器，system_settings.rs 仅承载设置页面，无需修改共享入口。沿用原有 macOS Claude 配置目录能力，不扩展其它应用声明。新增选择本机目录按钮，复用 settings 样式、现有弹窗布局与 toast。

原生异步选择器只对 loopback 网关及 local-apps 主机启用，并在打开前通过 local::launch_directory 核对当前租户的本机私有凭据。取消只解除等待，不修改草稿；成功选择仅填入草稿，保存继续调用网关。选择结果携带 Scope、请求代次、应用目录代次；关闭重开、主机/项目/租户切换后的旧结果不写回当前表单。无法恢复私有凭据或非 UTF-8 路径明确报错，不进行有损路径转换。

远端目录沿用手动输入和 GatewayClient::app_provider_directory_save，由 PUT /v1/app-providers/{app}/config/directory 提交 target_host_id 与 DirectoryOverride；读取使用 app_provider_directory。网关 paths.rs 的 provider_preview_context/provider_editor_context 负责授权后解析目标目录，原有拒绝、冲突、不可达错误继续保留，不回退本机选择器或本机文件系统。本任务没有新增远端浏览协议，也不声称支持任意远端目录枚举。现有 Form::finish 在保存失败时保留草稿。

按本轮用户覆盖要求，未运行 Cargo、调用 Cargo 的脚本、单元测试、实机测试或最终回归；未新增测试模块、未领取槽位，CARGO_TARGET_DIR 未设置。不声称编译或行为验收通过。任务卡按要求删除。后续统一验证应覆盖取消保值、选择后保存、凭据拒绝、过期选择结果丢弃，以及远端保存拒绝不回退本机。
