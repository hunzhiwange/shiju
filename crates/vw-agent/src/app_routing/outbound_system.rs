//! Read-only macOS proxy snapshot and application routing exceptions.

use axum::http::Uri;
use glob::{MatchOptions, Pattern};
use hyper_util::client::proxy::matcher::{Intercept, Matcher};
use reqwest::Url;
use std::net::IpAddr;
use system_configuration::{
    core_foundation::{
        ConcreteCFType,
        array::CFArray,
        base::{CFType, TCFType},
        dictionary::CFDictionary,
        number::CFNumber,
        string::CFString,
    },
    dynamic_store::SCDynamicStoreBuilder,
};

type Settings = CFDictionary<CFString, CFType>;

pub(super) struct SystemProxy {
    http: Option<Endpoint>,
    https: Option<Endpoint>,
    exceptions: Vec<Pattern>,
    networks: Matcher,
    exclude_simple: bool,
}

struct Endpoint {
    url: Url,
    intercept: Intercept,
}

impl Endpoint {
    fn parse(value: &str) -> Option<Self> {
        let url = if value.contains("://") {
            Url::parse(value).ok()?
        } else {
            Url::parse(&format!("http://{value}")).ok()?
        };
        let intercept = Matcher::builder()
            .all(url.as_str())
            .build()
            .intercept(&Uri::from_static("http://proxy-target.invalid"))?;
        Some(Self { url, intercept })
    }
}

impl SystemProxy {
    pub(super) fn read() -> Self {
        let settings =
            SCDynamicStoreBuilder::new("ShiJu routing").build().and_then(|s| s.get_proxies());
        Self::from_settings(settings.as_ref(), |key| std::env::var(key).ok())
    }

    pub(super) fn from_settings(
        settings: Option<&Settings>,
        env: impl Fn(&str) -> Option<String>,
    ) -> Self {
        let environment = |upper, lower| env(upper).or_else(|| env(lower));
        let all = environment("ALL_PROXY", "all_proxy");
        let no_proxy = environment("NO_PROXY", "no_proxy").unwrap_or_default();
        let mut networks = vec![no_proxy];
        let mut exceptions = Vec::new();
        let mut exclude_simple = false;
        if let Some(settings) = settings {
            exclude_simple = number(settings, "ExcludeSimpleHostnames") == Some(1);
            if let Some(entries) = setting::<CFArray>(settings, "ExceptionsList") {
                // SAFETY: SystemConfiguration returns a property-list array whose elements
                // are Core Foundation objects. Check each object's type below.
                let entries = unsafe {
                    CFArray::<CFType>::wrap_under_get_rule(entries.as_concrete_TypeRef())
                };
                for entry in &entries {
                    let Some(entry) = entry.downcast::<CFString>() else { continue };
                    let entry = entry.to_string();
                    let entry = entry.trim();
                    if entry.eq_ignore_ascii_case("<local>") {
                        exclude_simple = true;
                    } else if entry.contains('/') {
                        networks.push(entry.to_owned());
                    } else if let Ok(pattern) = Pattern::new(entry.trim_end_matches('.')) {
                        exceptions.push(pattern);
                    }
                }
            }
        }
        // Use the same NO_PROXY domain and CIDR matching as the existing HTTP clients.
        let networks = Matcher::builder().all("http://127.0.0.1").no(networks.join(",")).build();
        let endpoint = |upper, lower, kind| {
            if env("REQUEST_METHOD").is_some() {
                return None;
            }
            environment(upper, lower)
                .and_then(|value| Endpoint::parse(&value))
                .or_else(|| all.as_deref().and_then(Endpoint::parse))
                .or_else(|| native_endpoint(settings?, kind, "http"))
                .or_else(|| native_endpoint(settings?, "SOCKS", "socks5h"))
        };
        Self {
            http: endpoint("HTTP_PROXY", "http_proxy", "HTTP"),
            https: endpoint("HTTPS_PROXY", "https_proxy", "HTTPS"),
            exceptions,
            networks,
            exclude_simple,
        }
    }

    pub(super) fn url(&self, target: &Url) -> Option<&Url> {
        Some(&self.endpoint(target)?.url)
    }

    pub(super) fn intercept(&self, target: &Url) -> Option<&Intercept> {
        Some(&self.endpoint(target)?.intercept)
    }

    fn endpoint(&self, target: &Url) -> Option<&Endpoint> {
        let host = target.host_str()?.trim_end_matches('.');
        if (self.exclude_simple
            && matches!(target.host(), Some(url::Host::Domain(_)))
            && !host.contains('.'))
            || self.exceptions.iter().any(|pattern| {
                pattern.matches_with(
                    host,
                    MatchOptions { case_sensitive: false, ..MatchOptions::new() },
                )
            })
            || self.networks.intercept(&target.as_str().parse().ok()?).is_none()
        {
            return None;
        }
        match target.scheme() {
            "http" => self.http.as_ref(),
            "https" => self.https.as_ref(),
            _ => None,
        }
    }
}

fn setting<T: ConcreteCFType>(settings: &Settings, key: &str) -> Option<T> {
    settings.find(CFString::new(key))?.downcast::<T>()
}

fn number(settings: &Settings, key: &str) -> Option<i32> {
    setting::<CFNumber>(settings, key)?.to_i32()
}

fn native_endpoint(settings: &Settings, kind: &str, scheme: &str) -> Option<Endpoint> {
    if number(settings, &format!("{kind}Enable")) != Some(1) {
        return None;
    }
    let host = setting::<CFString>(settings, &format!("{kind}Proxy"))?.to_string();
    let port = u16::try_from(number(settings, &format!("{kind}Port"))?).ok()?;
    if port == 0 {
        return None;
    }
    let mut url = Url::parse(&format!("{scheme}://localhost")).ok()?;
    let host = host.trim();
    if host.parse::<std::net::Ipv6Addr>().is_ok() {
        url.set_host(Some(&format!("[{host}]"))).ok()?;
    } else {
        url.set_host(Some(host)).ok()?;
    }
    url.set_port(Some(port)).ok()?;
    Endpoint::parse(url.as_str())
}

pub(super) fn is_loopback(host: &str) -> bool {
    let host = host.trim_end_matches('.');
    let hostname = host.to_ascii_lowercase();
    hostname == "localhost"
        || hostname.ends_with(".localhost")
        || host
            .trim_start_matches('[')
            .trim_end_matches(']')
            .parse::<IpAddr>()
            .is_ok_and(|ip| ip.to_canonical().is_loopback())
}
