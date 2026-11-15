//! Reuse scoped domain inventories; never turn an empty choice into select-all.
use desktop_ui_controls::system_settings_common::settings_checkbox_style;
use iced::{
    Element,
    widget::{checkbox, column, text},
};
use vw_api_project_profiles::{ProjectProfile, ProjectProfileScope};
use gateway_device_sync::DeviceSyncSelection;
use vw_api_app_identity::AppId;

#[derive(Clone)]
pub enum Choice {
    Preferences,
    Provider(AppId, String),
    Mcp(String),
    Prompt(AppId, String),
    Skill(String),
    Repository(String, String),
    Profile(ProjectProfile),
}
impl Choice {
    fn checked(&self, s: &DeviceSyncSelection) -> bool {
        match self {
            Self::Preferences => s.preferences,
            Self::Provider(a, id) => s.providers.contains(&(*a, id.clone())),
            Self::Mcp(id) => s.mcp.contains(id),
            Self::Prompt(a, id) => s.prompts.contains(&(*a, id.clone())),
            Self::Skill(id) => s.skills.contains(id),
            Self::Repository(a, b) => s.repositories.contains(&(a.clone(), b.clone())),
            Self::Profile(p) => s.profiles.contains(&p.id.0),
        }
    }
    fn apply(&self, s: &mut DeviceSyncSelection, selected: bool) {
        fn toggle<T: PartialEq>(v: &mut Vec<T>, item: T, on: bool) {
            v.retain(|x| x != &item);
            if on {
                v.push(item);
            }
        }
        match self {
            Self::Preferences => s.preferences = selected,
            Self::Provider(a, id) => toggle(&mut s.providers, (*a, id.clone()), selected),
            Self::Mcp(id) => toggle(&mut s.mcp, id.clone(), selected),
            Self::Prompt(a, id) => toggle(&mut s.prompts, (*a, id.clone()), selected),
            Self::Skill(id) => toggle(&mut s.skills, id.clone(), selected),
            Self::Repository(a, b) => toggle(&mut s.repositories, (a.clone(), b.clone()), selected),
            Self::Profile(p) => {
                toggle(&mut s.profiles, p.id.0.clone(), selected);
                if selected {
                    for group in ProjectProfileScope::ALL {
                        let app = group.app_id();
                        if let Some(id) = p.payload.providers.get(group) {
                            toggle(&mut s.providers, (app, id.clone()), true);
                        }
                        if let Some(ids) = p.payload.mcp.get(group) {
                            for id in ids {
                                toggle(&mut s.mcp, id.clone(), true);
                            }
                        }
                        if let Some(id) = p.payload.prompts.get(group) {
                            if !id.is_empty() {
                                toggle(&mut s.prompts, (app, id.clone()), true);
                            }
                        }
                    }
                }
            }
        }
    }
}
/// An inventory row already checked against the active host and project by the shell.
pub struct SelectionRow {
    label: String,
    choice: Choice,
    available: bool,
}
impl SelectionRow {
    pub fn new(label: String, choice: Choice, available: bool) -> Self {
        Self { label, choice, available }
    }
}
pub fn view(
    inventory: Vec<SelectionRow>,
    selection: &DeviceSyncSelection,
    enabled: bool,
) -> Element<'static, DeviceSyncSelection> {
    let mut rows = column![text("同步范围（明确选择）"), text("库存来自对应设置页的已加载列表；缺少项目请先在对应页面刷新。组合会包含其供应商、MCP 和规则引用，取消组合后可逐项取消依赖。").size(12)].spacing(8);
    let mut add = |label: String, choice: Choice, available: bool| {
        let mut control =
            checkbox(choice.checked(selection)).label(label).style(settings_checkbox_style);
        if enabled && available {
            let current = selection.clone();
            control = control.on_toggle(move |on| {
                let mut next = current.clone();
                choice.apply(&mut next, on);
                next
            });
        }
        rows = std::mem::replace(&mut rows, column![]).push(control);
    };
    add("偏好设置".into(), Choice::Preferences, true);
    for row in inventory { add(row.label, row.choice, row.available); }
    rows.into()
}
