#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ScheduleCommand {
    Open,
    Enable(String),
    Disable(String),
    Delete(String),
}

pub(crate) fn parse_schedule_command(argument: Option<&str>) -> Result<ScheduleCommand, String> {
    let Some(argument) = argument.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(ScheduleCommand::Open);
    };
    let (verb, id) = argument
        .split_once(char::is_whitespace)
        .map(|(verb, id)| (verb, id.trim()))
        .ok_or_else(|| "用法: /cron <enable|disable|delete> <id>".to_string())?;
    if id.is_empty() {
        return Err("cron job id is required".to_string());
    }
    match verb.to_ascii_lowercase().as_str() {
        "enable" => Ok(ScheduleCommand::Enable(id.to_string())),
        "disable" => Ok(ScheduleCommand::Disable(id.to_string())),
        "delete" => Ok(ScheduleCommand::Delete(id.to_string())),
        "run" => Err("Gateway 未提供 cron 立即运行接口".to_string()),
        _ => Err("用法: /cron <enable|disable|delete> <id>".to_string()),
    }
}
