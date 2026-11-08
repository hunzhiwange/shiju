//! Resolve original model ownership from public catalog pointers.

use super::identity::metadata_model_key;
use std::collections::HashSet;
use vw_provider_resolver::models::{Model, ModelsSnapshot};

const GPT_CONTEXT_WINDOW_CAP: u64 = 272_000;

pub(super) fn find<'a>(snapshot: &'a ModelsSnapshot, model_id: &str) -> Option<&'a Model> {
    let normalized = metadata_model_key(model_id);
    if normalized.is_empty() {
        return None;
    }

    let mut vendors = HashSet::new();
    let mut pointed = None;
    let mut owned = Vec::new();
    for (provider_id, provider) in &snapshot.providers {
        // Shiju's local overlay cannot establish ownership in models.dev.
        if provider_id == "shiju" {
            continue;
        }
        for (id, model) in &provider.models {
            let canonical = model
                .canonical_model_id
                .as_deref()
                .and_then(|id| id.split_once('/'))
                .filter(|(vendor, id)| !vendor.is_empty() && !id.is_empty());
            if let Some((vendor, _)) = canonical {
                vendors.insert(vendor);
            }
            if metadata_model_key(id) != normalized {
                continue;
            }
            if let Some((vendor, canonical_id)) = canonical {
                if vendor != "shiju"
                    && let Some(target) = snapshot
                        .providers
                        .get(vendor)
                        .and_then(|provider| provider.models.get(canonical_id))
                {
                    if pointed.is_some_and(|(key, _)| key != (vendor, canonical_id)) {
                        return None;
                    }
                    pointed = Some(((vendor, canonical_id), target));
                    continue;
                }
                // A dangling foreign pointer cannot turn a hosted row into an original.
                if vendor != provider_id {
                    continue;
                }
            }
            owned.push((provider_id.as_str(), model));
        }
    }

    if let Some((_, model)) = pointed {
        return Some(model);
    }
    let mut candidates =
        owned.into_iter().filter_map(|(vendor, model)| vendors.contains(vendor).then_some(model));
    let model = candidates.next()?;
    candidates.next().is_none().then_some(model)
}

/// Borrow the subscription-safe window for GPT IDs. Deployment and user values bypass this cap.
pub(super) fn cap_gpt_context_window(model_id: &str, window: u64) -> u64 {
    let normalized = metadata_model_key(model_id);
    let is_gpt = normalized.match_indices("gpt-").any(|(offset, _)| {
        offset == 0 || !normalized.as_bytes()[offset - 1].is_ascii_alphanumeric()
    });
    if is_gpt { window.min(GPT_CONTEXT_WINDOW_CAP) } else { window }
}
