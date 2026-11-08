//! 对话流设置模块
//!
//! 该模块负责处理对话流（Dialogue Flow）相关的设置和配置管理，包括：
//! - 项目权限配置的加载、保存和重置
//! - 对话流 UI 行为设置的加载和保存
//!
//! 该模块主要与 Iced UI 框架集成，通过 `Task` 异步执行文件操作，
//! 并通过消息（Message）机制与主应用进行通信。

use crate::app::config::{
    PreferencesScope, SystemSettingsSection, gateway_client_endpoint, preferences_scope,
};
use crate::app::{App, Message};
use iced::Task;
#[cfg(not(target_arch = "wasm32"))]
use serde_json::{Map, Value};
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;

use super::messages::SettingsMessage;

/// 加载项目对话流权限配置的任务
///
/// 该函数异步读取项目配置文件中的 `permission` 字段，并将其格式化为 JSON 字符串。
/// 在 WebAssembly 环境中，该函数直接返回空对象。
///
/// # 参数
///
/// - `project_path`: 可选的项目根目录路径。如果为 `None`，返回空对象
///
/// # 返回值
///
/// 返回一个 `Task<Message>`，该任务会：
/// - 成功时：发送 `DialogueFlowPermissionLoaded(Ok(json_string))` 消息
/// - 失败时：发送 `DialogueFlowPermissionLoaded(Err(error_message))` 消息
///
/// # 行为说明
///
/// - 在 WASM32 架构下，直接返回空对象 "{}"
/// - 在非 WASM32 架构下，读取 `<project_path>/config.json` 文件
/// - 如果配置文件不存在，返回空对象而不是错误
/// - 如果配置文件存在但无法读取或解析，返回错误消息
///
/// # 示例
///
/// ```ignore
/// let task = dialogue_flow_permission_text_task(Some("/path/to/project".to_string()));
/// ```
pub fn dialogue_flow_permission_text_task(project_path: Option<String>) -> Task<Message> {
    Task::perform(
        async move {
            // WASM32 平台不支持文件系统操作，直接返回空对象
            #[cfg(target_arch = "wasm32")]
            {
                let _ = project_path;
                return Ok("{}".to_string());
            }
            // 非 WASM32 平台：执行实际的文件读取操作
            #[cfg(not(target_arch = "wasm32"))]
            {
                // 如果没有提供项目路径，返回空对象
                let Some(project_path) = project_path else {
                    return Ok("{}".to_string());
                };
                // 构建配置文件路径：<project_path>/config.json
                let config_path = PathBuf::from(project_path).join("config.json");
                // 使用 spawn_blocking 在阻塞上下文中读取文件内容
                let read_res =
                    tokio::task::spawn_blocking(move || std::fs::read_to_string(&config_path))
                        .await
                        .map_err(|e| e.to_string())?;
                // 处理文件读取结果
                let content = match read_res {
                    Ok(s) => s,
                    // 文件不存在时返回空对象（而不是错误）
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                        return Ok("{}".to_string());
                    }
                    Err(e) => return Err(e.to_string()),
                };
                // 解析 JSON 内容
                let v = serde_json::from_str::<Value>(&content)
                    .map_err(|e: serde_json::Error| e.to_string())?;
                // 提取 permission 字段，如果不存在则使用空对象
                let perm =
                    v.get("permission").cloned().unwrap_or_else(|| Value::Object(Map::new()));
                // 将 permission 对象序列化为美化的 JSON 字符串
                serde_json::to_string_pretty(&perm).map_err(|e| e.to_string())
            }
        },
        |res| Message::Settings(SettingsMessage::DialogueFlowPermissionLoaded(res)),
    )
}

#[cfg(test)]
#[path = "dialogue_flow_tests.rs"]
mod dialogue_flow_tests;

/// 重置项目对话流权限配置的任务
///
/// 该函数异步移除项目配置文件中的 `permission` 字段，并将更新后的配置写回文件。
/// 重置后会释放项目状态，确保配置变更生效。
///
/// # 参数
///
/// - `project_path`: 可选的项目根目录路径。如果为 `None`，返回空对象
///
/// # 返回值
///
/// 返回一个 `Task<Message>`，该任务会：
/// - 成功时：发送 `DialogueFlowPermissionLoaded(Ok(json_string))` 消息，其中 json_string 为重置后的权限配置（空对象）
/// - 失败时：发送 `DialogueFlowPermissionLoaded(Err(error_message))` 消息
///
/// # 行为说明
///
/// - 在 WASM32 架构下，直接返回空对象 "{}"
/// - 在非 WASM32 架构下：
///   1. 读取 `<project_path>/config.json` 文件
///   2. 从 JSON 根对象中移除 `permission` 字段
///   3. 将更新后的配置写回文件
///   4. 调用 `dispose` 释放项目状态
///   5. 返回空的权限对象
///
/// # 注意事项
///
/// - 如果配置文件不存在，返回空对象而不创建新文件
/// - 重置操作会触发项目状态的释放，可能会影响正在运行的代理实例
///
/// # 示例
///
/// ```ignore
/// let task = dialogue_flow_permission_reset_task(Some("/path/to/project".to_string()));
/// ```
fn dialogue_flow_permission_reset_task(project_path: Option<String>) -> Task<Message> {
    Task::perform(
        async move {
            // WASM32 平台不支持文件系统操作，直接返回空对象
            #[cfg(target_arch = "wasm32")]
            {
                let _ = project_path;
                return Ok("{}".to_string());
            }
            // 非 WASM32 平台：执行实际的文件读写操作
            #[cfg(not(target_arch = "wasm32"))]
            {
                // 如果没有提供项目路径，返回空对象
                let Some(project_path) = project_path else {
                    return Ok("{}".to_string());
                };
                // 构建配置文件路径：<project_path>/config.json
                let config_path = PathBuf::from(&project_path).join("config.json");
                // 读取现有配置文件内容
                let read_res = tokio::task::spawn_blocking({
                    let config_path = config_path.clone();
                    move || std::fs::read_to_string(&config_path)
                })
                .await
                .map_err(|e| e.to_string())?;

                // 解析 JSON 内容
                let mut root = match read_res {
                    Ok(s) => serde_json::from_str::<Value>(&s)
                        .map_err(|e: serde_json::Error| e.to_string())?,
                    // 文件不存在时返回空对象（而不是错误）
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                        return Ok("{}".to_string());
                    }
                    Err(e) => return Err(e.to_string()),
                };

                // 从根对象中移除 permission 字段
                if let Some(obj) = root.as_object_mut() {
                    obj.remove("permission");
                } else {
                    // 如果根不是对象（理论上不应该发生），则替换为空对象
                    root = Value::Object(Map::new());
                }

                // 将更新后的配置序列化为 JSON
                let text = serde_json::to_string_pretty(&root).map_err(|e| e.to_string())?;
                // 将更新后的配置写回文件
                tokio::task::spawn_blocking({
                    let config_path = config_path.clone();
                    let text = text.clone();
                    move || std::fs::write(&config_path, text)
                })
                .await
                .map_err(|e| e.to_string())?
                .map_err(|e| e.to_string())?;

                let perm =
                    root.get("permission").cloned().unwrap_or_else(|| Value::Object(Map::new()));
                serde_json::to_string_pretty(&perm).map_err(|e| e.to_string())
            }
        },
        |res| Message::Settings(SettingsMessage::DialogueFlowPermissionLoaded(res)),
    )
}

const REASONING_SUMMARY: &str = "dialogue_flow_show_reasoning_summary";
const SHELL_SECTION: &str = "dialogue_flow_expand_shell_tool_section";
const EDIT_SECTION: &str = "dialogue_flow_expand_edit_tool_section";

fn dialogue_flow_ui_settings_load_task(app: &mut App) -> Task<Message> {
    let endpoint = gateway_client_endpoint();
    let scope = preferences_scope(&endpoint);
    let persistence = &mut app.desktop_preferences_persistence;
    if persistence.has_system_settings_write(&scope, SystemSettingsSection::DialogueFlow) {
        return Task::none();
    }
    persistence.dialogue_flow_load_revision =
        persistence.dialogue_flow_load_revision.wrapping_add(1);
    let revision = persistence.dialogue_flow_load_revision;
    Task::perform(
        async move {
            #[cfg(target_arch = "wasm32")]
            {
                Ok((true, false, false))
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let cfg = vw_gateway_client::GatewayClient::new(endpoint)?
                    .desktop_system_settings_get::<vw_config_types::ui::AppSystemSettingsConfig>()
                    .await?
                    .unwrap_or_default();
                Ok((
                    cfg.dialogue_flow_show_reasoning_summary,
                    cfg.dialogue_flow_expand_shell_tool_section,
                    cfg.dialogue_flow_expand_edit_tool_section,
                ))
            }
        },
        move |result| {
            Message::Settings(SettingsMessage::DialogueFlowUiSettingsLoaded(
                scope, revision, result,
            ))
        },
    )
}

fn save_dialogue_flow_settings(
    app: &mut App,
    fields: impl IntoIterator<Item = (&'static str, bool)>,
) -> Task<Message> {
    let revision = &mut app.desktop_preferences_persistence.dialogue_flow_load_revision;
    *revision = revision.wrapping_add(1);
    #[cfg(target_arch = "wasm32")]
    {
        let _ = fields;
        Task::done(Message::Settings(SettingsMessage::DialogueFlowUiSettingsSaved(Ok(()))))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let fields =
            fields.into_iter().map(|(key, value)| (key.to_owned(), value.into())).collect();
        app.save_system_settings_fields(SystemSettingsSection::DialogueFlow, fields)
    }
}

fn dialogue_flow_load_is_current(app: &App, scope: PreferencesScope, revision: u64) -> bool {
    revision == app.desktop_preferences_persistence.dialogue_flow_load_revision
        && scope == preferences_scope(&gateway_client_endpoint())
}

/// 处理对话流设置相关的消息更新
///
/// 该函数是对话流设置模块的核心消息处理器，负责响应和处理所有与对话流配置相关的用户交互和系统事件。
/// 它根据不同的 `SettingsMessage` 类型，执行相应的操作并返回可能需要的后续任务。
///
/// # 参数
///
/// - `app`: 可变引用的应用状态实例，用于读取和更新应用数据
/// - `message`: 设置消息枚举，指定要执行的操作类型
///
/// # 返回值
///
/// 返回一个 `Task<Message>`，表示需要执行的后续任务。某些操作可能返回 `Task::none()`，表示不需要执行任何任务。
///
/// # 消息处理分支
///
/// ## 权限管理
///
/// - `DialogueFlowPermissionRefresh`: 刷新权限配置和对话流 UI 设置（同时触发两个加载任务）
/// - `DialogueFlowPermissionLoaded`: 权限配置加载完成，更新编辑器内容
/// - `DialogueFlowPermissionReset`: 重置权限配置（删除 permission 字段）
///
/// ## 对话流 UI 设置管理
///
/// - `DialogueFlowUiSettingsLoaded`: 对话流设置加载完成，更新界面或显示错误
/// - `DialogueFlowUiSettingsSave`: 手动触发对话流设置保存
/// - `DialogueFlowUiSettingsSaved`: 对话流设置保存完成，显示成功或失败消息
/// - `DialogueFlowShowReasoningSummaryToggled`: 切换推理摘要显示状态并立即保存
/// - `DialogueFlowExpandShellToolSectionToggled`: 切换 shell 工具展开状态并立即保存
/// - `DialogueFlowExpandEditToolSectionToggled`: 切换编辑工具展开状态并立即保存
///
/// # 示例
///
/// ```ignore
/// let task = update(&mut app, SettingsMessage::DialogueFlowPermissionRefresh);
/// ```
pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    match message {
        // 刷新权限配置和对话流 UI 设置（同时触发两个加载任务）
        SettingsMessage::DialogueFlowPermissionRefresh => Task::batch(vec![
            dialogue_flow_permission_text_task(app.project_path.clone()),
            dialogue_flow_ui_settings_load_task(app),
        ]),
        // 权限配置加载完成，更新编辑器内容
        SettingsMessage::DialogueFlowPermissionLoaded(res) => {
            // 如果加载失败，将错误消息作为文本内容显示
            let text = res.unwrap_or_else(|e| format!("读取失败: {e}"));
            // 更新权限编辑器的内容
            app.dialogue_flow_permission_editor =
                iced::widget::text_editor::Content::with_text(&text);
            Task::none()
        }
        // 重置权限配置（删除 permission 字段）
        SettingsMessage::DialogueFlowPermissionReset => {
            dialogue_flow_permission_reset_task(app.project_path.clone())
        }
        // 对话流设置加载完成，更新状态或显示错误消息
        SettingsMessage::DialogueFlowUiSettingsLoaded(scope, revision, res) => {
            if !dialogue_flow_load_is_current(app, scope, revision) {
                return Task::none();
            }
            match res {
                Ok((
                    show_reasoning_summary,
                    expand_shell_tool_section,
                    expand_edit_tool_section,
                )) => {
                    app.dialogue_flow_show_reasoning_summary = show_reasoning_summary;
                    app.dialogue_flow_expand_shell_tool_section = expand_shell_tool_section;
                    app.dialogue_flow_expand_edit_tool_section = expand_edit_tool_section;
                }
                Err(e) => {
                    app.dialogue_flow_settings_translation =
                        Some(crate::app::i18n::format::Message::new(
                            "读取配置失败: %s",
                            vec![e.clone().into()],
                        ));
                    app.dialogue_flow_settings_save_message = Some(format!("读取配置失败: {e}"));
                }
            }
            Task::none()
        }
        // 手动触发对话流设置保存
        SettingsMessage::DialogueFlowUiSettingsSave => save_dialogue_flow_settings(
            app,
            [
                (REASONING_SUMMARY, app.dialogue_flow_show_reasoning_summary),
                (SHELL_SECTION, app.dialogue_flow_expand_shell_tool_section),
                (EDIT_SECTION, app.dialogue_flow_expand_edit_tool_section),
            ],
        ),
        SettingsMessage::DialogueFlowShowReasoningSummaryToggled(value) => {
            app.dialogue_flow_show_reasoning_summary = value;
            save_dialogue_flow_settings(app, [(REASONING_SUMMARY, value)])
        }
        SettingsMessage::DialogueFlowExpandShellToolSectionToggled(value) => {
            app.dialogue_flow_expand_shell_tool_section = value;
            save_dialogue_flow_settings(app, [(SHELL_SECTION, value)])
        }
        SettingsMessage::DialogueFlowExpandEditToolSectionToggled(value) => {
            app.dialogue_flow_expand_edit_tool_section = value;
            save_dialogue_flow_settings(app, [(EDIT_SECTION, value)])
        }
        // 对话流设置保存完成，显示成功或失败消息
        SettingsMessage::DialogueFlowUiSettingsSaved(res) => {
            app.dialogue_flow_settings_translation = Some(match &res {
                Ok(()) => crate::app::i18n::format::Message::new("已保存对话流配置", vec![]),
                Err(e) => {
                    crate::app::i18n::format::Message::new("保存失败: %s", vec![e.clone().into()])
                }
            });
            app.dialogue_flow_settings_save_message = Some(match res {
                Ok(()) => "已保存对话流配置".to_string(),
                Err(e) => format!("保存失败: {e}"),
            });
            Task::none()
        }
        // 其他消息不处理，返回空任务
        _ => Task::none(),
    }
}
