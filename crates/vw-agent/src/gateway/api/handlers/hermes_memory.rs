//! Compose Hermes routes with the host's existing scoped authorization.
pub(super) fn router() -> axum::Router {
    gateway_hermes_memory::router(|path, headers, project, target, write| {
        let host = super::access::authorize_hermes_memory(path, headers, project, target, write)?;
        Ok(gateway_hermes_memory::AuthorizedHost {
            id: host.id,
            home: host.home,
            credential_root: host.credential_root,
        })
    })
}

#[cfg(test)]
#[path = "hermes_memory_tests.rs"]
mod tests;
