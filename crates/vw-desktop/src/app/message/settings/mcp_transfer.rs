use super::{McpMessage, SettingsMessage};
use crate::app::{App, Message, state::ToastKind};
use desktop_settings_mcp::configuration_transfer::{self as transfer, Message as Event};
use iced::Task;

pub(crate) fn message(event: Event) -> Message {
    Message::Settings(SettingsMessage::Mcp(McpMessage::Transfer(event)))
}

pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    match event {
        Event::CopyPrompt => Task::batch([
            iced::clipboard::write(transfer::AI_PROMPT.to_owned()),
            app.show_toast(ToastKind::Success, "已复制提示词，可粘贴到会话中"),
        ]),
        Event::Paste => {
            let state = &mut app.mcp_settings.page;
            if state.loading || state.saving || state.transfer_pending {
                return Task::none();
            }
            state.transfer_pending = true;
            iced::clipboard::read().map(|source| {
                let result = source
                    .filter(|source| !source.trim().is_empty())
                    .map(Some)
                    .ok_or_else(|| "剪贴板中没有 MCP 配置".to_owned());
                message(Event::Imported(result))
            })
        }
        Event::Import | Event::Export => {
            #[cfg(target_arch = "wasm32")]
            return app.show_toast(ToastKind::Error, "文件导入和导出需要桌面客户端");
            #[cfg(not(target_arch = "wasm32"))]
            {
                let state = &mut app.mcp_settings.page;
                if state.loading || state.saving || state.transfer_pending {
                    return Task::none();
                }
                if matches!(event, Event::Import) {
                    state.transfer_pending = true;
                    return Task::perform(import_file(), |result| message(Event::Imported(result)));
                }
                let backup = match transfer::backup(state) {
                    Ok(backup) => backup,
                    Err(error) => return app.show_toast(ToastKind::Error, error),
                };
                state.transfer_pending = true;
                Task::perform(export_file(backup), |result| message(Event::Exported(result)))
            }
        }
        Event::Imported(result) => {
            let state = &mut app.mcp_settings.page;
            if !std::mem::take(&mut state.transfer_pending) {
                return Task::none();
            }
            let source = match result {
                Ok(Some(source)) => source,
                Ok(None) => return Task::none(),
                Err(error) => return app.show_toast(ToastKind::Error, error),
            };
            if state.loading || state.saving {
                return app.show_toast(ToastKind::Error, "MCP 配置正在更新，请重新导入");
            }
            match transfer::stage_import(state, &source) {
                Ok((0, skipped)) => app
                    .show_toast(ToastKind::Info, format!("无新增服务器，跳过 {skipped} 个同名项")),
                Ok((added, skipped)) => {
                    let status = format!(
                        "已载入 {added} 个服务器，跳过 {skipped} 个同名项。检查 JSON 后点击保存。"
                    );
                    state.status_message = Some(status.clone());
                    app.show_toast(ToastKind::Success, status)
                }
                Err(error) => app.show_toast(ToastKind::Error, error),
            }
        }
        Event::Exported(result) => {
            if !std::mem::take(&mut app.mcp_settings.page.transfer_pending) {
                return Task::none();
            }
            match result {
                Ok(true) => app.show_toast(ToastKind::Success, "MCP JSON 备份已保存"),
                Ok(false) => Task::none(),
                Err(error) => app.show_toast(ToastKind::Error, error),
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn import_file() -> Result<Option<String>, String> {
    let Some(file) = rfd::AsyncFileDialog::new()
        .set_title("导入 MCP 配置")
        .add_filter("MCP 配置", &["json", "toml"])
        .pick_file()
        .await
    else {
        return Ok(None);
    };
    let path = file.path().to_owned();
    tokio::task::spawn_blocking(move || {
        use std::io::Read;
        let metadata = std::fs::metadata(&path).map_err(|_| "无法读取 MCP 配置文件")?;
        if !metadata.is_file() {
            return Err("请选择普通配置文件".into());
        }
        let file = std::fs::File::open(path).map_err(|_| "无法打开 MCP 配置文件")?;
        let mut source = String::new();
        file.take((transfer::MAX_FILE_BYTES + 1) as u64)
            .read_to_string(&mut source)
            .map_err(|_| "无法读取 UTF-8 MCP 配置")?;
        if source.len() > transfer::MAX_FILE_BYTES {
            return Err("MCP 配置文件超过 8 MiB".into());
        }
        Ok(Some(source))
    })
    .await
    .map_err(|_| "读取 MCP 配置失败")?
}

#[cfg(not(target_arch = "wasm32"))]
async fn export_file(backup: String) -> Result<bool, String> {
    let Some(file) = rfd::AsyncFileDialog::new()
        .set_title("导出 MCP JSON 备份")
        .set_file_name("shiju-mcp-backup.json")
        .add_filter("JSON", &["json"])
        .save_file()
        .await
    else {
        return Ok(false);
    };
    let path = file.path().to_owned();
    tokio::task::spawn_blocking(move || {
        use std::io::Write;
        let parent = path.parent().ok_or("备份路径无效")?;
        let mut temporary =
            tempfile::NamedTempFile::new_in(parent).map_err(|_| "无法创建备份文件")?;
        temporary.write_all(backup.as_bytes()).map_err(|_| "无法写入备份文件")?;
        temporary.as_file().sync_all().map_err(|_| "无法同步备份文件")?;
        temporary.persist(path).map_err(|_| "无法保存备份文件")?;
        Ok(true)
    })
    .await
    .map_err(|_| "导出 MCP 备份失败")?
}
