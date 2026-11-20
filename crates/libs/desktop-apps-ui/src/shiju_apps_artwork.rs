//! Load public artwork without forwarding account credentials.
use iced::Task;

use super::{Input, ShijuAppsMessage};

#[cfg(target_arch = "wasm32")]
pub(super) fn load(_: &mut Input<'_>) -> Task<ShijuAppsMessage> {
    Task::none()
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn load(app: &mut Input<'_>) -> Task<ShijuAppsMessage> {
    use desktop_app_launch::{ShijuAppsLoadState::Loaded, ShijuAppsState, ShijuDetailMetadata};
    use iced::futures::{StreamExt, stream};

    let state = &mut app.shiju_apps;
    let Some(account_id) = state.account_id.clone() else { return Task::none() };
    let mut urls = Vec::new();
    if let Loaded(entries) = &state.featured {
        for entry in entries {
            urls.push((Some(entry.cover_url.as_str()), 512));
            urls.push((entry.background_url.as_deref(), 512));
        }
    }
    if let Loaded(catalog) = &state.store {
        for entry in catalog.apps.iter().take(36) {
            urls.push((entry.presentation.cover_url.as_deref(), 512));
            urls.push((entry.presentation.background_url.as_deref(), 512));
        }
    }
    if let Loaded(mine) = &state.mine {
        for entry in mine.apps.iter().take(60) {
            urls.push((entry.icon_url.as_deref(), 128));
        }
    }
    if let Loaded(created) = &state.created {
        for entry in created.iter().take(40) {
            urls.push((entry.icon_url.as_deref(), 128));
        }
    }
    if let Loaded(detail) = &state.detail_metadata {
        let url = match detail {
            ShijuDetailMetadata::Development(item) => item.icon_url.as_deref(),
            ShijuDetailMetadata::Installation(item) => item.icon_url.as_deref(),
        };
        urls.push((url, 128));
    }
    let requests: Vec<_> = urls
        .into_iter()
        .filter_map(|(url, size)| {
            let raw = url?;
            let url = public_image_url(raw)?;
            let key = ShijuAppsState::artwork_key(raw);
            if state.artwork.contains_key(&key) || state.artwork.len() >= 128 {
                return None;
            }
            state.artwork.insert(key, None);
            Some((key, url, size))
        })
        .collect();
    if requests.is_empty() {
        return Task::none();
    }
    let Ok(client) = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(8))
        .build()
    else {
        return Task::none();
    };
    Task::perform(
        async move {
            stream::iter(requests)
                .map(|(key, url, size)| {
                    let client = client.clone();
                    async move { (key, download(&client, url, size).await) }
                })
                .buffer_unordered(6)
                .collect()
                .await
        },
        move |items| ShijuAppsMessage::ArtworkLoaded { account_id, items },
    )
}

#[cfg(not(target_arch = "wasm32"))]
fn public_image_url(raw: &str) -> Option<reqwest::Url> {
    let url = reqwest::Url::parse(raw).ok()?;
    let host = url.host_str()?;
    let shiju = host == "shiju.cloud" || host.ends_with(".shiju.cloud");
    let oss = host
        .strip_suffix(".aliyuncs.com")
        .and_then(|name| name.split_once('.'))
        .is_some_and(|(_, endpoint)| endpoint.starts_with("oss-") && !endpoint.contains('.'));
    (url.scheme() == "https"
        && url.port().is_none()
        && url.username().is_empty()
        && url.password().is_none()
        && (shiju || oss))
        .then_some(url)
}

#[cfg(not(target_arch = "wasm32"))]
async fn download(
    client: &reqwest::Client,
    url: reqwest::Url,
    size: u32,
) -> Option<iced::widget::image::Handle> {
    const MAX_BYTES: usize = 4 * 1024 * 1024;
    let mut response = client.get(url).send().await.ok()?.error_for_status().ok()?;
    if response.content_length().is_some_and(|bytes| bytes > MAX_BYTES as u64) {
        return None;
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.ok()? {
        if bytes.len() + chunk.len() > MAX_BYTES {
            return None;
        }
        bytes.extend_from_slice(&chunk);
    }
    let mut reader =
        image::ImageReader::new(std::io::Cursor::new(bytes)).with_guessed_format().ok()?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(4096);
    limits.max_image_height = Some(4096);
    limits.max_alloc = Some(48 * 1024 * 1024);
    reader.limits(limits);
    let pixels = reader.decode().ok()?.thumbnail(size, size).into_rgba8();
    Some(iced::widget::image::Handle::from_rgba(pixels.width(), pixels.height(), pixels.into_raw()))
}
