//! OpenClaw settings routing with the existing host policy injected.
use gateway_openclaw_settings::{Authorization, Host};

pub(super) fn router() -> axum::Router {
    gateway_openclaw_settings::router(authorization())
}

fn authorization() -> Authorization {
    Authorization {
        settings: |path, headers, project, target, write| {
            super::access::authorize_openclaw_settings(path, headers, project, target, write)
                .map(|host| Host { id: host.id, home: host.home, credential_root: host.credential_root })
        },
        sensitive_env: |path, headers, project, target| {
            super::access::authorize_openclaw_env_read(path, headers, project, target).map(|_| ())
        },
    }
}

#[cfg(test)]
#[path = "openclaw_settings_tests.rs"]
mod tests;
