//! Archived session tree validation and descendant-first deletion ordering.

use std::path::PathBuf;
use session_info::Info;

fn canonical_directory(directory: &str) -> PathBuf {
    std::fs::canonicalize(directory).unwrap_or_else(|_| PathBuf::from(directory))
}

pub fn delete_order(root: &Info, sessions: &[Info]) -> anyhow::Result<Vec<String>> {
    use std::collections::{BTreeMap, BTreeSet};

    anyhow::ensure!(
        root.parent_id.is_none() && root.time.archived.is_some() && root.is_user_navigable(),
        "archived deletion requires a user-navigable archived root"
    );
    let mut records = BTreeMap::<&str, Vec<&Info>>::new();
    let mut children = BTreeMap::<&str, BTreeSet<&str>>::new();
    for info in sessions {
        records.entry(&info.id).or_default().push(info);
        if let Some(parent) = info.parent_id.as_deref() {
            children.entry(parent).or_default().insert(&info.id);
        }
    }
    let directory = canonical_directory(&root.directory);
    let mut visited = BTreeSet::new();
    let mut stack = vec![(root.id.as_str(), false)];
    let mut order = Vec::new();
    while let Some((id, expanded)) = stack.pop() {
        if expanded {
            order.push(id.to_owned());
            continue;
        }
        anyhow::ensure!(visited.insert(id), "cycle in archived session tree");
        let matches =
            records.get(id).ok_or_else(|| anyhow::anyhow!("session tree node missing"))?;
        let info = matches[0];
        let value = serde_json::to_value(info)?;
        for duplicate in matches.iter().skip(1) {
            anyhow::ensure!(
                serde_json::to_value(duplicate)? == value,
                "ambiguous session ID in archived session tree"
            );
        }
        if id == root.id {
            anyhow::ensure!(value == serde_json::to_value(root)?, "archived root changed");
        }
        anyhow::ensure!(
            info.project_id == root.project_id && canonical_directory(&info.directory) == directory,
            "archived session descendant belongs to another project or directory"
        );
        stack.push((id, true));
        if let Some(descendants) = children.get(id) {
            for child in descendants.iter().rev() {
                stack.push((child, false));
            }
        }
    }
    Ok(order)
}

#[cfg(test)]
#[path = "archived_delete_tests.rs"]
mod tests;
