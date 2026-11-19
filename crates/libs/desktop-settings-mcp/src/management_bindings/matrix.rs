//! Matrix inputs carry exact record revisions and installation identities.
use super::*;
use vw_api_app_identity::AppId;

pub const APPS: [AppId; 12] = [
    AppId::Claude,
    AppId::Codex,
    AppId::Gemini,
    AppId::GrokBuild,
    AppId::OpenCode,
    AppId::Hermes,
    AppId::Pi,
    AppId::Mcode,
    AppId::Kimi,
    AppId::Dsh,
    AppId::Agy,
    AppId::Omp,
];
pub const NAME_WIDTH: f32 = 260.0;
pub const CELL_WIDTH: f32 = 190.0;

#[derive(Debug, Clone)]
pub enum Action {
    Search(String),
    Cell { record: McpRecordRef, target: McpAppTarget, enabled: bool },
    Row { record: McpRecordRef, enabled: bool },
    Column { target: McpAppTarget, enabled: bool },
    Retry,
    Undo,
}
#[derive(Debug, Clone)]
pub struct Change {
    pub record: McpCatalogRecord,
    pub target: McpAppTarget,
    pub enabled: bool,
    pub previous: bool,
}
#[derive(Debug, Clone)]
pub struct Attempt {
    pub change: Change,
    pub result: Result<McpDistributionItem, String>,
}
#[derive(Debug, Clone, Default)]
pub struct State {
    pub query: String,
    pub pending: Vec<Change>,
    pub attempts: Vec<Attempt>,
    pub undo: Vec<Change>,
}
pub fn visible_records<'a>(
    catalog: &'a McpCatalogResponse,
    query: &str,
) -> Vec<&'a McpCatalogRecord> {
    let query = query.trim().to_lowercase();
    catalog
        .records
        .iter()
        .filter(|r| {
            r.record.id.0.to_lowercase().contains(&query)
                || r.name.to_lowercase().contains(&query)
                || r.public_summary.as_deref().unwrap_or("").to_lowercase().contains(&query)
        })
        .collect()
}
/// Each trusted installation gets its own column; preferences never alter stored bindings.
pub fn columns(
    targets: &[McpAppTarget],
    shown: &[AppId],
    host: &str,
) -> Vec<(AppId, Option<McpAppTarget>)> {
    let mut columns = Vec::new();
    for app in APPS.into_iter().filter(|app| shown.contains(app)) {
        let mut found = Vec::new();
        for target in targets.iter().filter(|t| t.app_id == app && t.target_host_id == host) {
            if !found.contains(target) {
                found.push(target.clone());
            }
        }
        if found.is_empty() {
            columns.push((app, None));
        } else {
            columns.extend(found.into_iter().map(|t| (app, Some(t))));
        }
    }
    columns
}
pub fn plan(
    catalog: &McpCatalogResponse,
    targets: &[McpAppTarget],
    state: &State,
    action: &Action,
) -> Result<Vec<Change>, String> {
    let mut changes: Vec<Change> = Vec::new();
    let mut add = |record: &McpCatalogRecord,
                   target: &McpAppTarget,
                   enabled|
     -> Result<(), String> {
        if !targets.contains(target)
            || !crate::distribution::supported(catalog, std::slice::from_ref(record), target.app_id)
        {
            return Err("目标无可信实例或传输能力不可用".into());
        }
        if changes.iter().any(|c| {
            c.record.record == record.record
                && c.record.scope == record.scope
                && c.target == *target
        }) {
            return Ok(());
        }
        let previous = catalog
            .bindings
            .iter()
            .find(|b| b.record.id == record.record.id && b.target == *target)
            .is_some_and(|b| b.enabled);
        changes.push(Change { record: record.clone(), target: target.clone(), enabled, previous });
        Ok(())
    };
    match action {
        Action::Cell { record, target, enabled } => {
            let r = catalog.records.iter().find(|r| r.record == *record).ok_or("目录修订已变更")?;
            add(r, target, *enabled)?;
        }
        Action::Row { record, enabled } => {
            let r = catalog.records.iter().find(|r| r.record == *record).ok_or("目录修订已变更")?;
            for target in targets.iter().filter(|t| APPS.contains(&t.app_id)) {
                if crate::distribution::supported(catalog, std::slice::from_ref(r), target.app_id) {
                    add(r, target, *enabled)?;
                }
            }
        }
        Action::Column { target, enabled } => {
            for r in visible_records(catalog, &state.query) {
                if crate::distribution::supported(catalog, std::slice::from_ref(r), target.app_id) {
                    add(r, target, *enabled)?;
                }
            }
        }
        Action::Retry => {
            for attempt in &state.attempts {
                let Ok(item) = &attempt.result else {
                    continue;
                }; // Unknown writes require a refresh, never blind replay.
                if !matches!(item.native_outcome, McpDistributionOutcome::Failed { .. }) {
                    continue;
                }
                let latest = state
                    .attempts
                    .iter()
                    .rev()
                    .filter(|a| a.change.record.scope == attempt.change.record.scope)
                    .filter_map(|a| a.result.as_ref().ok())
                    .find(|i| i.record.id == item.record.id && i.desired_committed)
                    .map(|i| &i.record)
                    .unwrap_or(&item.record);
                let r = catalog
                    .records
                    .iter()
                    .find(|r| r.record == *latest && r.scope == attempt.change.record.scope)
                    .ok_or("失败项修订已变更，请刷新核实")?;
                add(r, &item.target, item.enabled)?;
            }
        }
        Action::Undo => {
            for change in &state.undo {
                let r = catalog
                    .records
                    .iter()
                    .find(|r| r.record == change.record.record && r.scope == change.record.scope)
                    .ok_or("撤销遇到中途修订冲突，请刷新核实")?;
                add(r, &change.target, change.enabled)?;
            }
        }
        Action::Search(_) => {}
    }
    if changes.is_empty() {
        return Err("没有可提交的目标".into());
    }
    Ok(changes)
}
/// Single-record requests are below every bulk limit and preserve actual scope.
/// Carry forward only revisions observed from a committed write in this operation.
pub async fn execute(client: GatewayClient, changes: Vec<Change>, token: String) -> Vec<Attempt> {
    let mut attempts: Vec<Attempt> = Vec::new();
    for mut change in changes {
        for previous in &attempts {
            if previous.change.record.scope == change.record.scope
                && previous.change.record.record.id == change.record.record.id
            {
                if let Ok(item) = &previous.result {
                    if item.desired_committed {
                        change.record.record = item.record.clone();
                    }
                }
            }
        }
        let result = apply(
            client.clone(),
            vec![change.record.clone()],
            change.target.clone(),
            change.enabled,
            token.clone(),
        )
        .await
        .and_then(|r| r.items.into_iter().next().ok_or_else(|| "响应缺少目标".into()));
        attempts.push(Attempt { change, result });
    }
    attempts
}
impl State {
    pub fn finish(&mut self, attempts: Vec<Attempt>) {
        self.pending.clear();
        self.undo = attempts
            .iter()
            .filter_map(|a| {
                let item = a.result.as_ref().ok()?;
                if !item.desired_committed || a.change.previous == item.enabled {
                    return None;
                }
                let mut change = a.change.clone();
                // Other targets in the same operation may have advanced this record again.
                change.record.record = attempts
                    .iter()
                    .rev()
                    .filter(|b| b.change.record.scope == a.change.record.scope)
                    .filter_map(|b| b.result.as_ref().ok())
                    .find(|b| b.record.id == item.record.id && b.desired_committed)?
                    .record
                    .clone();
                change.enabled = a.change.previous;
                change.previous = item.enabled;
                Some(change)
            })
            .collect();
        self.attempts = attempts;
    }
}

/// Toolbar stays outside the horizontal scroller; task 7 maps edit/menu actions and icons.
#[derive(Debug, Clone)]
pub enum ViewMessage {
    Binding(Action),
    Edit(McpRecordRef),
    Menu(McpRecordRef),
}
pub fn panel<'a>(
    catalog: Option<&'a Result<McpCatalogResponse, String>>,
    state: &'a State,
    targets: &[McpAppTarget],
    shown: &[AppId],
    host: &str,
    loading: bool,
    running: bool,
) -> iced::Element<'a, ViewMessage> {
    use desktop_ui_controls::system_settings_common::*;
    use iced::{
        Length,
        widget::{column, container, row, scrollable, text, text_input},
    };
    let search = text_input("搜索 ID、名称、命令或地址摘要", &state.query)
        .on_input(|q| ViewMessage::Binding(Action::Search(q)))
        .style(settings_text_input_style)
        .padding(SETTINGS_CONTROL_PADDING)
        .size(SETTINGS_CONTROL_TEXT_SIZE);
    let mut body = column![search].spacing(10);
    if loading {
        return body.push(text("正在加载目录…")).into();
    }
    let catalog = match catalog {
        Some(Ok(c)) => c,
        Some(Err(e)) => return body.push(settings_error_banner(e.clone())).into(),
        None => return body.push(text("尚未加载目录")).into(),
    };
    let records = visible_records(catalog, &state.query);
    body = body.push(text(format!("显示 {} / {} 项", records.len(), catalog.records.len())));
    if records.is_empty() {
        return body
            .push(text(if catalog.records.is_empty() {
                "目录为空，请添加 MCP"
            } else {
                "没有匹配的 MCP"
            }))
            .into();
    }
    let columns = columns(targets, shown, host);
    let mut header = row![text("MCP 名称 / 连接摘要").width(NAME_WIDTH)].spacing(8);
    for (app, target) in &columns {
        let count = target
            .as_ref()
            .map(|t| {
                records
                    .iter()
                    .filter(|r| {
                        catalog
                            .bindings
                            .iter()
                            .any(|b| b.record.id == r.record.id && b.target == *t && b.enabled)
                    })
                    .count()
            })
            .unwrap_or(0);
        let mut cell = column![text(format!("{app:?} · {count}"))].spacing(4);
        if let Some(target) = target {
            cell = cell.push(text(target.installation_id.0.clone()).size(11));
            for (label, enabled) in [("启用筛选项", true), ("关闭筛选项", false)] {
                cell = cell.push(crate::toolbar_button(
                    label,
                    (!running).then(|| {
                        ViewMessage::Binding(Action::Column { target: target.clone(), enabled })
                    }),
                ));
            }
        } else {
            cell = cell.push(text("无当前主机可信实例").size(12));
        }
        header = header.push(cell.width(CELL_WIDTH));
    }
    header = header.push(text("操作").width(260));
    let mut grid = column![header].spacing(8);
    for record in records {
        let mut line = row![
            column![
                text(record.name.clone()),
                text(record.record.id.0.clone()).size(11),
                text(record.public_summary.clone().unwrap_or_default()).size(12)
            ]
            .width(NAME_WIDTH)
        ]
        .spacing(8);
        for (_, target) in &columns {
            let mut cell = column![].spacing(4).width(CELL_WIDTH);
            if let Some(target) = target {
                let observed = catalog
                    .bindings
                    .iter()
                    .find(|b| b.record.id == record.record.id && b.target == *target);
                let enabled = observed.is_some_and(|b| b.enabled);
                cell = cell
                    .push(text(format!("期望{}", if enabled { "启用" } else { "关闭" })).size(12));
                cell = cell.push(
                    text(
                        observed
                            .map(|b| format!("上次观察：{:?}", b.native_status))
                            .unwrap_or_else(|| "上次观察：未验证".into()),
                    )
                    .size(11),
                );
                if state
                    .pending
                    .iter()
                    .any(|c| c.record.record.id == record.record.id && c.target == *target)
                    && running
                {
                    cell = cell.push(text("原生写入中…").size(12));
                } else if let Some(attempt) = state.attempts.iter().rev().find(|a| {
                    a.change.record.record.id == record.record.id && a.change.target == *target
                }) {
                    cell = cell.push(
                        text(match &attempt.result {
                            Ok(i) => crate::distribution::item_label(i),
                            Err(e) => e.clone(),
                        })
                        .size(11),
                    );
                }
                let supported = crate::distribution::supported(
                    catalog,
                    std::slice::from_ref(record),
                    target.app_id,
                );
                cell = cell.push(crate::toolbar_button(
                    if enabled { "关闭" } else { "启用" },
                    (!running && supported).then(|| {
                        ViewMessage::Binding(Action::Cell {
                            record: record.record.clone(),
                            target: target.clone(),
                            enabled: !enabled,
                        })
                    }),
                ));
                if !supported {
                    cell = cell.push(text("传输不支持或能力未验证").size(11));
                }
            } else {
                cell = cell.push(text("无可信安装实例").size(12));
            }
            line = line.push(cell);
        }
        let actions = column![
            crate::toolbar_button(
                "编辑",
                (!running).then(|| ViewMessage::Edit(record.record.clone()))
            ),
            crate::toolbar_button(
                "更多",
                (!running).then(|| ViewMessage::Menu(record.record.clone()))
            ),
            crate::toolbar_button(
                "整行启用",
                (!running).then(|| ViewMessage::Binding(Action::Row {
                    record: record.record.clone(),
                    enabled: true
                }))
            ),
            crate::toolbar_button(
                "整行关闭",
                (!running).then(|| ViewMessage::Binding(Action::Row {
                    record: record.record.clone(),
                    enabled: false
                }))
            )
        ]
        .spacing(4)
        .width(260);
        grid = grid.push(line.push(actions));
    }
    let scroll = scrollable(container(grid).padding([0, 7])).direction(
        scrollable::Direction::Horizontal(scrollable::Scrollbar::new().width(4).scroller_width(4)),
    );
    body = body.push(scroll.width(Length::Fill)).push(
        row![
            crate::toolbar_button(
                "重试失败项",
                (!running && !state.attempts.is_empty())
                    .then_some(ViewMessage::Binding(Action::Retry))
            ),
            crate::toolbar_button(
                "撤销已提交差量",
                (!running && !state.undo.is_empty()).then_some(ViewMessage::Binding(Action::Undo))
            )
        ]
        .spacing(8),
    );
    body.into()
}
