use std::collections::HashSet;

pub fn merge_project_sessions(
    existing: Option<Vec<session_info::Info>>,
    incoming: Vec<session_info::Info>,
) -> Vec<session_info::Info> {
    let Some(existing) = existing else {
        return incoming;
    };

    let mut seen: HashSet<String> = incoming.iter().map(|session| session.id.clone()).collect();
    let mut merged = incoming;
    for session in existing {
        if seen.insert(session.id.clone()) {
            merged.push(session);
        }
    }
    merged
}

#[cfg(test)]
#[path = "session_open_tests.rs"]
mod tests;
