use crate::TerminalState;
use crate::panes::TerminalPanes;

impl<S: Copy + Into<&'static str>> TerminalState<S> {
    pub fn groups(&self) -> impl Iterator<Item = (u64, bool)> + '_ {
        self.parked_groups
            .range(..self.active_group)
            .map(|(&id, _)| (id, false))
            .chain(std::iter::once((self.active_group, true)))
            .chain(self.parked_groups.range(self.active_group..).map(|(&id, _)| (id, false)))
    }

    pub fn group_name(&self, id: u64) -> Option<&str> {
        self.group_names.get(&id).map(String::as_str)
    }

    pub(crate) fn has_group(&self, id: u64) -> bool {
        id == self.active_group || self.parked_groups.contains_key(&id)
    }

    pub(crate) fn rename_group(&mut self, id: u64, title: &str) {
        let title = title.trim();
        if self.has_group(id) && !title.is_empty() {
            self.group_names.insert(id, title.to_owned());
        }
    }

    pub fn new_group(&mut self) -> bool {
        let last = self.parked_groups.last_key_value().map_or(0, |(&id, _)| id);
        let Some(id) = last.max(self.active_group).checked_add(1) else {
            return false;
        };
        self.parked_groups.insert(id, TerminalPanes::default());
        self.select_group(id)
    }

    pub fn select_group(&mut self, id: u64) -> bool {
        let Some(next) = self.parked_groups.remove(&id) else {
            return false;
        };
        self.panes.split_menu = None;
        self.group_menu = None;
        self.group_rename = None;
        let old = std::mem::replace(&mut self.panes, next);
        self.parked_groups.insert(self.active_group, old);
        self.active_group = id;
        self.active_id = self.panes.grid.get(self.panes.focused).and_then(|pane| pane.active_id);
        self.tab_context_menu_id = None;
        self.tab_context_menu_pos = None;
        self.content_menu = None;
        true
    }

    pub fn close_group(&mut self, id: u64) -> bool {
        if id == self.active_group {
            let Some((&next, _)) = self
                .parked_groups
                .range(id..)
                .next()
                .or_else(|| self.parked_groups.last_key_value())
            else {
                return false;
            };
            self.select_group(next);
        }
        let Some(closed) = self.parked_groups.remove(&id) else {
            return false;
        };
        self.group_menu = None;
        self.group_names.remove(&id);
        for id in closed.grid.iter().flat_map(|(_, pane)| pane.tabs.iter()) {
            self.close_terminal(*id);
        }
        true
    }
}
