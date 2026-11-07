//! Search and group presets for display, leaving the original preset intact.

use reqwest::Url;
use serde::Deserialize;
use std::{collections::HashMap, sync::LazyLock};

static FAMILIES: LazyLock<Vec<PresetFamily>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("presets/families.json")).expect("valid preset family data")
});

static ALIASES: LazyLock<HashMap<String, String>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("presets/search_aliases.json"))
        .expect("valid preset search aliases")
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PresetPlan {
    Payg,
    Coding,
    CodingPlan,
    AgentPlan,
    TokenPlan,
    EnterpriseLite,
    EnterprisePro,
    StepPlan,
    Aksk,
    ApiKey,
}

impl PresetPlan {
    fn label_key(self) -> &'static str {
        match self {
            Self::Payg => "providerPreset.plan.payg",
            Self::Coding => "providerPreset.plan.coding",
            Self::CodingPlan => "providerPreset.plan.codingPlan",
            Self::AgentPlan => "providerPreset.plan.agentPlan",
            Self::TokenPlan => "providerPreset.plan.tokenPlan",
            Self::EnterpriseLite => "providerPreset.plan.enterpriseLite",
            Self::EnterprisePro => "providerPreset.plan.enterprisePro",
            Self::StepPlan => "providerPreset.plan.stepPlan",
            Self::Aksk => "providerPreset.plan.aksk",
            Self::ApiKey => "providerPreset.plan.apiKey",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PresetRegion {
    Cn,
    Intl,
}

impl PresetRegion {
    fn label_key(self) -> &'static str {
        match self {
            Self::Cn => "providerPreset.region.cn",
            Self::Intl => "providerPreset.region.intl",
        }
    }
}

pub struct PresetSearchEntry<'a, T> {
    pub preset: T,
    pub name: &'a str,
    pub name_key: Option<&'a str>,
    pub website_url: Option<&'a str>,
    pub family: Option<&'a str>,
    pub plan: Option<PresetPlan>,
    pub region: Option<PresetRegion>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PresetFamily {
    pub id: String,
    pub name: String,
    pub name_key: Option<String>,
    #[serde(default)]
    plan_order: Vec<PresetPlan>,
}

pub struct PresetGroup<'a, T> {
    pub family: Option<&'static PresetFamily>,
    pub versions: Vec<&'a PresetSearchEntry<'a, T>>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum PresetMatch {
    All,
    /// Indices in the group's sorted version list.
    Versions(Vec<usize>),
}

pub struct PresetSearchHit<'a, T> {
    pub group: PresetGroup<'a, T>,
    pub matched: PresetMatch,
}

/// Groups the visible presets of one application in source row order.
pub fn group_presets<'a, T>(entries: &'a [PresetSearchEntry<'a, T>]) -> Vec<PresetGroup<'a, T>> {
    let mut groups: Vec<PresetGroup<'a, T>> = Vec::new();
    let mut positions: HashMap<&str, usize> = HashMap::new();
    for entry in entries {
        let family = entry.family.and_then(|id| FAMILIES.iter().find(|family| family.id == id));
        if let Some(family) = family {
            if let Some(&position) = positions.get(family.id.as_str()) {
                groups[position].versions.push(entry);
                continue;
            }
            positions.insert(&family.id, groups.len());
        }
        groups.push(PresetGroup { family, versions: vec![entry] });
    }

    for group in &mut groups {
        if group.versions.len() == 1 {
            group.family = None;
        } else if let Some(family) = group.family {
            sort_versions(&mut group.versions, family);
        }
    }
    groups
}

fn sort_versions<T>(versions: &mut [&PresetSearchEntry<'_, T>], family: &PresetFamily) {
    let mut plans = family.plan_order.clone();
    for version in versions.iter() {
        if let Some(plan) = version.plan
            && !plans.contains(&plan)
        {
            plans.push(plan);
        }
    }
    versions.sort_by_key(|entry| {
        let plan_rank =
            plans.iter().position(|plan| Some(*plan) == entry.plan).unwrap_or(plans.len());
        (plan_rank, entry.region)
    });
}

/// Searches the visible presets of one application. The translator supplies localized
/// display text when available. Hits borrow the original entries and retain every version.
pub fn search_presets<'a, T>(
    entries: &'a [PresetSearchEntry<'a, T>],
    query: &str,
    translate: impl Fn(&str) -> Option<String>,
) -> Vec<PresetSearchHit<'a, T>> {
    let query = query.to_lowercase();
    let terms: Vec<_> = query.split_whitespace().collect();
    group_presets(entries)
        .into_iter()
        .filter_map(|group| {
            let matched = if terms.is_empty() {
                PresetMatch::All
            } else {
                match_group(&group, &terms, &translate)?
            };
            Some(PresetSearchHit { group, matched })
        })
        .collect()
}

fn match_group<T>(
    group: &PresetGroup<'_, T>,
    terms: &[&str],
    translate: &impl Fn(&str) -> Option<String>,
) -> Option<PresetMatch> {
    if let Some(family) = group.family {
        let name = family.name_key.as_deref().and_then(translate);
        let main_domain = preset_domain(group.versions[0].website_url);
        if fields_match(
            terms,
            &[name.as_deref().unwrap_or(&family.name), domain_body(&main_domain)],
            &main_domain,
        ) {
            return Some(PresetMatch::All);
        }

        let mut hits = Vec::new();
        for (index, entry) in group.versions.iter().enumerate() {
            let display_name = entry.name_key.and_then(translate).unwrap_or_default();
            let plan = entry.plan.and_then(|plan| translate(plan.label_key()));
            let region = entry.region.and_then(|region| translate(region.label_key()));
            let domain = preset_domain(entry.website_url);
            let own_domain = if domain == main_domain { "" } else { &domain };
            let texts = [
                entry.name,
                &display_name,
                plan.as_deref().unwrap_or_default(),
                region.as_deref().unwrap_or_default(),
                domain_body(own_domain),
            ];
            if fields_match(terms, &texts, own_domain) {
                hits.push(index);
            }
        }
        if !hits.is_empty() {
            return Some(PresetMatch::Versions(hits));
        }
    }

    group
        .versions
        .iter()
        .any(|entry| {
            let display_name = entry.name_key.and_then(translate).unwrap_or_default();
            let domain = preset_domain(entry.website_url);
            let aliases = ALIASES.get(entry.name).map_or("", String::as_str);
            fields_match(
                terms,
                &[entry.name, &display_name, domain_body(&domain), aliases],
                &domain,
            )
        })
        .then_some(PresetMatch::All)
}

fn fields_match(terms: &[&str], texts: &[&str], domain: &str) -> bool {
    let haystack = texts.join(" ").to_lowercase();
    terms
        .iter()
        .all(|term| haystack.contains(term) || (term.contains('.') && domain.contains(term)))
}

fn preset_domain(website_url: Option<&str>) -> String {
    let Some(url) = website_url.and_then(|value| Url::parse(value).ok()) else {
        return String::new();
    };
    let host = url.host_str().unwrap_or_default();
    host.strip_prefix("www.").unwrap_or(host).to_lowercase()
}

fn domain_body(mut domain: &str) -> &str {
    if let Some((prefix, rest)) = domain.split_once('.')
        && matches!(
            prefix,
            "www" | "api" | "platform" | "open" | "console" | "cloud" | "dashboard" | "app"
        )
    {
        domain = rest;
    }
    while let Some((body, suffix)) = domain.rsplit_once('.') {
        if !matches!(
            suffix,
            "com"
                | "cn"
                | "ai"
                | "io"
                | "net"
                | "org"
                | "dev"
                | "app"
                | "top"
                | "xyz"
                | "cc"
                | "co"
                | "me"
                | "tech"
                | "site"
                | "pro"
                | "vip"
                | "us"
                | "hk"
                | "jp"
        ) {
            break;
        }
        domain = body;
    }
    domain
}
