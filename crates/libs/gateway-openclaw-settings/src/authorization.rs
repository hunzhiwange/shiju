use crate::{Authorization, Host};
fn authorization() -> Authorization {
    Authorization {
        settings: |path, headers, project, target, write| {
            app_access_service::access::authorize_openclaw_settings(path, headers, project, target, write)
                .map(|host| Host { id: host.id, home: host.home, credential_root: host.credential_root })
        },
        sensitive_env: |path, headers, project, target| {
            app_access_service::access::authorize_openclaw_env_read(path, headers, project, target).map(|_| ())
        },
    }
}

pub fn authorized_router() -> axum::Router { crate::router(authorization()) }
#[cfg(test)]
#[path = "host_migration_16_tests.rs"]
mod host_migration_16_tests;
