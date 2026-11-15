//! URL 验证能力由 network-policy 提供。

pub use network_policy::{
    DomainPolicy, UrlSchemePolicy, extract_host, host_matches_allowlist, is_private_or_local_host,
    normalize_allowed_domains, normalize_domain, validate_url,
};
