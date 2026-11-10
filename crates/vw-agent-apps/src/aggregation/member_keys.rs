use crate::providers::mode_state::StackState;

const KEY_BASE_MAX_LEN: usize = 24;

/// Reserve a routing key for a saved provider in the application's stack.
///
/// The reservation outlives both membership and the provider record. Callers
/// supply an authorized, validated provider and persist through the existing transaction.
pub fn allocate_key(stack: &mut StackState, id: &str, name: &str, icon: Option<&str>) -> String {
    if let Some((key, _)) = stack.keys.iter().find(|(_, owner)| owner.as_str() == id) {
        return key.clone();
    }

    let base = [icon.filter(|icon| !icon.starts_with("data:")), Some(name)]
        .into_iter()
        .flatten()
        .map(slug)
        .find(|candidate| !candidate.is_empty())
        .unwrap_or_else(|| {
            let id_prefix: String = slug(id).chars().filter(|&c| c != '-').take(6).collect();
            format!("p{id_prefix}")
        });

    let mut key = base.clone();
    let mut suffix = 2;
    while stack.keys.contains_key(&key) {
        key = format!("{base}-{suffix}");
        suffix += 1;
    }
    stack.keys.insert(key.clone(), id.to_owned());
    key
}

fn slug(text: &str) -> String {
    let mut key = String::new();
    for c in text.chars() {
        if c.is_ascii_alphanumeric() {
            key.push(c.to_ascii_lowercase());
        } else if !key.is_empty() && !key.ends_with('-') {
            key.push('-');
        }
        if key.len() == KEY_BASE_MAX_LEN {
            break;
        }
    }
    key.trim_end_matches('-').to_owned()
}
