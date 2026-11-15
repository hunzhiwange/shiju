//! Request-owned ACP CLI launch configuration.
pub mod profile;
pub mod gateway;
pub mod drivers;

/// Process-keyed one-way digest: cache diagnostics cannot disclose or guess credentials.
pub fn fingerprint(bytes: &[u8]) -> String {
    static KEY: std::sync::LazyLock<ring::hmac::Key> = std::sync::LazyLock::new(|| {
        let mut key = [0; 32];
        getrandom::fill(&mut key).expect("ACP cache fingerprint entropy unavailable");
        ring::hmac::Key::new(ring::hmac::HMAC_SHA256, &key)
    });
    hex::encode(ring::hmac::sign(&KEY, bytes).as_ref())
}

pub fn launch_identity(
    tenant: vw_api_transport::TenantScope,
    project: String,
    agent: &str,
    model: &model_types::Model,
    session: &str,
) -> model_transport::RelayIdentity {
    model_transport::RelayIdentity {
        tenant,
        project,
        session: format!("{session}:{agent}"),
        provider: model.provider_id.clone(),
        model: model.id.clone(),
        generation: 0,
    }
}
