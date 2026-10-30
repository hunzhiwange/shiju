//! 有界工具目录分页；拒绝循环游标，不返回不完整目录。
use anyhow::{Result, bail};
use rmcp::{
    Peer, RoleClient,
    model::{PaginatedRequestParam, Prompt, Resource, ResourceTemplate, Tool},
};
use std::{collections::HashSet, time::Duration};

pub(crate) async fn list_tools(peer: &Peer<RoleClient>) -> Result<Vec<Tool>> {
    collect("tools/list", |cursor| async move {
        let page = peer.list_tools(Some(PaginatedRequestParam { cursor })).await?;
        Ok((page.tools, page.next_cursor))
    })
    .await
}
pub(crate) async fn list_resources(peer: &Peer<RoleClient>) -> Result<Vec<Resource>> {
    collect("resources/list", |cursor| async move {
        let page = peer.list_resources(Some(PaginatedRequestParam { cursor })).await?;
        Ok((page.resources, page.next_cursor))
    })
    .await
}
pub(crate) async fn list_templates(peer: &Peer<RoleClient>) -> Result<Vec<ResourceTemplate>> {
    collect("resources/templates/list", |cursor| async move {
        let page = peer.list_resource_templates(Some(PaginatedRequestParam { cursor })).await?;
        Ok((page.resource_templates, page.next_cursor))
    })
    .await
}
pub(crate) async fn list_prompts(peer: &Peer<RoleClient>) -> Result<Vec<Prompt>> {
    collect("prompts/list", |cursor| async move {
        let page = peer.list_prompts(Some(PaginatedRequestParam { cursor })).await?;
        Ok((page.prompts, page.next_cursor))
    })
    .await
}
async fn collect<T, F, Fut>(method: &str, mut fetch: F) -> Result<Vec<T>>
where
    F: FnMut(Option<String>) -> Fut,
    Fut: std::future::Future<Output = Result<(Vec<T>, Option<String>)>>,
{
    tokio::time::timeout(Duration::from_secs(30), async {
        let mut tools = Vec::new();
        let mut cursor = None;
        let mut seen = HashSet::new();
        for _ in 0..100 {
            let (items, next_cursor) = fetch(cursor).await?;
            if items.len() > 2048_usize.saturating_sub(tools.len()) {
                bail!("{method} exceeded the catalog limit of 2048 items");
            }
            tools.extend(items);
            let Some(next) = next_cursor else {
                return Ok(tools);
            };
            if next.len() > 64 * 1024 {
                bail!("{method} pagination cursor exceeds 65536 bytes");
            }
            if !seen.insert(next.clone()) {
                bail!("{method} returned a repeated pagination cursor");
            }
            cursor = Some(next);
        }
        bail!("{method} exceeded the pagination limit of 100 pages")
    })
    .await
    .map_err(|_| anyhow::anyhow!("{method} pagination timed out"))?
}
